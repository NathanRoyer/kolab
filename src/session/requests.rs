use serde::Deserialize;

use crate::database::{
    EntityId,
    entities::{Revision, EntityTag, IndexInEntity},
    objects::{
        Token, Cell, UserData, Email, Username, Element,
        UserId, ConversationId, SheetId, DocumentId, BucketId,
    },
};

pub type ReadOnly = bool;
pub type Discard = bool;
pub type EntityType = String;
pub type Code = String;
pub type Invite = usize;

#[derive(Debug, Clone, Deserialize)]
pub struct Request {
    pub num: usize,
    #[serde(flatten)]
    pub data: RequestData,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "request", content = "parameters")]
#[serde(rename_all = "kebab-case")]
pub enum RequestData {
    // account related
    SendChallenge(Email, ChallengeTarget),
    CompleteChallenge(Code),
    CreateAccount(Username, String),
    GetToken(UserId, String),
    OpenSession(UserId, Token),
    LoadUserData(Option<UserId>),
    SetUserData(Revision, UserData),
    OpenInvite(Revision, Invite, Discard),
    WhoIs(Username),
    CreateEntity(EntityType, String),
    ServerShutdown,

    // generic entity actions
    LoadHistory(EntityId),
    SetEntityTags(EntityId, Vec<EntityTag>),
    RenameEntity(EntityId, String),
    CreateInvite(EntityId, ReadOnly, Vec<UserId>),
    TransferOwnership(EntityId, Username),
    BanGuest(EntityId, Username),
    Drop(EntityId),

    // conversations
    LoadMessagesBefore(ConversationId, MessageCursor),
    PostMessage(ConversationId, Revision, String),
    EditMessage(ConversationId, Revision, IndexInEntity, String),

    // spreadsheets
    LoadSpreadsheet(SheetId),
    SetCell(SheetId, Revision, IndexInEntity, Cell),

    // documents
    LoadDocument(DocumentId),
    InsertElement(DocumentId, Revision, IndexInEntity, Element),
    DeleteElement(DocumentId, Revision, IndexInEntity),
    SetElement(DocumentId, Revision, IndexInEntity, Element),

    // buckets
    LoadBucket(BucketId),
    DeleteFile(BucketId, Revision, IndexInEntity),
    // SetFile(BucketId, Revision, Option<IndexInEntity>, File),
    FinishFile(BucketId, Revision, String),
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub enum ChallengeTarget {
    AccountCreation,
    Login,
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(tag = "cursor", content = "index")]
#[serde(rename_all = "kebab-case")]
pub enum MessageCursor {
    Specific(IndexInEntity),
    Latest,
}
