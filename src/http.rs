use futures_lite::{AsyncReadExt, AsyncWriteExt};
use async_net::{TcpListener, TcpStream};
use async_channel::Sender;
use async_lock::RwLock;
use async_fs::read;

use std::str::from_utf8;

use crate::{INDEX_HTML, MAIN_JS, STYLE_CSS, session};
use crate::executor::Task;

async fn sleep_ms(millis: u64) {
    async_io::Timer::after(std::time::Duration::from_millis(millis)).await;
}

pub async fn listen(tx_tasks: Sender<Task>) {
    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();

    while let Ok((stream, _addr)) = listener.accept().await {
        let new_task = process_request(stream);
        if tx_tasks.send(new_task.into()).await.is_err() {
            println!("Failed to schedule request handler");
        }
    }
}

async fn process_request(stream: TcpStream) {
    let mut buffer = [0; 128];

    let length = loop {
        let Ok(length) = stream.peek(&mut buffer).await else {
            println!("couldn't peek at HTTP request");
            return;
        };

        let buffer = &buffer[..length];
        let idx_cr = buffer.iter().position(|b| *b == b'\r');

        if let Some(idx_cr) = idx_cr {
            break idx_cr;
        } else {
            sleep_ms(10).await;
        }
    };

    let Ok(first_line) = from_utf8(&buffer[..length]) else {
        println!("Invalid HTTP request");
        return;
    };

    let mut parts = first_line.split(' ');

    let Some(_method @ ("GET" | "POST")) = parts.next() else {
        println!("Not a GET request");
        return;
    };

    let Some(http_path) = parts.next() else {
        println!("Missing path in GET request");
        return;
    };

    let Some(http_version @ ("HTTP/1.0" | "HTTP/1.1")) = parts.next() else {
        println!("Unsupported version in GET request");
        return;
    };

    let None = parts.next() else {
        println!("Invalid HTTP request");
        return;
    };

    println!("request: {}", http_path);

    match http_path {
        "/session" => session(stream).await,
        "/" => reply_lock(stream, http_version, "200 OK", "text/html", &INDEX_HTML).await,
        "/main.js" => reply_lock(stream, http_version, "200 OK", "text/javascript", &MAIN_JS).await,
        "/style.css" => reply_lock(stream, http_version, "200 OK", "text/css", &STYLE_CSS).await,
        _ => file_reply(stream, http_version, http_path).await,
    };
}

async fn file_reply(stream: TcpStream, http_version: &str, http_path: &str) {
    let Some(file_name) = http_path.strip_prefix("/files/") else {
        return not_found(stream, http_version).await;
    };

    if file_name.contains('/') {
        return not_found(stream, http_version).await;
    }

    let file_path = format!("files/{file_name}");
    let Ok(bytes) = read(file_path).await else {
        return not_found(stream, http_version).await;
    };

    let content_type = match infer::get(&bytes) {
        Some(ftype) => ftype.mime_type(),
        None => "application/octet-stream",
    };

    reply(stream, http_version, "200 OK", content_type, &bytes).await;
}

async fn not_found(stream: TcpStream, http_version: &str) {
    reply(stream, http_version, "404 Not Found", "text/html", b"Not Found!").await;
}

async fn reply_lock(
    stream: TcpStream,
    http_version: &str,
    code: &str,
    content_type: &str,
    staticc: &RwLock<Vec<u8>>,
) {
    let payload = {
        let reader = staticc.read().await;
        reader.clone()
    };
    reply(stream, http_version, code, content_type, &payload).await;
}

async fn reply(
    mut stream: TcpStream,
    http_version: &str,
    code: &str,
    content_type: &str,
    payload: &[u8],
) {
    let cont_len = format!("Content-Length: {}\r\n", payload.len());
    let cont_type = format!("Content-Type: {}\r\n", content_type);
    let server = format!("Server: Kolab\r\n");
    let reply = format!("{http_version} {code}\r\n{cont_len}{server}{cont_type}\r\n");

    let mut buffer = [0; 128];
    let mut request = Vec::with_capacity(1024);
    while !request.ends_with(b"\r\n\r\n") {
        let Ok(len) = stream.read(&mut buffer).await else {
            println!("reply failure");
            return;
        };
        request.extend_from_slice(&buffer[..len]);
    }

    let _ = stream.write_all(reply.as_bytes()).await;
    let _ = stream.write_all(payload).await;
    let _ = stream.flush().await;
}
