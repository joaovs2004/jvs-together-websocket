use serde::Deserialize;

#[derive(Deserialize)]
pub struct YoutubeDataResponse {
    pub items: Vec<YoutubeDataItem>
}

#[derive(Deserialize)]
pub struct YoutubeDataItem {
    pub snippet: Snippet,
    #[serde(rename = "contentDetails")]
    pub content_details: ContentDetails
}

#[derive(Deserialize)]
pub struct Snippet {
    pub title: String
}

#[derive(Deserialize)]
pub struct ContentDetails {
    #[serde(rename = "contentRating")]
    pub content_rating: ContentRating
}

#[derive(Deserialize)]
pub struct ContentRating {
    #[serde(rename = "ytRating")]
    pub yt_rating: Option<String>
}