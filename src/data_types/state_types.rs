use std::collections::HashMap;
use serde::Serialize;
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use futures_util::stream::SplitSink;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct HistoryEntry {
    pub url: String,
    pub video_id: String,
    pub title: String
}

#[derive(Debug)]
pub struct User {
    pub name: String,
}

#[derive(Debug)]
pub struct Room {
    pub users: HashMap<Uuid, User>,
    pub ready_count: u32,
    pub current_video: String,
    pub current_video_payload: String,
    pub history: Vec<HistoryEntry>
}

#[derive(Debug)]
pub struct JvsState {
    pub rooms: HashMap<String, Room>,
    pub ws_clients: HashMap<Uuid, SplitSink<WebSocketStream<TcpStream>, Message>>
}