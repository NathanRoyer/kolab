use serde::{Serialize, Deserialize};
use litemap::LiteMap;

use crate::serde_utils::SerdeRwLock as RwLock;

use objects::{Conversation, Bucket, Sheet, Document, User, Username, Hash};
use objects::{UserId, ConvId, DocumentId, BucketId, SheetId};
use entities::{Entities, EntityData};
use update::Update;

use std::sync::Arc;
use std::fmt::Debug;
use std::iter::once;

pub mod update;
pub mod objects;
pub mod entities;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntityId {
    Conversation(ConvId),
    Document(DocumentId),
    Bucket(BucketId),
    Spreadsheet(SheetId),
    User(UserId),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InviteData {
    pub orig_name: String,
    pub sender: UserId,
    pub target: EntityId,
    pub read_only: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    pub conversations: Entities<Conversation>,
    pub buckets: Entities<Bucket>,
    pub sheets: Entities<Sheet>,
    pub documents: Entities<Document>,
    pub users: Entities<User>,
    pub usernames: RwLock<LiteMap<Username, UserId>>,
    pub file_rc: RwLock<LiteMap<Hash, usize>>,
}

impl Database {
    pub const fn init() -> Self {
        Self {
            conversations: Entities::init(),
            buckets: Entities::init(),
            sheets: Entities::init(),
            documents: Entities::init(),
            users: Entities::init(),
            usernames: RwLock::new(LiteMap::new()),
            file_rc: RwLock::new(LiteMap::new()),
        }
    }

    pub async fn notify_users(&self, update: Update) {
        let update = Arc::new(update);

        let maybe_metadata = match update.id {
            EntityId::Conversation(id) => self.conversations.metadata(id).await,
            EntityId::Bucket(id) => self.buckets.metadata(id).await,
            EntityId::Spreadsheet(id) => self.sheets.metadata(id).await,
            EntityId::Document(id) => self.documents.metadata(id).await,
            EntityId::User(id) => self.users.metadata(id).await,
        };

        let Some(metadata) = maybe_metadata else {
            println!("notify_users: no such entity");
            return;
        };

        let user_iter = metadata.guests.into_iter();
        for user_id in user_iter.chain(once(metadata.author)) {
            let Some(arc_user) = self.users.find(user_id).await else {
                println!("notify_users: invalid user_id in entity guests");
                continue;
            };

            let user = arc_user.read().await;
            let sessions = user.sessions.clone();
            core::mem::drop(user);

            for tx_update in sessions.iter_values() {
                println!("notifying one user session");
                let _ = tx_update.send(update.clone()).await;
            }
        }
    }

    pub async fn push_guest(&self, entity_id: EntityId, guest: UserId) {
        match entity_id {
            EntityId::Conversation(id) => self.conversations.try_push_guest(id, guest).await,
            EntityId::Bucket(id) => self.buckets.try_push_guest(id, guest).await,
            EntityId::Spreadsheet(id) => self.sheets.try_push_guest(id, guest).await,
            EntityId::Document(id) => self.documents.try_push_guest(id, guest).await,
            EntityId::User(id) => self.users.try_push_guest(id, guest).await,
        }
    }

    pub async fn drop_access(&self, entity_id: EntityId, guest: UserId) {
        match entity_id {
            EntityId::Conversation(id) => self.conversations.try_drop_access(id, guest).await,
            EntityId::Bucket(id) => self.buckets.try_drop_access(id, guest).await,
            EntityId::Spreadsheet(id) => self.sheets.try_drop_access(id, guest).await,
            EntityId::Document(id) => self.documents.try_drop_access(id, guest).await,
            EntityId::User(id) => self.users.try_drop_access(id, guest).await,
        }
    }

    pub async fn metadata(&self, entity_id: EntityId) -> Option<EntityData> {
        match entity_id {
            EntityId::Conversation(id) => self.conversations.metadata(id).await,
            EntityId::Bucket(id) => self.buckets.metadata(id).await,
            EntityId::Spreadsheet(id) => self.sheets.metadata(id).await,
            EntityId::Document(id) => self.documents.metadata(id).await,
            EntityId::User(id) => self.users.metadata(id).await,
        }
    }

    pub async fn load_from_json(&self, saved_db: &str) {
        let reference: Self = serde_json::from_str(&saved_db).unwrap();
        self.conversations.restore(reference.conversations).await;
        self.buckets.restore(reference.buckets).await;
        self.sheets.restore(reference.sheets).await;
        self.documents.restore(reference.documents).await;
        self.users.restore(reference.users).await;

        let mut src = reference.usernames.write().await;
        let mut dst = self.usernames.write().await;
        *dst = std::mem::take(&mut src);
    }

    pub async fn inc_file_rc(&self, hash: &Hash) {
        let mut file_rc = self.file_rc.write().await;
        if let Some(counter) = file_rc.get_mut(hash) {
            *counter += 1;
        } else {
            file_rc.insert(hash.clone(), 1);
        }
    }

    pub async fn dec_file_rc(&self, hash: &Hash) {
        let mut file_rc = self.file_rc.write().await;
        if let Some(counter) = file_rc.get_mut(hash) {
            *counter -= 1;
        }
    }
}
