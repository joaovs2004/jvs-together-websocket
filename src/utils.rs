use anyhow::{anyhow, Result};
use tokio_tungstenite::tungstenite::Message;
use xtra::WeakAddress;

use crate::data_types::state_types::{JvsState, StateGetClientsMessage, StateGenericMessage};
use crate::data_types::msg_types::ServerMsg;

pub async fn broadcast_message(msg: ServerMsg, addr: WeakAddress<JvsState>, room_id: String) -> Result<()> {
    let server_msg = serde_json::to_string(&msg).expect("Failed to serialize");

    return addr.send(StateGenericMessage::SendSocketMessage { room_id, message: Message::Text(server_msg.clone()) }).await.map_err(|e| anyhow!(e));
}

pub async fn send_connected_clients(addr: WeakAddress<JvsState>, room_id: String) -> Result<()> {
    let connected_clients = ServerMsg::ConnectedClients {
        clients: addr.send(StateGetClientsMessage { room_id: room_id.clone() }).await?
    };

    broadcast_message(connected_clients, addr, room_id).await?;

    Ok(())
}