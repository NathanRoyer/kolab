use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::de::{Visitor, Error};
use async_lock::RwLock;

use super::database::EntityId;

use std::ops::{Deref, DerefMut};
use std::fmt::{self, Debug};

#[derive(Debug)]
pub struct SerdeRwLock<T: Debug> {
    inner: RwLock<T>,
}

impl<T: Debug> SerdeRwLock<T> {
    pub const fn new(inner: T) -> Self {
        Self {
            inner: RwLock::new(inner),
        }
    }
}

impl<T: Debug> Deref for SerdeRwLock<T> {
    type Target = RwLock<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: Debug> DerefMut for SerdeRwLock<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: Debug + Serialize> Serialize for SerdeRwLock<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let inner = self.inner.try_read().unwrap();
        inner.serialize(serializer)
    }
}

impl<'de, T: Debug + Deserialize<'de>> Deserialize<'de> for SerdeRwLock<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let inner = T::deserialize(deserializer)?;
        Ok(Self {
            inner: RwLock::new(inner),
        })
    }
}

impl Serialize for EntityId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let (variant, raw_id) = match self {
            EntityId::Conversation(raw_id) => ("conv", raw_id),
            EntityId::Document(raw_id) => ("document", raw_id),
            EntityId::Bucket(raw_id) => ("bucket", raw_id),
            EntityId::Spreadsheet(raw_id) => ("sheet", raw_id),
            EntityId::User(raw_id) => ("user", raw_id),
        };

        let id_str = format!("{variant}-{raw_id}");
        serializer.serialize_str(&id_str)
    }
}

struct EntityIdVisitor;

impl<'de> Visitor<'de> for EntityIdVisitor {
    type Value = EntityId;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an entity id, for instant 'conv-53'")
    }

    fn visit_str<E>(self, id_str: &str) -> Result<Self::Value, E> where E: Error {
        let Some((variant, raw_id_str)) = id_str.split_once('-') else {
            return Err(E::custom(format!("Invalid Entity ID: {}", id_str)));
        };

        let Ok(raw_id) = raw_id_str.parse() else {
            return Err(E::custom(format!("Invalid Entity raw ID: {}", raw_id_str)));
        };

        match variant {
            "conv" => Ok(EntityId::Conversation(raw_id)),
            "document" => Ok(EntityId::Document(raw_id)),
            "bucket" => Ok(EntityId::Bucket(raw_id)),
            "sheet" => Ok(EntityId::Spreadsheet(raw_id)),
            "user" => Ok(EntityId::User(raw_id)),
            other => Err(E::custom(format!("Invalid Entity ID variant: {}", other))),
        }
    }
}

impl<'de> Deserialize<'de> for EntityId {
    fn deserialize<D>(deserializer: D) -> Result<EntityId, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_str(EntityIdVisitor)
    }
}

