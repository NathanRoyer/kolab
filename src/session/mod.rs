use futures_lite::future::{pending, race};
use litemap::LiteMap;

use crate::{
    serde_utils::SerdeRwLock as RwLock,
    database::{
        EntityId,
        update::Update,
        objects::{User, UserId},
        entities::{Entity, EntityData},
    }
};

use crate::{
    DATABASE, FULL_DB_ACCESS, WebSocket, SinkExt, Message as WsMessage,
    StreamExt, Receiver, StringifyError,
};

use requests::{Request, RequestData};
use replies::{Reply, ReplyData};
use upload::TemporaryFile;

use std::net::SocketAddr;
use std::fmt::Debug;
use std::sync::Arc;

type ErrMsg = &'static str;
type EntitiesDataMap = LiteMap<EntityId, EntityData>;

mod upload;
mod account;
mod objects;
mod replies;
mod entities;
mod requests;

pub type SessionId = usize;

pub struct Session {
    session_id: SessionId,
    _peer_addr: SocketAddr,
    socket: WebSocket,
    rx_update: Option<Receiver<Arc<Update>>>,
    tmp_file: Option<TemporaryFile>,
    user_id: Option<UserId>,
}

fn get_session_id() -> usize {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT_SESSION_ID: AtomicUsize = AtomicUsize::new(0);
    NEXT_SESSION_ID.fetch_add(1, Ordering::SeqCst)
}

impl Session {
    pub async fn run(peer_addr: SocketAddr, socket: WebSocket) {
        let mut this = Self {
            session_id: get_session_id(),
            _peer_addr: peer_addr,
            socket,
            rx_update: None,
            tmp_file: None,
            user_id: None,
        };

        this.actually_run().await;
    }

    pub async fn actually_run(&mut self) {
        loop {
            let rx_update = self.rx_update.clone();
            let update_recv = async {
                match rx_update {
                    Some(rx_update) => Select::A(rx_update.recv().await),
                    None => pending().await,
                }
            };

            let msg_recv = async { Select::B(self.socket.next().await) };
            let race_result = race(update_recv, msg_recv).await;

            let _reader = FULL_DB_ACCESS.read();
            let status = match race_result {
                Select::A(Ok(update)) => self.handle_update(update).await,
                Select::B(Some(Ok(WsMessage::Text(text)))) => self.handle_message(text).await,
                Select::B(Some(Ok(WsMessage::Binary(bytes)))) => self.handle_bytes(bytes).await,
                Select::B(Some(Ok(WsMessage::Ping(bytes)))) => self.handle_ping(bytes).await,
                other => Err(format!("Select() Error: {:?}", other)),
            };

            if let Err(msg) = status {
                println!("Session error: {}", msg);
                break;
            }
        }

        if let Some(user_id) = self.user_id {
            if let Some(arc_user) = DATABASE.users.find(user_id).await {
                let mut user = arc_user.write().await;
                user.end_of_session(self.session_id);
            }
        }
    }

    async fn send(&mut self, text: String) -> Result<(), String> {
        self.socket.send(WsMessage::Text(text)).await.fmt_err("send")
    }

    async fn handle_message(&mut self, text: String) -> Result<(), String> {
        let request: Request = serde_json::from_str(&text).fmt_err("serde")?;
        println!("REQUEST {:?}", request);
        let num = request.num;

        let reply = match self.handle_request(request).await {
            Ok(reply) => reply,
            Err(msg) => Reply {
                num,
                data: ReplyData::GenericFailure(msg.to_string()),
            },
        };

        // println!("REPLY {:?}", reply);
        let text = serde_json::to_string(&reply).fmt_err("to_string")?;
        self.send(text).await
    }

    async fn handle_update(&mut self, update: Arc<Update>) -> Result<(), String> {
        let json = serde_json::to_string(&update).fmt_err("handle_update")?;
        let _ = self.socket.send(WsMessage::Text(json)).await;

        Ok(())
    }

