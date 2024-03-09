use anyhow::Result;
use futures_util::StreamExt;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio_tungstenite::accept_async;
use uuid::Uuid;

use crate::data_types::msg_types::{ClientMsg, ServerMsg};
use crate::data_types::state_types::{HistoryEntry, JvsState, Room, User};
use crate::data_types::response_types::InvidiousResponse;
use crate::utils::{broadcast_message, send_connected_clients};

use url::Url;



pub async fn handle_connection(
    state: Arc<Mutex<JvsState>>,
    stream: TcpStream,
    peer: SocketAddr,
) -> Result<()> {
    let ws_stream = accept_async(stream).await.expect("Failed to accept");
    println!("New WebSocket connection: {}", peer);

    // let mut clone_stream = ws_stream.clone();

    let (ws_sender, mut ws_receiver) = ws_stream.split();

    // Add new user to Room on connection
    let user_id = Uuid::new_v4();

    state.lock().await.ws_clients.insert(user_id, ws_sender);

    // Handle incoming WebSocket messages
    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(msg) => {
                if msg.is_text() {
                    if let Message::Text(msg) = msg {
                        handle_msg(&msg, &state, user_id).await?;
                    }
                } else if msg.is_close() {
                    let mut state = state.lock().await;

                    state.ws_clients.remove(&user_id);

                    let rooms = state.rooms.iter_mut();

                    for (_key, value) in rooms {
                        if value.users.contains_key(&user_id) {
                            value.users.remove(&user_id);
                        }
                    }

                    break;
                }
            }
            Err(_) => break,
        }
    }

    Ok(())
}

async fn handle_msg(
    msg: &str,
    state: &Arc<Mutex<JvsState<>>>,
    user_id: Uuid,
) -> Result<()> {
    let mut state = state.lock().await;

    let client_msg = serde_json::from_str::<ClientMsg>(msg);

    if client_msg.is_err() {
        println!("{}", msg);
        return Ok(());
    }

    let client_msg = client_msg.unwrap();


    match client_msg {
        ClientMsg::SetName { name, room_id } => {
            let room = state.rooms.get_mut(&room_id).expect("Cannot find room");
            let user = room.users.get_mut(&user_id);

            match user {
                Some(user) => {
                    user.name = name;
                    send_connected_clients(&mut state, room_id).await?;
                },
                None => { println!("User not found!") },
            };
        }
        ClientMsg::SetReady { room_id } => {
            let room = state.rooms.get_mut(&room_id).expect("Cannot find room");

            room.ready_count += 1;

            if room.ready_count == room.users.len() as u32 {
                room.ready_count = 0;

                let set_playing = ServerMsg::SetPlaying {
                    status: true
                };

                broadcast_message(set_playing, &mut state, room_id).await?;
            }
        },
        ClientMsg::SendToRoom { room_id } => {
            let room = state.rooms.entry(room_id.clone()).or_insert(Room {
                users: HashMap::new(),
                ready_count: 0,
                current_video: String::from(""),
                current_video_payload: String::from(""),
                history: Vec::new(),
            });

            let user = User {
                name: user_id.to_string()
            };

            room.users.insert(user_id, user);

            send_connected_clients(&mut state, room_id).await?;
        },
        ClientMsg::SetVideo { url, room_id } => {
            let parsed_url = Url::parse(
                &url
            )?;

            if !["www.youtube.com", "youtube.com", "youtu.be"].contains(&parsed_url.host_str().unwrap_or("google.com")) {
                println!("a");
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

            let room = state.rooms.get_mut(&room_id).expect("Cannot find room");

            if room.current_video == video_id {
                return Ok(());
            }

            let basic_info = reqwest::get(format!("https://invidious.slackjeff.com.br/api/v1/videos/{}?fields=title,isFamilyFriendly", video_id)).await?.json::<InvidiousResponse>().await?;

            room.current_video = video_id.clone();
            room.ready_count = 0;
            room.history.push(HistoryEntry {
                url,
                video_id: video_id.clone(),
                title: basic_info.title
            });

            if basic_info.is_family_friendly {
                let payload = ServerMsg::SetVideo { video_id: video_id.clone(), is_restricted_video: false };
                broadcast_message(payload, &mut state, room_id).await?;

                // let history = ServerMsg::UpdateHistory { history:  room.history };
                // broadcast_message(history, &mut state, room_id).await?;
            }
        },
        ClientMsg::SetPlaying { status, room_id } => {
            let set_playing = ServerMsg::SetPlaying {
                status
            };

            broadcast_message(set_playing, &mut state, room_id).await?;
        }
    }

    Ok(())
}