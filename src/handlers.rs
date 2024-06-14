use anyhow::Result;
use futures_util::StreamExt;
use reqwest::Client;
use tokio::net::TcpStream;
use tokio::time;
use tokio_tungstenite::tungstenite::Message;
use xtra::WeakAddress;
use std::net::SocketAddr;
use std::time::Duration;

use tokio_tungstenite::accept_async;
use uuid::Uuid;

use crate::data_types::msg_types::{ClientMsg, ServerMsg};
use crate::data_types::state_types::{JvsState, StateGenericMessage, StateGetCurrentVideoMessage, StateGetHistoryMessage, StateRemoveUserMessage, StateSetReadyMessage};
use crate::data_types::response_types::InvidiousResponse;
use crate::utils::{broadcast_message, send_connected_clients};

use url::Url;

pub async fn handle_connection(
    addr: WeakAddress<JvsState>,
    stream: TcpStream,
    peer: SocketAddr,
) -> Result<()> {
    let ws_stream = accept_async(stream).await.expect("Failed to accept");
    println!("New WebSocket connection: {}", peer);

    // let mut clone_stream = ws_stream.clone();

    let (ws_sender, mut ws_receiver) = ws_stream.split();

    let mut interval = time::interval(Duration::from_secs(20));

    interval.tick().await;

    // Add new user to Room on connection
    let user_id = Uuid::new_v4();

    addr.send(StateGenericMessage::InsertUser { user_id, ws: ws_sender }).await?;

    // Handle incoming WebSocket messages
    loop {
        tokio::select! {
            val = ws_receiver.next() => {
                match val.unwrap() {
                    Ok(msg) => {
                        if msg.is_text() {
                            if let Message::Text(msg) = msg {
                                handle_msg(&msg, addr.clone(), user_id).await?;
                                addr.send(StateGenericMessage::SendMsgToUser { user_id, message: ServerMsg::UnlockSetVideo }).await?;
                            }
                        } else if msg.is_close() {
                            let room_id = addr.send(StateRemoveUserMessage { user_id }).await?;

                            if let Some (room_id) = room_id {
                                send_connected_clients(addr.clone(), room_id).await?;
                            }

                            break;
                        }
                    }
                    Err(_) => break,
                }
            },
            _val = interval.tick() => {
                addr.send(StateGenericMessage::SendMsgToUser { user_id, message: ServerMsg::Ping }).await?;
            }
        }
    }

    Ok(())
}

async fn handle_msg(
    msg: &str,
    addr: WeakAddress<JvsState>,
    user_id: Uuid,
) -> Result<()> {
    let client_msg = serde_json::from_str::<ClientMsg>(msg);

    if client_msg.is_err() {
        return Ok(());
    }

    let client_msg = client_msg.unwrap();

    match client_msg {
        ClientMsg::SetName { name, room_id } => {
            let result = addr.send(StateGenericMessage::RenameUser { user_id, name, room_id: room_id.clone() }).await;

            match result {
                Ok(_) => send_connected_clients(addr, room_id).await?,
                Err(_) => println!("User not found!")
            }
        }
        ClientMsg::SetReady { room_id } => {
            let should_play = addr.send(StateSetReadyMessage { room_id: room_id.clone() }).await?;

            if should_play {
                let set_playing = ServerMsg::SetPlaying {
                    status: true
                };

                broadcast_message(set_playing, addr, room_id).await?;
            }
        },
        ClientMsg::SendToRoom { room_id } => {
            addr.send(StateGenericMessage::JoinRoom { room_id: room_id.clone(), user_id }).await?;

            let room_history = addr.send(StateGetHistoryMessage { room_id: room_id.clone() }).await?;
            let history = ServerMsg::UpdateHistory { history: room_history };
            broadcast_message(history, addr.clone(), room_id.clone()).await?;

            send_connected_clients(addr, room_id).await?;
        },
        ClientMsg::SetVideo { url, room_id } => {
            let parsed_url = Url::parse(
                &url
            )?;

            if !["www.youtube.com", "youtube.com", "youtu.be"].contains(&parsed_url.host_str().unwrap_or("google.com")) {
                return Ok(());
            }

            let video_id = match parsed_url.host_str().unwrap() {
                "youtu.be" => parsed_url.path()[1..].to_string(),
                "youtube.com" | "www.youtube.com" => {
                    parsed_url.query_pairs().find(|p| p.0 == "v").unwrap_or(("".into(), "".into())).1.to_string()
                },
                _ => String::default()
            };

            if video_id == "" {
                return Ok(());
            }

            let room_current_video = addr.send(StateGetCurrentVideoMessage { room_id: room_id.clone() }).await?;

            if room_current_video == video_id {
                return Ok(());
            }

            let client = Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .unwrap();

            let basic_info = client.get(format!("https://inv.tux.pizza/api/v1/videos/{}?fields=title,isFamilyFriendly", video_id))
                .timeout(Duration::from_secs(60))
                .send().await?.json::<InvidiousResponse>().await?;

            addr.send(StateGenericMessage::SetVideo { room_id: room_id.clone(), video_id: video_id.clone(), url, title: basic_info.title }).await?;

            let room_history = addr.send(StateGetHistoryMessage { room_id: room_id.clone() }).await?;

            if basic_info.is_family_friendly {
                let payload = ServerMsg::SetVideo { video_id, is_restricted_video: false };
                broadcast_message(payload, addr.clone(), room_id.clone()).await?;

                let history = ServerMsg::UpdateHistory { history: room_history };
                broadcast_message(history, addr.clone(), room_id.clone()).await?;
            }
        },
        ClientMsg::SetPlaying { status, room_id } => {
            let set_playing = ServerMsg::SetPlaying {
                status
            };

            broadcast_message(set_playing, addr, room_id).await?;
        },
        ClientMsg::Seeked { time, room_id } => {
            let seek = ServerMsg::Seeked { time };

            broadcast_message(seek, addr, room_id).await?;
        },
        ClientMsg::SetPlaybackRate { rate, room_id } => {
            let rate = ServerMsg::SetPlaybackRate { rate };
            broadcast_message(rate, addr, room_id).await?;
        },
        ClientMsg::Pong => {
            println!("Client is alive");
        }
    }

    Ok(())
}