    async fn handle_bytes(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        if self.tmp_file.is_none() {
            let tmp_file = TemporaryFile::new().await?;
            self.tmp_file = Some(tmp_file);
        }

        let tmp_file = self.tmp_file.as_mut().unwrap();
        tmp_file.extend_from_slice(&bytes).await;

        Ok(())
    }

    async fn handle_ping(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        self.socket.send(WsMessage::Pong(bytes)).await.fmt_err("ping")
    }

    async fn handle_request(&mut self, request: Request) -> Result<Reply, ErrMsg> {
        use RequestData::*;
        let n = request.num;
        match request.data {
            // account related
            SendChallenge(a, b) => self.handle_send_challenge(n, a, b).await,
            CompleteChallenge(a) => self.handle_complete_challenge(n, a).await,
            CreateAccount(a, b) => self.handle_create_account(n, a, b).await,
            GetToken(a, b) => self.handle_get_token(n, a, b).await,
            OpenSession(a, b) => self.handle_open_session(n, a, b).await,
            LoadUserData(a) => self.handle_load_user_data(n, a).await,
            SetUserData(a, b) => self.handle_set_user_data(n, a, b).await,
            OpenInvite(a, b, c) => self.handle_open_invite(n, a, b, c).await,
            WhoIs(a) => self.handle_who_is(n, a).await,
            CreateEntity(a, b) => self.handle_create_entity(n, a, b).await,
            ServerShutdown => self.handle_server_shutdown(n).await,

            // generic entity actions
            LoadHistory(a) => self.handle_load_history(n, a).await,
            SetEntityTags(a, b) => self.handle_set_entity_tags(n, a, b).await,
            RenameEntity(a, b) => self.handle_rename_entity(n, a, b).await,
            CreateInvite(a, b, c) => self.handle_create_invite(n, a, b, c).await,
            TransferOwnership(a, b) => self.handle_transfer_ownership(n, a, b).await,
            BanGuest(a, b) => self.handle_ban_guest(n, a, b).await,
            Drop(a) => self.handle_drop(n, a).await,

            // conversations
            LoadMessagesBefore(a, b) => self.handle_load_messages_before(n, a, b).await,
            PostMessage(a, b, c) => self.handle_post_message(n, a, b, c).await,
            EditMessage(a, b, c, d) => self.handle_edit_message(n, a, b, c, d).await,

            // spreadsheets
            LoadSpreadsheet(a) => self.handle_load_spreadsheet(n, a).await,
            SetCell(a, b, c, d) => self.handle_set_cell(n, a, b, c, d).await,

            // documents
            LoadDocument(a) => self.handle_load_document(n, a).await,
            InsertElement(a, b, c, d) => self.handle_insert_element(n, a, b, c, d).await,
            DeleteElement(a, b, c) => self.handle_delete_element(n, a, b, c).await,
            SetElement(a, b, c, d) => self.handle_set_element(n, a, b, c, d).await,

            // buckets
            LoadBucket(a) => self.handle_load_bucket(n, a).await,
            DeleteFile(a, b, c) => self.handle_delete_file(n, a, b, c).await,
            // SetFile(a, b, c, d) => self.handle_set_file(n, a, b, c, d).await,
            FinishFile(a, b, c) => self.handle_finish_file(n, a, b, c).await,
        }
    }

    pub async fn get_user(&self) -> Result<(Arc<RwLock<Entity<User>>>, UserId), ErrMsg> {
        let user_id = self.user_id.ok_or("Not logged in yet")?;
        let arc_user = DATABASE.users.find(user_id).await.ok_or("No such user")?;
        Ok((arc_user, user_id))
    }
}

impl RwLock<Entity<User>> {
    pub async fn check_access_to(&self, entity: EntityId, read_write: bool) -> Result<(), ErrMsg> {
        let user = self.read().await;
        let maybe_access = user.secret.entities.get(&entity);
        let access = maybe_access.ok_or("No such entity")?;
        match access.read_only && read_write {
            true => Err("Read-only access"),
            false => Ok(()),
        }
    }
}

#[derive(Debug)]
enum Select<A: Debug, B: Debug> {
    A(A),
    B(B),
}
