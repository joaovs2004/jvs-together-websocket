use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use xtra::prelude::*;
use dotenv::dotenv;
use std::env;

use super::response_types::YoutubeDataResponse;

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

// Actor
#[derive(Default, xtra::Actor)]
pub struct InstancesManager;

// Actor messages
pub struct InstancesFetchVideoMessage {
    pub video_id: String,
}

// Messages implementations
impl Handler<InstancesFetchVideoMessage> for InstancesManager {
    type Return = Result<YoutubeDataResponse>;

    async fn handle(
        &mut self,
        message: InstancesFetchVideoMessage,
        _ctx: &mut Context<Self>,
    ) -> Self::Return {
        dotenv().ok();
        let api_key = env::var("YOUTUBE_API_KEY").expect("You need to write the YOUTUBE_API_KEY in .env");

        let response = reqwest::get(format!(
            "https://www.googleapis.com/youtube/v3/videos?part=contentDetails,snippet&id={}&key={}",
            message.video_id, api_key
        )).await;

        let mut video_info: Result<YoutubeDataResponse> =
            Err(anyhow!("No video found"));

        match response {
            Ok(response) => {
                let youtube_response = response.json::<YoutubeDataResponse>().await;
                match youtube_response {
                    Ok(youtube_response) => {
                        video_info = Ok(youtube_response)
                    },
                    Err(_) => return video_info,
                }
            },
            Err(_) => return video_info,
        }

        return video_info;
    }
}

