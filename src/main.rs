use async_tungstenite::tungstenite::protocol::Message;
use async_channel::{Sender, Receiver, unbounded};
use futures_lite::future::{block_on, or};
use async_fs::{read, read_to_string};
use async_tungstenite::accept_async;
use futures_util::sink::SinkExt;
use futures_lite::{StreamExt};
use sha2::{Digest, Sha256};
use async_net::TcpStream;
use async_lock::RwLock;

type WebSocket = async_tungstenite::WebSocketStream<TcpStream>;

mod http;
mod backup;
mod session;
mod database;
mod executor;
mod serde_utils;

use session::Session;
use database::Database;
use executor::{spawn_runner, runner};

static DATABASE: Database = Database::init();
static FULL_DB_ACCESS: RwLock<()> = RwLock::new(());
static TX_BACKUP_SIGNAL: RwLock<Option<Sender<()>>> = RwLock::new(None);

static INDEX_HTML: RwLock<Vec<u8>> = RwLock::new(Vec::new());
static STYLE_CSS: RwLock<Vec<u8>> = RwLock::new(Vec::new());
static MAIN_JS: RwLock<Vec<u8>> = RwLock::new(Vec::new());

async fn trigger_backup() {
    let reader = TX_BACKUP_SIGNAL.read().await;
    let _ = reader.as_ref().unwrap().send(()).await;
}

fn crypto_hash(salt: Option<[u8; 32]>, mut string: String) -> String {
    let salt = salt.as_ref().map(|s| s.as_slice()).unwrap_or(b"");
    let mut hasher = Sha256::new();

    hasher.update(salt);
    hasher.update(&string);

    let hash = to_hex(hasher.finalize().into());

    // erase from ram
    // the whitespace will move to the left, erasing every char
    string.push(' ');
    while string.len() > 0 {
        string.remove(0);
    }

    hash
}

async fn session(stream: TcpStream) {
    let Ok(address) = stream.peer_addr() else {
        println!("Failed to read peer address");
        return;
    };

    let Ok(ws_stream) = accept_async(stream).await else {
        println!("Failed to negociate WS session");
        return;
    };

    Session::run(address, ws_stream).await;
    println!("End Of Session");
}

async fn init_resource(staticc: &RwLock<Vec<u8>>, path: &str) {
    let mut dst = staticc.write().await;
    let src = read(path).await.unwrap();
    dst.extend_from_slice(&src);
    dst.extend_from_slice(b"\r\n");
}

pub fn main() {
    let backup_task = {
        let (tx_signal, rx_signal) = unbounded();

        block_on(async move {
            let mut writer = TX_BACKUP_SIGNAL.write().await;
            let _ = writer.insert(tx_signal);
        });

        backup::backup_task(rx_signal)
    };

    let init_resources = async move {
        init_resource(&INDEX_HTML, "front/index.html").await;
        init_resource(&STYLE_CSS, "front/style.css").await;

        // gather all js
        init_resource(&MAIN_JS, "front/js/common.js").await;
        init_resource(&MAIN_JS, "front/js/socket.js").await;
        init_resource(&MAIN_JS, "front/js/entities.js").await;
        init_resource(&MAIN_JS, "front/js/user.js").await;
        init_resource(&MAIN_JS, "front/js/conv.js").await;
        init_resource(&MAIN_JS, "front/js/doc.js").await;
        init_resource(&MAIN_JS, "front/js/bucket.js").await;
        init_resource(&MAIN_JS, "front/js/context-menu.js").await;
        init_resource(&MAIN_JS, "front/js/emojis.js").await;
        init_resource(&MAIN_JS, "front/js/init.js").await;

        let db_json = read_to_string("database.json").await.unwrap();
        DATABASE.load_from_json(&db_json).await;
    };

    block_on(init_resources);

    let (tx_tasks, rx_tasks) = unbounded();
    let http_task = http::listen(tx_tasks.clone());

    tx_tasks.try_send(http_task.into()).unwrap();
    tx_tasks.try_send(backup_task.into()).unwrap();

    // spawn 3 threads to have 4 threads total
    spawn_runner(&rx_tasks);
    spawn_runner(&rx_tasks);
    spawn_runner(&rx_tasks);

    runner(rx_tasks);
}

trait StringifyError<T> {
    fn fmt_err(self, context: &str) -> Result<T, String>;
}

impl<T, E: core::fmt::Debug> StringifyError<T> for Result<T, E> {
    fn fmt_err(self, context: &str) -> Result<T, String> {
        self.map_err(|e| format!("[{context}] {e:?}"))
    }
}

fn to_hex(array: [u8; 32]) -> String {
    let [
        a0, a1, a2, a3, a4, a5, a6, a7,
        b0, b1, b2, b3, b4, b5, b6, b7,
        c0, c1, c2, c3, c4, c5, c6, c7,
        d0, d1, d2, d3, d4, d5, d6, d7,
    ] = array;

    let a = u64::from_le_bytes([a0, a1, a2, a3, a4, a5, a6, a7]);
    let b = u64::from_le_bytes([b0, b1, b2, b3, b4, b5, b6, b7]);
    let c = u64::from_le_bytes([c0, c1, c2, c3, c4, c5, c6, c7]);
    let d = u64::from_le_bytes([d0, d1, d2, d3, d4, d5, d6, d7]);

    format!("{:016x}{:016x}{:016x}{:016x}", a, b, c, d)
}

fn from_hex(hex: &str) -> Result<[u8; 32], &'static str> {
    if hex.len() != 64 {
        return Err("from_hex: bad hexadecimal string length");
    }

    let a = u64::from_str_radix(&hex[ 0..16], 16);
    let b = u64::from_str_radix(&hex[16..32], 16);
    let c = u64::from_str_radix(&hex[32..48], 16);
    let d = u64::from_str_radix(&hex[48..64], 16);

    let [Ok(a), Ok(b), Ok(c), Ok(d)] = [a, b, c, d] else {
        return Err("from_hex: bad hexadecimal string");
    };

    let [a0, a1, a2, a3, a4, a5, a6, a7] = a.to_le_bytes();
    let [b0, b1, b2, b3, b4, b5, b6, b7] = b.to_le_bytes();
    let [c0, c1, c2, c3, c4, c5, c6, c7] = c.to_le_bytes();
    let [d0, d1, d2, d3, d4, d5, d6, d7] = d.to_le_bytes();

    let bytes = [
        a0, a1, a2, a3, a4, a5, a6, a7,
        b0, b1, b2, b3, b4, b5, b6, b7,
        c0, c1, c2, c3, c4, c5, c6, c7,
        d0, d1, d2, d3, d4, d5, d6, d7,
    ];

    Ok(bytes)
}
