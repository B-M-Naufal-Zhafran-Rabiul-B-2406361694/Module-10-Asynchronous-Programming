// Copyright 2024 Google LLC
// SPDX-License-Identifier: Apache-2.0

use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::sync::broadcast::{Sender, channel};
use tokio_websockets::{Message, ServerBuilder, WebSocketStream};

#[derive(Clone)]
struct User {
    addr: SocketAddr,
    nick: String,
}

type Users = Arc<Mutex<Vec<User>>>;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum MessageType {
    Users,
    Register,
    Message,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebSocketMessage {
    message_type: MessageType,
    data_array: Option<Vec<String>>,
    data: Option<String>,
}

#[derive(Serialize)]
struct ChatMessage {
    from: String,
    message: String,
}

async fn handle_connection(
    addr: SocketAddr,
    mut ws_stream: WebSocketStream<TcpStream>,
    bcast_tx: Sender<String>,
    users: Users,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut bcast_rx = bcast_tx.subscribe();

    loop {
        tokio::select! {
            incoming = ws_stream.next() => {
                match incoming {
                    Some(Ok(msg)) => {
                        if let Some(text) = msg.as_text() {
                            println!("From client {addr}: \"{text}\"");
                            if let Err(e) = handle_client_message(addr, text, &bcast_tx, &users).await {
                                eprintln!("Error handling message from {addr}: {e}");
                            }
                        }
                    }
                    _ => break,
                }
            }
            msg = bcast_rx.recv() => {
                ws_stream.send(Message::text(msg?)).await?;
            }
        }
    }

    remove_user(addr, &bcast_tx, &users).await?;
    Ok(())
}

async fn handle_client_message(
    addr: SocketAddr,
    text: &str,
    bcast_tx: &Sender<String>,
    users: &Users,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let message: WebSocketMessage = serde_json::from_str(text)?;

    match message.message_type {
        MessageType::Register => {
            if let Some(nick) = message.data {
                register_user(addr, nick, bcast_tx, users).await?;
            }
        }
        MessageType::Message => {
            if let Some(message) = message.data {
                let sender = {
                    let users = users.lock().await;
                    users
                        .iter()
                        .find(|user| user.addr == addr)
                        .map(|user| user.nick.clone())
                };

                if let Some(from) = sender {
                    let chat_message = ChatMessage { from, message };
                    let payload = WebSocketMessage {
                        message_type: MessageType::Message,
                        data: Some(serde_json::to_string(&chat_message)?),
                        data_array: None,
                    };

                    bcast_tx.send(serde_json::to_string(&payload)?)?;
                }
            }
        }
        MessageType::Users => {}
    }

    Ok(())
}

async fn register_user(
    addr: SocketAddr,
    nick: String,
    bcast_tx: &Sender<String>,
    users: &Users,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    {
        let mut users = users.lock().await;

        if let Some(user) = users.iter_mut().find(|user| user.addr == addr) {
            user.nick = nick;
        } else {
            users.push(User { addr, nick });
        }
    }

    broadcast_users(bcast_tx, users).await
}

async fn remove_user(
    addr: SocketAddr,
    bcast_tx: &Sender<String>,
    users: &Users,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let changed = {
        let mut users = users.lock().await;
        let before = users.len();
        users.retain(|user| user.addr != addr);
        before != users.len()
    };

    if changed {
        broadcast_users(bcast_tx, users).await?;
    }

    Ok(())
}

async fn broadcast_users(
    bcast_tx: &Sender<String>,
    users: &Users,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let data_array = {
        let users = users.lock().await;
        users.iter().map(|user| user.nick.clone()).collect()
    };

    let payload = WebSocketMessage {
        message_type: MessageType::Users,
        data_array: Some(data_array),
        data: None,
    };

    bcast_tx.send(serde_json::to_string(&payload)?)?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (bcast_tx, _) = channel(16);
    let users = Arc::new(Mutex::new(Vec::new()));

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("listening on port 8080");

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("New connection from {addr:?}");
        let bcast_tx = bcast_tx.clone();
        let users = users.clone();
        tokio::spawn(async move {
            let ws_stream = ServerBuilder::new().accept(socket).await?;
            handle_connection(addr, ws_stream, bcast_tx, users).await
        });
    }
}
