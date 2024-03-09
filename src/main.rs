use tokio::sync::Mutex;
use std::{collections::HashMap, sync::Arc};
use tokio::net::TcpListener;
use handlers::handle_connection;
use data_types::state_types::JvsState;

mod handlers;
mod data_types;
mod utils;

#[tokio::main]
async fn main() {
    let state = Arc::new(Mutex::new(JvsState {
        rooms: HashMap::new(),
        ws_clients: HashMap::new()
    }));

    let server = TcpListener::bind("127.0.0.1:9001").await.expect("Server bind failed");

    while let Ok((stream, _)) = server.accept().await {
        let peer = stream
            .peer_addr()
            .expect("connected streams should have a peer address");
        println!("Peer address: {}", peer);

        tokio::spawn(handle_connection(state.clone(), stream, peer));
    }
}
