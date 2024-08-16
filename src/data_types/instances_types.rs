use anyhow::{anyhow, Result};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;
use xtra::prelude::*;

use super::response_types::InvidiousResponse;

#[derive(Serialize, Deserialize, Debug)]
struct InstanceMonitor {
    pub uptime: f64,
    pub down: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct Instance {
    pub uri: String,
    pub monitor: Option<InstanceMonitor>,
}

type InvidiousInstancesApiResponse = (String, Instance);

// Actor
#[derive(Default, xtra::Actor)]
pub struct InstancesManager {
    instances: Vec<InvidiousInstancesApiResponse>,
}

// Actor messages
pub struct InstancesFetchVideoMessage {
    pub video_id: String,
}

pub struct InstancesUpdateInstancesMessage {}

// Messages implementations
impl Handler<InstancesFetchVideoMessage> for InstancesManager {
    type Return = Result<InvidiousResponse>;

    async fn handle(
        &mut self,
        message: InstancesFetchVideoMessage,
        _ctx: &mut Context<Self>,
    ) -> Self::Return {
        let mut set = JoinSet::new();

        for instance in &self.instances {
            set.spawn(reqwest::get(format!(
                "{}/api/v1/videos/{}?fields=title,isFamilyFriendly",
                instance.1.uri, message.video_id
            )));
        }

        let mut video_info: Result<InvidiousResponse> =
            Err(anyhow!("No invidious instance could process the video"));

        while let Some(join_result) = set.join_next().await {
            match join_result {
                Ok(request_result) => match request_result {
                    Ok(response) => {
                        let instance_url =
                            &response.url().host_str().unwrap_or("Unknown").to_owned();

                        match response.status() {
                            StatusCode::OK => match response.json::<InvidiousResponse>().await {
                                Ok(info) => {
                                    println!(
                                        "Chosen instance for video {}: {}",
                                        message.video_id, instance_url
                                    );
                                    video_info = Ok(info);
                                    break;
                                }
                                Err(_) => continue,
                            },
                            _ => continue,
                        }
                    }
                    Err(_) => continue,
                },
                Err(_) => continue,
            }
        }

        set.abort_all();

        return video_info;
    }
}

impl Handler<InstancesUpdateInstancesMessage> for InstancesManager {
    type Return = Result<()>;

    async fn handle(
        &mut self,
        _message: InstancesUpdateInstancesMessage,
        _ctx: &mut Context<Self>,
    ) -> Self::Return {
        println!("updating invdious instances!!");
        // Fetch available invidious' apis
        self.instances = reqwest::get("https://api.invidious.io/instances.json")
            .await?
            .json::<Vec<InvidiousInstancesApiResponse>>()
            .await?
            .into_iter()
            .filter(|dr| match &dr.1.monitor {
                Some(monitor) => {
                    !monitor.down && monitor.uptime > 70.0 && dr.1.uri.starts_with("http")
                }
                None => false,
            })
            .collect::<Vec<InvidiousInstancesApiResponse>>();

        return Ok(());
    }
}
