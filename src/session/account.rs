use litemap::LiteMap;

use crate::{
    database::{
        EntityId,
        update::{Update, UpdateType},
        objects::{Token, UserData, Email, Username, UserId, AssociatedImage},
        entities::{Revision, EntityAccess, EntityData},
    }
};

use crate::{DATABASE, crypto_hash, to_hex, trigger_backup};
use super::requests::{ChallengeTarget, Code, Invite};
use super::replies::{Reply, ReplyData};
use super::{Session, ErrMsg};

use std::sync::Arc;
use std::mem::drop;
use std::iter::once;

#[allow(unused_variables)]
impl Session {
    pub(super) async fn handle_send_challenge(
        &mut self,
        num: usize,
        a: Email,
        b: ChallengeTarget,
    ) -> Result<Reply, ErrMsg> {
        Err("unimplemented")
    }

    pub(super) async fn handle_complete_challenge(
        &mut self,
        num: usize,
        a: Code,
    ) -> Result<Reply, ErrMsg> {
        Err("unimplemented")
    }

    pub(super) async fn handle_create_account(
        &mut self,
        num: usize,
        name: Username,
        password: String,
    ) -> Result<Reply, ErrMsg> {
        let password_hash = crypto_hash(password);

        if true {
            let reader = DATABASE.usernames.read().await;
            if reader.contains_key(&name) {
                return Err("Username already taken");
            }
        }

        let metadata = EntityData {
            image: AssociatedImage::random_gradient(),
            author: 0, // set after insertion
            guests: Vec::new(),
        };

        let user_id = DATABASE.users.new_entity(metadata).await;

        if true {
            let mut writer = DATABASE.usernames.write().await;
            let _ = writer.insert(name.clone(), user_id);
        }

        let arc_user = DATABASE.users.find(user_id).await.unwrap();
        let mut user = arc_user.write().await;

        user.secret.server_admin = user_id == 0;
        user.secret.password_hash = password_hash;
        user.secret.max_file_size = 50 * 1024 * 1024; // 50 MiB
        user.metadata.author = user_id;
        user.public = UserData {
            name,
            email: Email::default(),
            status: "Exploring".to_string(),
        };

        Ok(Reply::new(num, ReplyData::ValidUsername(user_id)))
    }

    pub(super) async fn handle_get_token(
        &mut self,
        num: usize,
        user_id: UserId,
        password: String,
    ) -> Result<Reply, ErrMsg> {
        let password_hash = crypto_hash(password);

        let arc_user = DATABASE.users.find(user_id).await.ok_or("No such user")?;
        let mut user = arc_user.write().await;

        if user.secret.password_hash != password_hash {
            return Err("Wrong password");
        }

        let token = to_hex(rand::random());
        user.tokens.push(token.clone());

        Ok(Reply::new(num, ReplyData::AuthenticationToken(token)))
    }

    pub(super) async fn handle_open_session(
        &mut self,
        num: usize,
        user_id: UserId,
        token: Token,
    ) -> Result<Reply, ErrMsg> {
        if self.user_id.is_some() {
            return Err("Session already opened");
        }

        let arc_user = DATABASE.users.find(user_id).await.ok_or("No such user")?;
        let mut user = arc_user.write().await;

        if user.tokens.contains(&token) {
            let (tx_update, rx_update) = async_channel::unbounded();

            let id = self.session_id;
            user.set_tx_update(id, tx_update);

            self.rx_update = Some(rx_update);
            self.user_id = Some(user_id);

            Ok(Reply::new(num, ReplyData::GenericSuccess))
        } else {
            Err("Wrong Token")
        }
    }

    pub(super) async fn handle_load_user_data(
        &mut self,
        num: usize,
        maybe_user_id: Option<UserId>,
    ) -> Result<Reply, ErrMsg> {
        let is_self = maybe_user_id.is_none();
        let maybe_user_id = maybe_user_id.or(self.user_id);
        let user_id = maybe_user_id.ok_or("Not logged in yet")?;
        let arc_user = DATABASE.users.find(user_id).await.ok_or("No such user")?;

        let user = arc_user.read().await;
        let revision = user.revision;
        let image = user.metadata.image.clone();
        let public = user.public.clone();
        let secret = user.secret.clone();
        drop(user);

        let reply_data = if is_self {
            let num_ent = secret.entities.len();
            let mut entity_map = LiteMap::with_capacity(num_ent + 1);

            let this_user = EntityId::User(user_id);
            let entity_ids = secret.entities.iter_keys();

            for id in entity_ids.chain(once(&this_user)) {
                let data = DATABASE.metadata(*id).await;
                let data = data.ok_or("Internal DB inconsistency")?;
                entity_map.insert(*id, data);
            }

            ReplyData::SelfData(revision, public, entity_map, secret)
        } else {
            ReplyData::UserData(revision, public, image)
        };

        Ok(Reply::new(num, reply_data))
    }

