use serde::{Serialize, Deserialize};
use async_channel::Sender;
use litemap::LiteMap;

use crate::session::SessionId;
use super::{EntityId, InviteData};
use super::entities::{EntityAccess, IndexInEntity};
use super::update::Update;

use std::sync::Arc;

pub type Username = String;
pub type CellTag = String;
pub type Token = String;
pub type Email = String;
pub type Hash = String;

pub type UserId = u32;
pub type ConvId = u32;
pub type BucketId = u32;
pub type DocumentId = u32;
pub type SheetId = u32;

pub type Stamp = u64;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Conversation {
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// UserId::MAX if server-sent
    pub author: UserId,
    pub content: String,
    pub created: Stamp,
    #[serde(default)]
    pub extended: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MessageExtension {
    pub content: String,
    pub reactions: LiteMap<char, Vec<UserId>>,
    pub edited: Option<Stamp>,
    pub replying_to: Option<IndexInEntity>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Bucket {
    pub files: Vec<File>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub name: String,
    pub sha256: Hash,
    pub size: usize,
    pub uploaded: Stamp,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Sheet {
    pub cells: LiteMap<IndexInEntity, Cell>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cell {
    text: String,
    formula: CellFormula,
    tags: Vec<CellTag>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Document {
    pub elements: Vec<Element>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Element {
    data: String,
    style: ElementStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ElementStyle {
    Title,
    Part,
    Chapter,
    Section,
    Subsection,
    Image,
    Paragraph,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct User {
    pub public: UserData,
    pub secret: SecretUserData,
    // internal user data:
    pub tokens: Vec<Token>,
    #[serde(skip)]
    pub sessions: LiteMap<SessionId, Sender<Arc<Update>>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserData {
    pub name: Username,
    pub email: Email,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
#[serde(rename_all = "kebab-case")]
pub enum AssociatedImage {
    Picture(String),
    Gradient([String; 2]),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecretUserData {
    pub invites: Vec<InviteData>,
    pub entities: LiteMap<EntityId, EntityAccess>,
    pub password_hash: String,
    #[serde(default)]
    pub password_salt: String,
    pub server_admin: bool,
    pub max_file_size: usize,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum CellFormula {
    Literal,
    TagSum,
    TagMean,
    TagMode,
    TagCount,
    TagRange,
    TagMedian,
    TagMinimum,
    TagMaximum,
    TagProduct,
    CellsSum,
    CellsRatio,
    CellsProduct,
    CellsRemainder,
    CellsDifference,
    CellSqrt,
}

impl User {
    pub fn set_tx_update(&mut self, session_id: usize, tx_update: Sender<Arc<Update>>) {
        self.sessions.insert(session_id, tx_update);
    }

    pub fn end_of_session(&mut self, session_id: usize) {
        self.sessions.remove(&session_id);
    }
}

impl AssociatedImage {
    pub fn random_gradient() -> Self {
        let [a, b, c]: [u8; 3] = rand::random();
        let [d, e, f]: [u8; 3] = rand::random();

        let c1 = format!("#{:02x}{:02x}{:02x}", a, b, c);
        let c2 = format!("#{:02x}{:02x}{:02x}", d, e, f);

        Self::Gradient([c1, c2])
    }
}
