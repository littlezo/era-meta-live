use serde::{Deserialize, Serialize};
use std::sync::Arc;

use shared::interface::{LiveListParams, LiveList, RoomInfo, PlatformType};
use shared::http_client::HttpClient;
use shared::logger::Logger;

// 创建静态日志记录器
static LOGGER: std::sync::OnceLock<Logger> = std::sync::OnceLock::new();

fn logger() -> &'static Logger {
    LOGGER.get_or_init(|| {
        Logger::new(Some(PlatformType::Huya), "huya::live_list")
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HuyaStreamerFrontend {
    pub room_id: String,
    pub title: String,
    pub nickname: String,
    pub avatar: String,
    pub room_cover: String,
    pub viewer_count_str: String,
    pub platform: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HuyaLiveListFrontendResponse {
    pub error: i32,
    pub msg: Option<String>,
    pub data: Option<Vec<HuyaStreamerFrontend>>, // simple list, frontend can decide pagination by page size
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct HuyaApiResponse {
    #[serde(rename = "vList")]
    v_list: Option<Vec<serde_json::Value>>, // Items are dynamic; we'll map selectively
}

fn map_huya_item_to_frontend(item: &serde_json::Value) -> Option<HuyaStreamerFrontend> {
    let s_nick = item
        .get("sNick")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let s_intro = item
        .get("sIntroduction")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let s_screenshot = item
        .get("sScreenshot")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let l_profile_room = item
        .get("lProfileRoom")
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
        .to_string();
    let s_avatar_180 = item
        .get("sAvatar180")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let l_user_count = item.get("lUserCount").and_then(|v| v.as_i64()).unwrap_or(0);

    // Viewer count string: format simple number with suffix if large
    let viewer_count_str = if l_user_count >= 10_000 {
        format!("{:.1}万", (l_user_count as f64) / 10_000.0)
    } else {
        l_user_count.to_string()
    };

    Some(HuyaStreamerFrontend {
        room_id: l_profile_room,
        title: s_intro,
        nickname: s_nick,
        avatar: s_avatar_180,
        room_cover: s_screenshot,
        viewer_count_str,
        platform: "huya".to_string(),
    })
}

pub async fn fetch_huya_live_list(
    client: Arc<HttpClient>,
    params: LiveListParams
) -> Result<(LiveList, Option<serde_json::Value>), String> {
    let i_gid = params.category_id.unwrap_or_else(|| "0".to_string());
    let i_page_no = params.page.unwrap_or(1);
    let i_page_size = params.page_size.unwrap_or(20);
    
    let url = format!(
        "https://live.huya.com/liveHttpUI/getLiveList?iGid={}&iPageNo={}&iPageSize={}",
        urlencoding::encode(&i_gid),
        i_page_no,
        i_page_size
    );

    let resp_value: serde_json::Value = client
        .get_json(&url)
        .await
        .map_err(|e| {
            logger().error(format!("Request failed: {}", e));
            format!("Request failed: {}", e)
        })?;
    
    // 保存原始响应数据
    let raw_data = Some(resp_value.clone());

    // 兼容两种可能的返回结构：顶层 vList 或 data.vList
    let v_list_opt = resp_value
        .get("vList")
        .and_then(|v| v.as_array())
        .cloned()
        .or_else(|| {
            resp_value
                .get("data")
                .and_then(|d| d.get("vList"))
                .and_then(|v| v.as_array())
                .cloned()
        });

    if let Some(arr) = v_list_opt {
        let items: Vec<RoomInfo> = arr
            .iter()
            .filter_map(|item| map_huya_item_to_frontend(item))
            .map(|streamer| {
                let room_id_clone = streamer.room_id.clone();
                let i_gid_clone = i_gid.clone();
                // 将streamer转换为JSON以便作为raw数据
                let streamer_json = serde_json::json!(
                    {
                        "room_id": streamer.room_id,
                        "title": streamer.title,
                        "nickname": streamer.nickname,
                        "avatar": streamer.avatar,
                        "room_cover": streamer.room_cover,
                        "viewer_count_str": streamer.viewer_count_str,
                        "i_gid": i_gid_clone
                    }
                );
                RoomInfo {
                    room_id: streamer.room_id,
                    title: streamer.title,
                    streamer_name: streamer.nickname,
                    streamer_id: room_id_clone, // 使用room_id作为streamer_id，实际应该有专门的主播ID
                    avatar_url: Some(streamer.avatar),
                    cover_url: Some(streamer.room_cover),
                    live_status: true, // 直播列表中的主播应该都是在线的
                    live_status_detail: "LIVE".to_string(),
                    viewer_count: Some(
                        streamer.viewer_count_str.replace("万", "0000")
                            .parse::<u64>()
                            .unwrap_or(0)
                    ),
                    viewer_count_str: Some(streamer.viewer_count_str),
                    category_name: None,
                    category_id: Some(i_gid_clone),
                    tags: None,
                    other: None,
                    raw: Some(streamer_json),
                }
            })
            .collect();
        
        // Calculate has_more before moving items
        let has_more = items.len() >= i_page_size as usize;
        let live_list = LiveList {
            items,
            total: None,
            page: Some(i_page_no),
            page_size: Some(i_page_size),
            has_more,
            other: None,
            raw: raw_data.clone(),
        };
        
        // 返回原始API数据
        Ok((live_list, raw_data))
    } else {
        Err("No vList in response".to_string())
    }
}
