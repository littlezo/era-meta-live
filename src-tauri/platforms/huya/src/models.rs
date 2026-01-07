use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct HuyaUnifiedStreamEntry {
    pub quality: String,
    pub bit_rate: i32,
    pub url: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct HuyaUnifiedResponse {
    pub title: Option<String>,
    pub nick: Option<String>,
    pub avatar: Option<String>,
    pub introduction: Option<String>,
    pub profile_room: Option<String>,
    pub is_live: bool,
    pub flv_tx_urls: Vec<HuyaUnifiedStreamEntry>,
    pub selected_url: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RoomDetail {
    pub status: bool,
    pub title: Option<String>,
    pub nick: Option<String>,
    pub avatar180: Option<String>,
}

#[derive(Clone, Debug)]
pub struct WebStreamCandidate {
    pub base_flv: String,
    pub cdn: String,
}

#[derive(Clone, Debug)]
pub struct HuyaWebStreamData {
    pub is_live: bool,
    pub candidates: Vec<WebStreamCandidate>,
}