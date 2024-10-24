use serde::Serialize;

use crate::database::{
    EntityId,
    entities::{Revision, IndexInEntity},
    objects::{
        Message, Token, Cell, UserData, Element, AssociatedImage,
        File, UserId, SecretUserData,
    },
};

use super::EntitiesDataMap;

#[derive(Debug, Clone, Serialize)]
pub struct Reply {
    pub num: usize,
    #[serde(flatten)]
    pub data: ReplyData,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "reply", content = "parameters")]
#[serde(rename_all = "kebab-case")]
pub enum ReplyData {
    AuthenticationToken(Token),
    ValidUsername(UserId),
    UserData(Revision, UserData, AssociatedImage),
    SelfData(Revision, UserData, EntitiesDataMap, SecretUserData),
    EntityCreated(EntityId),
    // History(Vec<(Revision, Change)>),
    Messages(Revision, IndexInEntity, Vec<Message>),
    Spreadsheet(Revision, Vec<(IndexInEntity, Cell)>),
    Document(Revision, Vec<Element>),
    Bucket(Revision, Vec<File>),
    GenericSuccess,
    GenericFailure(String),
}

impl Reply {
    pub fn new(num: usize, data: ReplyData) -> Self {
        Self {
            num,
            data,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Change;