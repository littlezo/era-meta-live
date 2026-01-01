use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use tauri::{command, AppHandle, Emitter, State};
use tokio::sync::{mpsc, Mutex};

use platforms::common::FollowHttpClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlatformKind {
    Douyu,
    Douyin,
    Bilibili,
    Huya,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowStreamerInput {
    pub platform: PlatformKind,
    pub id: String,
    pub nickname: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchConfig {
    pub streamers: Vec<FollowStreamerInput>,
    pub interval_ms: u64,
    pub enable_notification: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowStatusUpdate {
    pub platform: PlatformKind,
    pub id: String,
    pub live_status: String, // "LIVE" | "OFFLINE" | "REPLAY" | "UNKNOWN"
    pub nickname: Option<String>,
    pub room_title: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Default)]
pub struct FollowWatchState {
    pub task_handle: StdMutex<Option<tokio::task::JoinHandle<()>>>,
    pub shutdown_tx: StdMutex<Option<mpsc::Sender<()>>>,
    pub last_status: Arc<Mutex<HashMap<String, String>>>, // key: "<platform>:<id>", value: live_status
    pub current_config: StdMutex<Option<WatchConfig>>,
}

fn key_of(s: &FollowStreamerInput) -> String {
    format!(
        "{}:{}",
        match s.platform {
            PlatformKind::Douyu => "douyu",
            PlatformKind::Douyin => "douyin",
            PlatformKind::Bilibili => "bilibili",
            PlatformKind::Huya => "huya",
        },
        s.id
    )
}

#[command]
pub async fn start_follow_watch_service(
    app: AppHandle,
    follow_http: State<'_, FollowHttpClient>,
    state: State<'_, FollowWatchState>,
    config: WatchConfig,
) -> Result<(), String> {
    let (tx, mut rx) = mpsc::channel::<()>(1);

    // stop existing
    if let Some(prev) = state.task_handle.lock().unwrap().take() {
        prev.abort();
    }
    if let Some(prev_tx) = state.shutdown_tx.lock().unwrap().take() {
        let _ = prev_tx.try_send(());
    }
    *state.shutdown_tx.lock().unwrap() = Some(tx.clone());
    *state.current_config.lock().unwrap() = Some(config.clone());

    let http_client = follow_http.0.clone();
    let app_handle = app.clone();
    let status_map = state.last_status.clone();

    let handle = tokio::spawn(async move {
        loop {
            // shutdown check
            if let Ok(_) = rx.try_recv() {
                break;
            }
            // read config snapshot
            let cfg = config.clone();
            for s in cfg.streamers.iter() {
                let platform = &s.platform;
                let id = s.id.clone();
                let mut live_status = "UNKNOWN".to_string();
                let mut nickname: Option<String> = s.nickname.clone();
                let mut room_title: Option<String> = None;
                let mut avatar_url: Option<String> = None;

                match platform {
                    PlatformKind::Douyin => {
                        let normalized_id = platforms::douyin::web_api::normalize_douyin_live_id(&id);
                        match platforms::douyin::web_api::fetch_room_data(&http_client, &normalized_id, None).await {
                            Ok(room_data) => {
                                let room = room_data.room;
                                let status_num = room.get("status").and_then(|v| v.as_i64()).unwrap_or_default();
                                live_status = if status_num == 2 { "LIVE".to_string() } else { "OFFLINE".to_string() };
                                nickname = room.get("anchor_name").and_then(|v| v.as_str()).map(|s| s.to_string()).or(nickname);
                                room_title = room.get("title").and_then(|v| v.as_str()).map(|s| s.to_string());
                                avatar_url = platforms::douyin::douyin_streamer_detail::extract_avatar(&room);
                            }
                            Err(_) => {
                                live_status = "UNKNOWN".to_string();
                            }
                        }
                    }
                    PlatformKind::Douyu => {
                        use reqwest::header::{HeaderMap, HeaderValue};
                        let mut headers = HeaderMap::new();
                        headers.insert("Accept", HeaderValue::from_static("application/json, text/plain, */*"));
                        headers.insert("Accept-Language", HeaderValue::from_static("zh-CN,zh;q=0.9"));
                        headers.insert("Cache-Control", HeaderValue::from_static("no-cache"));
                        headers.insert("Pragma", HeaderValue::from_static("no-cache"));
                        headers.insert("Referer", HeaderValue::from_str(&format!("https://www.douyu.com/{}", id)).unwrap());
                        headers.insert("User-Agent", HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36"));
                        let resp = http_client
                            .inner
                            .get(format!("https://www.douyu.com/betard/{}", id))
                            .headers(headers)
                            .send()
                            .await;
                        if let Ok(res) = resp {
                            if res.status().is_success() {
                                if let Ok(json) = res.json::<serde_json::Value>().await {
                                    let room_data_ref = json
                                        .get("data")
                                        .and_then(|d| d.get("room"))
                                        .or_else(|| json.get("data"))
                                        .or_else(|| json.get("room"))
                                        .or_else(|| Some(&json));
                                    if let Some(room) = room_data_ref {
                                        let s_status = room.get("show_status").and_then(|v| v.as_i64()).unwrap_or_default();
                                        let v_loop = room.get("videoLoop").and_then(|v| v.as_i64()).unwrap_or_default();
                                        live_status = if s_status == 1 {
                                            if v_loop == 1 { "REPLAY".to_string() } else { "LIVE".to_string() }
                                        } else { "OFFLINE".to_string() };
                                        nickname = room.get("nickname").and_then(|v| v.as_str()).map(|s| s.to_string()).or(nickname);
                                        room_title = room.get("room_name").and_then(|v| v.as_str()).map(|s| s.to_string());
                                        avatar_url = room.get("avatar_mid").and_then(|v| v.as_str()).map(|s| s.to_string())
                                            .or_else(|| room.get("avatar").and_then(|a| a.get("middle")).and_then(|v| v.as_str()).map(|s| s.to_string()));
                                    }
                                }
                            } else {
                                live_status = "UNKNOWN".to_string();
                            }
                        } else {
                            live_status = "UNKNOWN".to_string();
                        }
                    }
                    PlatformKind::Bilibili => {
                        use reqwest::header::{HeaderMap, HeaderValue, REFERER, USER_AGENT};
                        let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36";
                        let mut headers = HeaderMap::new();
                        headers.insert(USER_AGENT, HeaderValue::from_str(ua).unwrap());
                        headers.insert(REFERER, HeaderValue::from_static("https://live.bilibili.com/"));
                        let client = &http_client.inner;
                        // get wbi keys
                        let keys = platforms::bilibili::streamer_info::get_wbi_keys(client, &headers).await;
                        if let Ok((img_key, sub_key)) = keys {
                            let (wts, w_rid) = platforms::bilibili::streamer_info::build_wbi_sign(&id, &img_key, &sub_key);
                            let base = "https://api.live.bilibili.com/xlive/web-room/v1/index/getInfoByRoom";
                            let params = vec![("room_id", id.clone()), ("wts", wts), ("w_rid", w_rid)];
                            if let Ok(resp) = client.get(base).headers(headers.clone()).query(&params).send().await {
                                if let Ok(text) = resp.text().await {
                                    if let Ok(j) = serde_json::from_str::<serde_json::Value>(&text) {
                                        let data = j["data"].clone();
                                        let base_info = data["anchor_info"]["base_info"].clone();
                                        let room_info = data["room_info"].clone();
                                        room_title = room_info["title"].as_str().map(|s| s.to_string());
                                        nickname = base_info["uname"].as_str().map(|s| s.to_string()).or(nickname);
                                        avatar_url = base_info["face"].as_str().map(|s| s.to_string());
                                        let ls = room_info["live_status"].as_i64().unwrap_or(0) as i32;
                                        live_status = if ls == 1 { "LIVE".to_string() } else { "OFFLINE".to_string() };
                                    }
                                }
                            }
                        } else {
                            live_status = "UNKNOWN".to_string();
                        }
                    }
                    PlatformKind::Huya => {
                        live_status = "UNKNOWN".to_string();
                    }
                }

                // emit update event
                let update = FollowStatusUpdate {
                    platform: platform.clone(),
                    id: id.clone(),
                    live_status: live_status.clone(),
                    nickname: nickname.clone(),
                    room_title: room_title.clone(),
                    avatar_url: avatar_url.clone(),
                };
                let _ = app_handle.emit("follow_status_update", update.clone());

                // notification on LIVE transition
                if cfg.enable_notification {
                    let key = key_of(s);
                    let mut map = status_map.lock().await;
                    let prev = map.get(&key).cloned().unwrap_or_default();
                    if prev != "LIVE" && live_status == "LIVE" {
                        let title = "开播提醒";
                        let body = format!(
                            "{} 开播了！\n{}",
                            nickname.clone().unwrap_or_else(|| id.clone()),
                            room_title.clone().unwrap_or_default()
                        );
                        let _ = app_handle.emit("notify", serde_json::json!({ "title": title, "body": body }));
                    }
                    map.insert(key, live_status.clone());
                }
            }

            // sleep for interval
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_millis(config.interval_ms)) => {},
                _ = rx.recv() => { break; }
            }
        }
    });

    *state.task_handle.lock().unwrap() = Some(handle);
    Ok(())
}

#[command]
pub async fn stop_follow_watch_service(state: State<'_, FollowWatchState>) -> Result<(), String> {
    if let Some(handle) = state.task_handle.lock().unwrap().take() {
        handle.abort();
    }
    let maybe_tx = { state.shutdown_tx.lock().unwrap().take() };
    if let Some(tx) = maybe_tx {
        let _ = tx.send(()).await;
    }
    Ok(())
}

#[command]
pub async fn send_test_notification(app: AppHandle) -> Result<(), String> {
    let _ = app.emit("notify", serde_json::json!({
        "title": "测试通知",
        "body": "这是一条测试通知，如果您能看到这条消息，说明通知功能正常工作。"
    }));
    Ok(())
}
