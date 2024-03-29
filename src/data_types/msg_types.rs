use serde::{Serialize, Deserialize};

use super::state_types::HistoryEntry;

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all(deserialize = "camelCase"), rename_all_fields = "camelCase")]
pub enum ClientMsg {
    SetName { name: String, room_id: String },
    SetReady {room_id: String},
    SendToRoom { room_id: String},
    SetVideo { url: String, room_id: String },
    SetPlaying { status: bool, room_id: String },
    Seeked { time: f64, room_id: String },
    SetPlaybackRate { rate: f32, room_id: String }
}

#[derive(Serialize, Debug)]
#[serde(tag = "type", rename_all(serialize = "camelCase"), rename_all_fields = "camelCase")]
pub enum ServerMsg {
    SetPlaying { status: bool },
    ConnectedClients { clients: Vec<String> },
    SetVideo { video_id: String, is_restricted_video: bool },
    UpdateHistory { history: Vec<HistoryEntry> },
    Seeked { time: f64 },
    UnlockSetVideo,
    SetPlaybackRate { rate: f32 }
}