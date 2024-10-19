use crate::{
    database::{
        EntityId, InviteData,
        update::{Update, UpdateType},
        objects::{Username, UserId},
        entities::EntityTag,
    }
};

use crate::DATABASE;
use super::replies::{Reply, ReplyData};
use super::{Session, ErrMsg};

use std::sync::Arc;

#[allow(unused_variables)]
impl Session {
    pub(super) async fn handle_load_history(
        &mut self,
        num: usize,
        a: EntityId,
    ) -> Result<Reply, ErrMsg> {
        Err("unimplemented")
    }

    pub(super) async fn handle_set_entity_tags(
        &mut self,
        num: usize,
        entity_id: EntityId,
        tags: Vec<EntityTag>,
    ) -> Result<Reply, ErrMsg> {
        let (arc_user, _user_id) = self.get_user().await?;

        let mut user = arc_user.write().await;
        let access = user.secret.entities.get_mut(&entity_id).ok_or("No such entity")?;
        access.tags = tags;
        Ok(Reply::new(num, ReplyData::GenericSuccess))
    }

    pub(super) async fn handle_rename_entity(
        &mut self,
        num: usize,
        entity_id: EntityId,
        name: String,
    ) -> Result<Reply, ErrMsg> {
        let (arc_user, _user_id) = self.get_user().await?;

        let mut user = arc_user.write().await;
        let access = user.secret.entities.get_mut(&entity_id).ok_or("No such entity")?;
        access.local_name = name;
        Ok(Reply::new(num, ReplyData::GenericSuccess))
    }

    pub(super) async fn handle_create_invite(
        &mut self,
        num: usize,
        target: EntityId,
        read_only: bool,
        guests: Vec<UserId>,
    ) -> Result<Reply, ErrMsg> {
        let not_friend_request = !matches!(target, EntityId::User(_));
        let (arc_user, user_id) = self.get_user().await?;

        let metadata = DATABASE.metadata(target).await.ok_or("No such entity")?;
        if metadata.author != user_id {
            return Err("User is not the author of this entity");
        }

        let (orig_name, maybe_friend_check) = {
            let user = arc_user.read().await;
            let friends = user.metadata.guests.clone();
            let name = match user.secret.entities.get(&target) {
                Some(access) => access.local_name.clone(),
                None => "Friend Request".into(),
            };
            (name, not_friend_request.then_some(friends))
        };

        for guest_id in guests.iter() {
            if let Some(friends) = maybe_friend_check.as_ref() {
                if !friends.contains(guest_id) {
                    return Err("Not a friend (yet)");
                }
            }

            if metadata.guests.contains(guest_id) {
                return Err("Guest already has access");
            }

            let maybe_arc = DATABASE.users.find(*guest_id).await;
            let arc_guest = maybe_arc.ok_or("No such guest user id")?;
            let guest = arc_guest.read().await;
            if guest.secret.invites.iter().any(|data| data.target == target) {
                return Err("Guest already invited");
            }
        }

        let invite_data = InviteData {
            sender: user_id,
            orig_name,
            target,
            read_only,
        };

        for guest_id in guests {
            let arc_user = DATABASE.users.find(guest_id).await.unwrap();
            let mut user = arc_user.write().await;
            user.secret.invites.push(invite_data.clone());
            let sessions = user.sessions.clone();
            let data = serde_json::Value::Null;
            core::mem::drop(user);

            let data = serde_json::to_value(&invite_data);
            let data = data.unwrap_or_default();

            let update = Arc::new(Update {
                update_type: UpdateType::NewInvite,
                id: EntityId::User(guest_id),
                new_revision: 0,
                index: 0,
                data,
            });

            for tx_update in sessions.iter_values() {
                println!("notifying one user session");
                let _ = tx_update.send(update.clone()).await;
            }
        }

        Ok(Reply::new(num, ReplyData::GenericSuccess))
    }

    pub(super) async fn handle_transfer_ownership(
        &mut self,
        num: usize,
        a: EntityId,
        b: Username,
    ) -> Result<Reply, ErrMsg> {
        Err("unimplemented")
    }

    pub(super) async fn handle_ban_guest(
        &mut self,
        num: usize,
        a: EntityId,
        b: Username,
    ) -> Result<Reply, ErrMsg> {
        Err("unimplemented")
    }

    pub(super) async fn handle_drop(
        &mut self,
        num: usize,
        entity_id: EntityId,
    ) -> Result<Reply, ErrMsg> {
        let (arc_user, user_id) = self.get_user().await?;
        DATABASE.drop_access(entity_id, user_id).await;

        let update = Update::new(UpdateType::ByeGuest, entity_id, 0, 0, &user_id);
        DATABASE.notify_users(update).await;

        let mut user = arc_user.write().await;
        user.secret.entities.remove(&entity_id);
        // todo: get rid of relevant invites
        Ok(Reply::new(num, ReplyData::GenericSuccess))
    }
}