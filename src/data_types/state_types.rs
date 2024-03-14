use std::collections::HashMap;
use serde::Serialize;
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use futures_util::{stream::SplitSink, SinkExt};
use uuid::Uuid;
use xtra::prelude::*;

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Default)]
pub struct Room {
    pub users: HashMap<Uuid, User>,
    pub ready_count: u32,
    pub current_video: String,
    pub current_video_payload: String,
    pub history: Vec<HistoryEntry>
}

#[derive(Debug, Default, xtra::Actor)]
pub struct JvsState {
    pub rooms: HashMap<String, Room>,
    pub ws_clients: HashMap<Uuid, SplitSink<WebSocketStream<TcpStream>, Message>>
}

// Messages

pub enum StateGenericMessage {
    InsertUser { user_id: Uuid, ws: SplitSink<WebSocketStream<TcpStream>, Message> },
    RemoveUser { user_id: Uuid },
    RenameUser { user_id: Uuid, name: String, room_id: String },
    JoinRoom { user_id: Uuid, room_id: String },
    SetVideo { room_id: String, video_id: String, url: String, title: String },
    SendSocketMessage { room_id: String, message: Message }
}

pub struct StateSetReadyMessage {
    pub room_id: String
}

pub struct StateGetCurrentVideoMessage {
    pub room_id: String
}

pub struct StateGetClientsMessage {
    pub room_id: String
}

pub struct StateGetHistoryMessage {
    pub room_id: String
}

impl Handler<StateGenericMessage> for JvsState {
    type Return = ();

    async fn handle(
        &mut self,
        message: StateGenericMessage,
        _ctx: &mut Context<Self>,
    ) {
        match message {
            StateGenericMessage::InsertUser { user_id, ws } => {
                self.ws_clients.insert(user_id, ws);
            },
            StateGenericMessage::RemoveUser { user_id } => {
                self.ws_clients.remove(&user_id);

                let rooms = self.rooms.iter_mut();

                for (_key, value) in rooms {
                    if value.users.contains_key(&user_id) {
                        value.users.remove(&user_id);
                    }
                }
            },
            StateGenericMessage::RenameUser { user_id, name, room_id } => {
                let room = self.rooms.get_mut(&room_id).expect("Cannot find room");
                let user = room.users.get_mut(&user_id).expect("Cannot find user");

                user.name = name;
            },
            StateGenericMessage::JoinRoom { user_id, room_id } => {
                let room = self.rooms.entry(room_id).or_insert(Room::default());
                let user = User {
                    name: user_id.to_string()
                };

                room.users.insert(user_id, user);
            },
            StateGenericMessage::SetVideo { room_id, video_id, url, title } => {
                let room = self.rooms.get_mut(&room_id).expect("Cannot find room");

                room.current_video = video_id.clone();
                room.ready_count = 0;
                room.history.push(HistoryEntry {
                    url,
                    video_id: video_id.clone(),
                    title: title
                });
            },
            StateGenericMessage::SendSocketMessage { room_id, message } => {
                let room = self.rooms.get_mut(&room_id).expect("Failed to get users");

                for (key, _client) in room.users.iter_mut() {
                    let ws = self.ws_clients.get_mut(key).expect("Failed to find socket");

                    let _ = ws.send(message.clone()).await;
                }
            }
        };
    }
}

impl Handler<StateSetReadyMessage> for JvsState {
    type Return = bool;

    async fn handle(
        &mut self,
        message: StateSetReadyMessage,
        _ctx: &mut Context<Self>,
    ) -> bool {
        let room = self.rooms.get_mut(&message.room_id).expect("Cannot find room");

        room.ready_count += 1;

        if room.ready_count == room.users.len() as u32 {
            room.ready_count = 0;

            return true;
        }

        false
    }
}

impl Handler<StateGetCurrentVideoMessage> for JvsState {
    type Return = String;

    async fn handle(
        &mut self,
        message: StateGetCurrentVideoMessage,
        _ctx: &mut Context<Self>,
    ) -> String {
        let room = self.rooms.get_mut(&message.room_id).expect("Cannot find room");

        room.current_video.clone()
    }
}

impl Handler<StateGetClientsMessage> for JvsState {
    type Return = Vec<String>;

    async fn handle(
        &mut self,
        message: StateGetClientsMessage,
        _ctx: &mut Context<Self>,
    ) -> Vec<String> {
        let room = self.rooms.get_mut(&message.room_id).expect("Cannot find room");
        let mut connected_clients: Vec<String> = Vec::new();

        for (_key, client) in room.users.iter() {
            connected_clients.push(client.name.clone());
        }

        connected_clients
    }
}

impl Handler<StateGetHistoryMessage> for JvsState {
    type Return = Vec<HistoryEntry>;

    async fn handle(
        &mut self,
        message: StateGetHistoryMessage,
        _ctx: &mut Context<Self>,
    ) -> Vec<HistoryEntry> {
        let room = self.rooms.get_mut(&message.room_id).expect("Cannot find room");

        room.history.clone()
    }
}