use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

use shared::interface::{LiveListParams, LiveList, RoomInfo, PlatformType};
use shared::logger::Logger;
use shared::http_client::HttpClient;

// 创建静态日志记录器
static LOGGER: std::sync::OnceLock<Logger> = std::sync::OnceLock::new();

fn logger() -> &'static Logger {
    LOGGER.get_or_init(|| {
        Logger::new(Some(PlatformType::Douyu), "douyu::live_list")
    })
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct LiveStreamer {
    rid: String,
    room_name: String,
    nickname: String,
    room_src: String,
    avatar: String,
    hn: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FrontendStreamer {
    pub rid: String,
    #[serde(rename = "roomName")]
    pub room_name: String,
    pub nickname: String,
    #[serde(rename = "roomSrc")]
    pub room_src: String,
    pub avatar: String,
    pub hn: String, // Will store 'ol' (online count) as string
    #[serde(rename = "isLive", skip_serializing_if = "Option::is_none")]
    pub is_live: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LiveListDataWrapper {
    pub list: Vec<FrontendStreamer>,
    pub total: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FrontendLiveListResponse {
    pub error: i32,
    pub msg: Option<String>,
    pub data: Option<LiveListDataWrapper>,
}

// Structs for parsing Douyu's mobile API (hgapi/live/cate/newRecList) response
// Based on newRecList.json and web search result
#[derive(Deserialize, Debug)]
struct NewRecStreamerRaw {
    rid: i64, // numeric room ID
    #[serde(rename = "roomName")]
    room_name: String,
    nickname: String,
    #[serde(rename = "roomSrc")]
    room_src: String, // Main cover image
    avatar: String,
    hn: String, // Viewers count string (e.g., "101.8万")
                // rs_ext: Option<Vec<ImageRsExtRaw>>, // Removed as unused
}

#[derive(Deserialize, Debug)]
struct NewRecListDataRaw {
    list: Vec<NewRecStreamerRaw>,
    total: i32, // Total number of streamers
}

#[derive(Deserialize, Debug)]
struct NewRecListApiResponse {
    error: i32,
    msg: Option<String>,
    data: Option<NewRecListDataRaw>, // Optional because API might return error
}

// Structs for parsing Douyu's V1 API response for third-level categories
#[derive(Deserialize, Debug)]
struct DouyuV1Streamer {
    rid: u32, // Douyu uses number for rid here
    rn: String,
    nn: String,
    av: String,
    ol: u32,      // Douyu online count
    rs16: String, // Cover image
    #[serde(rename = "type")]
    stream_type: Option<u32>, // Example: type:1 might mean live
                  // Add any other fields you might need, e.g. cid3 for verification
}

#[derive(Deserialize, Debug)]
struct DouyuV1Data {
    rl: Vec<DouyuV1Streamer>,
    // Douyu's V1 directory API typically doesn't provide a total count.
    // It might have `pgcnt` (page count) in some versions, but not in the example.
    // We will estimate `total` based on the number of items returned vs. page size.
}

#[derive(Deserialize, Debug)]
struct DouyuV1ApiResponse {
    code: i32,
    msg: Option<String>,
    data: Option<DouyuV1Data>,
}

pub async fn fetch_live_list(client: &HttpClient, offset: u32, cate2: String, limit: u32) -> FrontendLiveListResponse {
    let url = format!(
        "https://m.douyu.com/hgapi/live/cate/newRecList?offset={}&cate2={}&limit={}",
        offset, cate2, limit
    );

    // Create headers for this request
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static("Mozilla/5.0 (iPhone; CPU iPhone OS 13_2_3 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/13.0.3 Mobile/15E148 Safari/604.1")
    );

    let text = match client.get_text_with_headers(&url, Some(headers)).await {
        Ok(t) => t,
        Err(e) => {
            logger().error(format!("Reqwest error: {}", e));
            return FrontendLiveListResponse {
                error: 500,
                msg: Some(format!("Network request failed: {}", e)),
                data: None,
            };
        }
    };

    match serde_json::from_str::<NewRecListApiResponse>(&text) {
        // Use new API response struct
        Ok(douyu_response) => {
            if douyu_response.error == 0 {
                if let Some(douyu_data) = douyu_response.data {
                    let streamers_transformed: Vec<FrontendStreamer> = douyu_data
                        .list
                        .into_iter()
                        .map(|s_raw| {
                            FrontendStreamer {
                                rid: s_raw.rid.to_string(),
                                room_name: s_raw.room_name,
                                nickname: s_raw.nickname,
                                avatar: s_raw.avatar,
                                room_src: s_raw.room_src, // Using main room_src for now
                                hn: s_raw.hn,
                                is_live: Some(true), // Assuming all returned by this API are live
                            }
                        })
                        .collect();

                    let frontend_data = LiveListDataWrapper {
                        list: streamers_transformed,
                        total: douyu_data.total as u32, // API returns i32, wrapper expects u32
                    };
                    FrontendLiveListResponse {
                        error: 0,
                        msg: douyu_response.msg.or_else(|| Some("Success".to_string())),
                        data: Some(frontend_data),
                    }
                } else {
                    logger().error(format!("API success but no data field. Raw: {}", text));
                    FrontendLiveListResponse {
                        error: -1,
                        msg: Some("Douyu API success code but no data field.".to_string()),
                        data: None,
                    }
                }
            } else {
                logger().error(format!("API returned error {}. Msg: {:?}. Raw: {}", douyu_response.error, douyu_response.msg, text));
                FrontendLiveListResponse {
                    error: douyu_response.error,
                    msg: douyu_response
                        .msg
                        .or_else(|| Some("Error from Douyu API".to_string())),
                    data: None,
                }
            }
        }
        Err(e) => {
            logger().error(format!("Error parsing Douyu Mobile JSON: {}. Raw: {}", e, text));
            FrontendLiveListResponse {
                error: -2,
                msg: Some(format!("Failed to parse Douyu API response: {}", e)),
                data: None,
            }
        }
    }
}

// New function for third-level categories
pub async fn fetch_live_list_for_cate3(
    client: &HttpClient,
    cate3_id: String,
    page: u32,
    limit: u32,
) -> FrontendLiveListResponse {
    let current_page = if page == 0 { 1 } else { page }; // Ensure page is at least 1 for the URL

    let url = format!(
        r"https://www.douyu.com/gapi/rkc/directory/mixListV1/3_{}/{}limit={}",
        cate3_id,
        current_page,
        limit
    );
    logger().debug(format!("Fetching URL: {}", url));

    // Create headers for this request
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
    );

    let text = match client.get_text_with_headers(&url, Some(headers)).await {
        Ok(t) => t,
        Err(e) => {
            logger().error(format!("Reqwest error: {}", e));
            return FrontendLiveListResponse {
                error: 500,
                msg: Some(format!("Network request failed: {}", e)),
                data: None,
            };
        }
    };

    match serde_json::from_str::<DouyuV1ApiResponse>(&text) {
        Ok(douyu_response) => {
            if douyu_response.code == 0 {
                if let Some(douyu_data) = douyu_response.data {
                    let streamers_transformed: Vec<FrontendStreamer> = douyu_data
                        .rl
                        .into_iter()
                        .map(|s| FrontendStreamer {
                            rid: s.rid.to_string(),
                            room_name: s.rn,
                            nickname: s.nn,
                            avatar: s.av, // This is usually a path, might need full URL prefix if not already there
                            room_src: s.rs16,
                            hn: s.ol.to_string(), // Convert online count to string
                            is_live: Some(s.stream_type.map_or(true, |st| st == 1)), // Assume live if no type or type is 1
                        })
                        .collect();

                    let total_returned = streamers_transformed.len() as u32;
                    let estimated_total = if total_returned < limit {
                        (current_page - 1) * limit + total_returned // If less than limit, means it's the last page
                    } else {
                        current_page * limit + 1 // Otherwise, assume there's at least one more page
                    };

                    let frontend_data = LiveListDataWrapper {
                        list: streamers_transformed,
                        total: estimated_total, // Using estimated total
                    };
                    FrontendLiveListResponse {
                        error: 0,
                        msg: douyu_response.msg.or_else(|| Some("Success".to_string())),
                        data: Some(frontend_data),
                    }
                } else {
                    logger().error(format!("API success but no data field. Raw: {}", text));
                    FrontendLiveListResponse {
                        error: -1,
                        msg: Some("Douyu API success code but no data field.".to_string()),
                        data: None,
                    }
                }
            } else {
                logger().error(format!("API returned error {}. Msg: {:?}. Raw: {}", douyu_response.code, douyu_response.msg, text));
                FrontendLiveListResponse {
                    error: douyu_response.code,
                    msg: douyu_response
                        .msg
                        .or_else(|| Some("Error from Douyu API".to_string())),
                    data: None,
                }
            }
        }
        Err(e) => {
            logger().error(format!("Error parsing Douyu V1 API JSON: {}. Raw: {}", e, text));
            FrontendLiveListResponse {
                error: -2,
                msg: Some(format!("Failed to parse Douyu API response: {}", e)),
                data: None,
            }
        }
    }
}

// 与LivePlatform trait兼容的获取直播列表函数，返回LiveList和原始API数据
pub async fn fetch_live_list_with_params(
    client: Arc<shared::http_client::HttpClient>,
    params: LiveListParams
) -> Result<(LiveList, Option<Value>), String> {
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);
    let offset = (page - 1) * page_size;
    
    // 根据category_id决定调用哪个API
    let response = if let Some(category_id) = &params.category_id {
        // 假设category_id是三级分类ID
        fetch_live_list_for_cate3(
            &client,
            category_id.clone().to_string(),
            page,
            page_size
        ).await
    } else {
        // 默认获取推荐直播列表
        fetch_live_list(&client, offset, "0".to_string(), page_size).await
    };
    
    if response.error != 0 {
        return Err(response.msg.unwrap_or_else(|| "API error".to_string()));
    }
    
    let data = response.data.ok_or("No data returned")?;
    
    // 克隆data.list以避免部分移动值问题
    let list_clone = data.list.clone();
    
    // 转换为统一的LiveList结构
    let items: Vec<RoomInfo> = list_clone
        .into_iter()
        .map(|streamer| {
            let rid_clone = streamer.rid.clone();
            // 将streamer转换为JSON以便作为raw数据
            let streamer_json = serde_json::json!(
                {
                    "rid": streamer.rid,
                    "room_name": streamer.room_name,
                    "nickname": streamer.nickname,
                    "avatar": streamer.avatar,
                    "room_src": streamer.room_src,
                    "is_live": streamer.is_live,
                    "hn": streamer.hn
                }
            );
            RoomInfo {
                room_id: streamer.rid,
                title: streamer.room_name,
                streamer_name: streamer.nickname,
                streamer_id: rid_clone, // 使用room_id作为streamer_id，实际应该有专门的主播ID
                avatar_url: Some(streamer.avatar),
                cover_url: Some(streamer.room_src),
                live_status: streamer.is_live.unwrap_or(true),
                live_status_detail: if streamer.is_live.unwrap_or(true) { "LIVE" } else { "OFFLINE" }.to_string(),
                viewer_count: streamer.hn.parse::<u64>().ok(),
                viewer_count_str: Some(streamer.hn),
                category_name: None,
                category_id: params.category_id.clone(),
                tags: None,
                other: None,
                raw: Some(streamer_json),
            }
        })
        .collect();
    
    // Calculate has_more before moving items
    let has_more = items.len() >= page_size as usize;
    
    // 创建一个简单的原始数据Value，使用原始data（不包含已移动的list字段）
    let raw_json = serde_json::json! ({
        "error": response.error,
        "msg": response.msg,
        "data": {
            "list": data.list,
            "total": data.total
        }
    });
    
    Ok((LiveList {
        items,
        total: Some(data.total as u64),
        page: Some(page),
        page_size: Some(page_size),
        has_more,
        other: None,
        raw: Some(raw_json.clone()),
    }, Some(raw_json)))
}