    pub(super) async fn handle_set_user_data(
        &mut self,
        num: usize,
        rev: Revision,
        data: UserData,
    ) -> Result<Reply, ErrMsg> {
        let (arc_user, user_id) = self.get_user().await?;

        let mut user = arc_user.write().await;

        if user.revision != rev {
            return Err("Out of date");
        }

        user.revision += 1;
        let update = Update::user(user_id, user.revision, &data);
        user.public = data;

        drop(user);

        DATABASE.notify_users(update).await;
        Ok(Reply::new(num, ReplyData::GenericSuccess))
    }

    pub(super) async fn handle_open_invite(
        &mut self,
        num: usize,
        revision: Revision,
        invite: Invite,
        discard: bool,
    ) -> Result<Reply, ErrMsg> {
        let (arc_user, user_id) = self.get_user().await?;

        let data = {
            let user = arc_user.read().await;
            let data = user.secret.invites.get(invite);
            data.cloned().ok_or("Invalid invite")?
        };

        let access = EntityAccess {
            read_only: data.read_only,
            local_name: data.orig_name,
            tags: Vec::new(),
        };

        if true {
            let mut user = arc_user.write().await;
            if user.revision != revision {
                return Err("bad revision");
            }

            if !discard {
                // todo: send update to guests
                user.secret.entities.insert(data.target, access);
            }

            user.revision += 1;
            user.secret.invites.remove(invite);
        };

        // friendship! make it reciprocal
        let mut maybe_update = None;
        if let (EntityId::User(friend_id), false) = (data.target, discard) {
            let our_id = EntityId::User(user_id);
            let arc_friend = DATABASE.users.find(friend_id).await.ok_or("friend gone")?;
            let access = EntityAccess {
                read_only: true,
                local_name: "Friend Request".into(),
                tags: Vec::new(),
            };

            let mut friend = arc_friend.write().await;
            friend.revision += 1;
            friend.secret.entities.insert(our_id, access);
            let sessions = friend.sessions.clone();
            drop(friend);

            DATABASE.push_guest(our_id, friend_id).await;

            let update = Update::friend(friend_id, user_id);
            maybe_update = Some((Arc::new(update), sessions));
        } else if !discard {
            let update = Update::new(UpdateType::NewGuest, data.target, 0, 0, &user_id);
            DATABASE.notify_users(update).await;
        }

        DATABASE.push_guest(data.target, user_id).await;

        if let Some((update, sessions)) = maybe_update {
            for tx_update in sessions.iter_values() {
                println!("notifying one user session");
                let _ = tx_update.send(update.clone()).await;
            }
        }

        Ok(Reply::new(num, ReplyData::GenericSuccess))
    }

    pub(super) async fn handle_who_is(&mut self, num: usize, username: Username) -> Result<Reply, ErrMsg> {
        // self.user_id.ok_or("Not logged in yet")?;
        let reader = DATABASE.usernames.read().await;
        match reader.get(&username) {
            Some(user_id) => Ok(Reply::new(num, ReplyData::ValidUsername(*user_id))),
            None => Err("Invalid username"),
        }
    }

    pub(super) async fn handle_create_entity(
        &mut self,
        num: usize,
        entity_type: String,
        local_name: String,
    ) -> Result<Reply, ErrMsg> {
        let (arc_user, user_id) = self.get_user().await?;

        let metadata = EntityData {
            image: AssociatedImage::random_gradient(),
            author: user_id,
            guests: Vec::new(),
        };

        let db = &DATABASE;
        let entity_id = match &*entity_type {
            "conv" => EntityId::Conversation(db.conversations.new_entity(metadata).await),
            "doc" => EntityId::Document(db.documents.new_entity(metadata).await),
            "sheet" => EntityId::Spreadsheet(db.sheets.new_entity(metadata).await),
            "bucket" => EntityId::Bucket(db.buckets.new_entity(metadata).await),
            _ => return Err("Invalid entity type"),
        };

        let access = EntityAccess {
            read_only: false,
            local_name,
            tags: Vec::new(),
        };

        let mut user = arc_user.write().await;
        user.secret.entities.insert(entity_id, access);

        Ok(Reply::new(num, ReplyData::EntityCreated(entity_id)))
    }

    pub(super) async fn handle_server_shutdown(&mut self, num: usize) -> Result<Reply, ErrMsg> {
        let (arc_user, user_id) = self.get_user().await?;
        let user = arc_user.read().await;

        if user.secret.server_admin {
            trigger_backup().await;
            Ok(Reply::new(num, ReplyData::GenericSuccess))
        } else {
            Err("Not an admin!")
        }
    }
}