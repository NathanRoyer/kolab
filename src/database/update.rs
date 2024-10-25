use serde::Serialize;

use super::{EntityId, UserId};
use super::entities::{Revision, IndexInEntity};
use super::objects::{UserData, Cell, SheetId};

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateType {
    SetUser,
    NewInvite,
    NewGuest,
    ByeGuest,
    NewFriend,
    NewMessage,
    SetMessage,
    SetCell,
    SetElement,
    NewElement,
    ByeElement,
    NewFile,
    SetFile,
    ByeFile,
}

#[derive(Debug, Serialize)]
pub struct Update {
    #[serde(rename = "type")]
    pub update_type: UpdateType,
    pub id: EntityId,
    pub new_revision: Revision,
    pub index: IndexInEntity,
    pub data: serde_json::Value,
}

impl Update {
    pub fn new<S: Serialize>(
        update_type: UpdateType,
        id: EntityId,
        rev: Revision,
        index: IndexInEntity,
        data: &S,
    ) -> Self {
        Update {
            update_type,
            id,
            new_revision: rev,
            index,
            data: serde_json::to_value(data).unwrap_or_default(),
        }
    }

    pub fn user(user_id: UserId, rev: Revision, data: &UserData) -> Self {
        Self::new(UpdateType::SetUser, EntityId::User(user_id), rev, 0, data)
    }

    pub fn friend(sender_id: UserId, receiver_id: UserId) -> Self {
        let rcv = receiver_id as u64;
        Self::new(UpdateType::NewFriend, EntityId::User(sender_id), 0, rcv, &())
    }

    pub fn cell(sheet_id: SheetId, rev: Revision, index: IndexInEntity, data: &Cell) -> Self {
        Self::new(UpdateType::SetCell, EntityId::Spreadsheet(sheet_id), rev, index, data)
    }
}
