use serde::Deserialize;

#[derive(Deserialize)]
pub struct InvidiousResponse {
    pub title: String,
    #[serde(rename = "isFamilyFriendly")]
    pub is_family_friendly: bool
}