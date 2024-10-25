use serde::{Serialize, Deserialize};

use crate::serde_utils::SerdeRwLock as RwLock;
use super::objects::{UserId, AssociatedImage};

use std::sync::Arc;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

pub type EntityTag = String;
pub type Revision = u32;
pub type IndexInEntity = u64;

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)] 
pub struct Entities<T: Debug> {
    inner: RwLock<Vec<Arc<RwLock<Entity<T>>>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Entity<T: Debug> {
    pub inner: T,
    pub metadata: EntityData,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntityData {
    pub image: AssociatedImage,
    pub author: UserId,
    pub guests: Vec<UserId>,
    pub revision: Revision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityAccess {
    pub read_only: bool,
    pub local_name: String,
    pub tags: Vec<EntityTag>,
    pub last_seen_rev: Revision,
}

impl<T: Debug> Deref for Entity<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: Debug> DerefMut for Entity<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: Debug + Default> Entities<T> {
    pub const fn init() -> Self {
        Self {
            inner: RwLock::new(Vec::new()),
        }
    }

    pub async fn find(&self, raw_id: u32) -> Option<Arc<RwLock<Entity<T>>>> {
        let reader = self.inner.read().await;
        reader.get(raw_id as usize).cloned()
    }

    pub async fn metadata(&self, raw_id: u32) -> Option<EntityData> {
        let arc_user = self.find(raw_id).await?;
        let user = arc_user.read().await;
        Some(user.metadata.clone())
    }

    pub async fn new_entity(&self, metadata: EntityData) -> u32 {
        let entity = Entity {
            inner: T::default(),
            metadata,
        };

        let arc_entity = Arc::new(RwLock::new(entity));
        let mut writer = self.inner.write().await;
        let raw_id = writer.len() as u32;
        writer.push(arc_entity);

        raw_id
    }

    pub(super) async fn try_push_guest(&self, raw_id: u32, guest: UserId) {
        let Some(arc_entity) = self.find(raw_id).await else {
            println!("couldn't push guest!!");
            return;
        };

        let mut entity = arc_entity.write().await;
        entity.metadata.guests.push(guest);
    }

    pub(super) async fn try_drop_access(&self, raw_id: u32, user_id: UserId) {
        let Some(arc_entity) = self.find(raw_id).await else {
            println!("couldn't remove user access!!");
            return;
        };

        let mut entity = arc_entity.write().await;
        if entity.metadata.author == user_id {
            if entity.metadata.guests.is_empty() {
                entity.inner = Default::default();
                entity.metadata.author = UserId::MAX;
            } else {
                // make oldest guest the owner (author)
                let new_author = entity.metadata.guests.swap_remove(0);
                entity.metadata.author = new_author;
            }
        } else {
            let guests = &mut entity.metadata.guests;
            if let Some(i) = guests.iter().position(|g| *g == user_id) {
                guests.remove(i);
            } else {
                println!("No such guest");
            }
        }
    }

    pub(super) async fn restore(&self, reference: Self) {
        let mut src = reference.inner.write().await;
        let mut dst = self.inner.write().await;
        *dst = std::mem::take(&mut src);
    }
}
