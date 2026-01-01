// Wrapper functions for platform commands from the platforms crate
// These wrapper functions allow the frontend to call commands from the platforms crate

use platforms::common::{GetStreamUrlPayload, LiveStreamInfo};
use tauri::{AppHandle, State};

// Douyin commands - only include the ones actually used by the frontend
#[tauri::command]
pub async fn fetch_douyin_streamer_info(
    payload: GetStreamUrlPayload,
    follow_http: State<'_, platforms::common::FollowHttpClient>,
) -> Result<LiveStreamInfo, String> {
    platforms::douyin::fetch_douyin_streamer_info(payload, follow_http).await
}

#[tauri::command]
pub async fn get_douyin_live_stream_url_with_quality(
    app_handle: AppHandle,
    payload: GetStreamUrlPayload,
    quality: String,
    cookie: Option<String>,
) -> Result<LiveStreamInfo, String> {
    platforms::douyin::douyin_streamer_detail::get_douyin_live_stream_url_with_quality(
        app_handle,
        payload,
        quality,
        cookie,
    )
    .await
}

#[tauri::command]
pub async fn start_douyin_danmu_listener(
    payload: GetStreamUrlPayload,
    app_handle: AppHandle,
    state: State<'_, platforms::common::DouyinDanmakuState>,
) -> Result<(), String> {
    platforms::douyin::douyin_danmu_listener::start_douyin_danmu_listener(
        payload,
        app_handle,
        state,
    )
    .await
}
