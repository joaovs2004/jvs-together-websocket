use anyhow::Result;
use futures_util::SinkExt;
use tokio_tungstenite::tungstenite::Message;

use crate::data_types::state_types::JvsState;
use crate::data_types::msg_types::ServerMsg;

pub async fn broadcast_message(msg: ServerMsg, state: &mut JvsState, room_id: String) -> Result<()> {
    let room = state.rooms.get_mut(&room_id).expect("Failed to get users");

    let server_msg = serde_json::to_string(&msg).expect("Failed to serialize");

    for (key, _client) in room.users.iter_mut() {
        let ws = state.ws_clients.get_mut(key).expect("Failed to find socket");
        ws.send(Message::Text(server_msg.clone())).await?;
    }

    Ok(())
}

pub async fn send_connected_clients(state: &mut JvsState, room_id: String) -> Result<()> {
    let room = state.rooms.get_mut(&room_id).expect("Cannot find room");

    let mut connected_clients: Vec<String> = Vec::new();

    for (_key, client) in room.users.iter() {
        connected_clients.push(client.name.clone());
    }

    let connected_clients = ServerMsg::ConnectedClients {
        clients: connected_clients
    };

    broadcast_message(connected_clients, state, room_id).await?;

    Ok(())
}