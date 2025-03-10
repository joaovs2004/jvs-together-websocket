use anyhow::Result;
use data_types::state_types::JvsState;
use handlers::handle_connection;
use tokio::net::TcpListener;
use xtra::Mailbox;

use crate::data_types::instances_types::InstancesManager;

mod data_types;
mod handlers;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    let state_addr = xtra::spawn_tokio(JvsState::default(), Mailbox::unbounded());
    let instances_addr = xtra::spawn_tokio(InstancesManager::default(), Mailbox::unbounded());
    let server = TcpListener::bind("127.0.0.1:9001").await.expect("Server bind failed");

    loop {
        tokio::select! {
            Ok((stream, _)) = server.accept() => {
                let peer = stream
                    .peer_addr()
                    .expect("connected streams should have a peer address");
                println!("Peer address: {}", peer);

                tokio::spawn(handle_connection(state_addr.downgrade(), instances_addr.downgrade(), stream, peer));
            },
        }
    }
}
