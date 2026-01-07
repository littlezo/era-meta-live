use serde::{Deserialize, Serialize};
use shared::interface::{Message, PlatformType, RoomInfo, StreamQuality, StreamerInfo};

// 定义统一的RPC服务接口

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetRoomInfoRequest {
    pub platform: PlatformType,
    pub room_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetRoomInfoResponse {
    pub success: bool,
    pub room_info: Option<RoomInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetStreamUrlRequest {
    pub platform: PlatformType,
    pub room_id: String,
    pub quality: StreamQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetStreamUrlResponse {
    pub success: bool,
    pub stream_url: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetLiveListRequest {
    pub platform: PlatformType,
    pub category_id: Option<String>,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetLiveListResponse {
    pub success: bool,
    pub rooms: Option<Vec<RoomInfo>>,
    pub has_more: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCategoriesRequest {
    pub platform: PlatformType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCategoriesResponse {
    pub success: bool,
    pub categories: Option<Vec<serde_json::Value>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetStreamerInfoRequest {
    pub platform: PlatformType,
    pub streamer_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetStreamerInfoResponse {
    pub success: bool,
    pub streamer_info: Option<StreamerInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchStreamersRequest {
    pub platform: PlatformType,
    pub keyword: String,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchStreamersResponse {
    pub success: bool,
    pub streamers: Option<Vec<StreamerInfo>>,
    pub has_more: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartMessageListenerRequest {
    pub platform: PlatformType,
    pub room_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartMessageListenerResponse {
    pub success: bool,
    pub listener_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopMessageListenerRequest {
    pub listener_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopMessageListenerResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageNotification {
    pub listener_id: String,
    pub message: Message,
}

// RPC服务定义
#[async_trait::async_trait]
pub trait RpcService {
    async fn get_room_info(&self, request: GetRoomInfoRequest) -> GetRoomInfoResponse;
    async fn get_stream_url(&self, request: GetStreamUrlRequest) -> GetStreamUrlResponse;
    async fn get_live_list(&self, request: GetLiveListRequest) -> GetLiveListResponse;
    async fn get_categories(&self, request: GetCategoriesRequest) -> GetCategoriesResponse;
    async fn get_streamer_info(&self, request: GetStreamerInfoRequest) -> GetStreamerInfoResponse;
    async fn search_streamers(&self, request: SearchStreamersRequest) -> SearchStreamersResponse;
    async fn start_message_listener(&self, request: StartMessageListenerRequest) -> StartMessageListenerResponse;
    async fn stop_message_listener(&self, request: StopMessageListenerRequest) -> StopMessageListenerResponse;
}
