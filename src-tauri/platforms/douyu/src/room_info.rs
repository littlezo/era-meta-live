// Douyu specific API logic will go here
// NOTE: This module already uses reqwest and is consistent with the unified Douyu HTTP client approach.


use serde::{Deserialize, Serialize};
use serde_json::Value;

use shared::interface::{RoomInfo, PlatformError, PlatformType};
use shared::logger::Logger;

// 创建静态日志记录器
static LOGGER: std::sync::OnceLock<Logger> = std::sync::OnceLock::new();

fn logger() -> &'static Logger {
    LOGGER.get_or_init(|| {
        Logger::new(Some(PlatformType::Douyu), "douyu::room_info")
    })
}

// Define the structure to be returned to TypeScript
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct DouyuFollowInfo {
    room_id: String,
    room_name: Option<String>,
    nickname: Option<String>,
    avatar_url: Option<String>,
    video_loop: Option<i64>,
    show_status: Option<i64>,
}

pub async fn fetch_room_info(
    client: &shared::http_client::HttpClient,
    room_id: String,
) -> Result<(RoomInfo, Option<Value>), PlatformError> {
    let url = format!("https://www.douyu.com/betard/{}", room_id);
    
    logger().log_request("fetch_room_info", &url, &None);

    let text = client
        .get_text(&url)
        .await
        .map_err(|e| PlatformError::Network(e))?;
        
    logger().debug(format!("Douyu API response text: {}", text));
    
    let full_json_value = match serde_json::from_str::<Value>(&text) {
        Ok(val) => {
            logger().debug(format!("Successfully parsed JSON for room {}", room_id));
            logger().log_structured(shared::logger::LogLevel::Debug, "Douyu房间数据", &val);
            val
        },
        Err(e) => {
            logger().error(format!("Failed to parse JSON for room {}: {}. Ensure API returns valid JSON.", room_id, e.to_string()));
            return Err(PlatformError::Parse(format!(
                "Failed to parse JSON for room {}: {}. Ensure API returns valid JSON.",
                room_id,
                e.to_string()
            )));
        }
    };
    
    logger().debug("Attempting to locate room data in JSON response");
    let room_data_ref = full_json_value
        .get("data")
        .and_then(|d| d.get("room")) // Path 1: { data: { room: { ... } } }
        .or_else(|| full_json_value.get("data")) // Path 2: { data: { ...room_info... } }
        .or_else(|| full_json_value.get("room")) // Path 3: { room: { ... } }
        .or_else(|| Some(&full_json_value)); // Path 4: { ...room_info... } (root is room object);

    let room_data = match room_data_ref {
        Some(data) => {
            logger().debug("Successfully located room data in JSON response");
            data
        },
        None => {
            logger().error(format!("Could not locate room data block in JSON response for room {}", room_id));
            return Err(PlatformError::Api(format!(
                "Could not locate room data block in JSON response for room {}",
                room_id
            )));
        }
    };

    let get_str = |val: &Value, key: &str| val.get(key).and_then(|v| v.as_str()).map(String::from);
    let get_i64 = |val: &Value, key: &str| val.get(key).and_then(|v| v.as_i64());
    let get_u64 = |val: &Value, key: &str| val.get(key).and_then(|v| v.as_u64()).or_else(|| val.get(key).and_then(|v| v.as_i64()).map(|i| i as u64));
    let _get_bool = |val: &Value, key: &str| val.get(key).and_then(|v| v.as_bool());
    let get_nested_str = |val: &Value, path: &[&str]| {
        let mut current = val;
        for key_part in path.iter() {
            current = current.get(key_part)?;
        }
        current.as_str().map(String::from)
    };

    // Prioritize avatar_mid if it exists at the room_data level, then try avatar.middle
    let avatar_final_url = get_str(room_data, "avatar_mid")
        .or_else(|| get_nested_str(room_data, &["avatar", "middle"]));
    logger().debug(format!("Extracted avatar URL: {:?}", avatar_final_url));

    // If API provides its own room_id, prefer that. Otherwise, use the input room_id.
    let final_room_id = get_str(room_data, "room_id").unwrap_or_else(|| room_id.clone());
    logger().debug(format!("Final room ID: {}", final_room_id));

    // Check live status: 1 = live, 0 = offline (Douyu API convention)
    let show_status = get_i64(room_data, "show_status").unwrap_or(0);
    let live_status = show_status == 1;
    logger().debug(format!("Live status: {}, show_status: {}", live_status, show_status));
    
    // Convert viewer count from i64 to u64 if needed
    let viewer_count = get_u64(room_data, "online").or_else(|| get_u64(room_data, "hn"));
    logger().debug(format!("Viewer count: {:?}", viewer_count));
    
    // Build unified RoomInfo structure
    let room_info = RoomInfo {
        room_id: final_room_id,
        title: get_str(room_data, "room_name").unwrap_or_else(|| "".to_string()),
        streamer_name: get_str(room_data, "nickname").unwrap_or_else(|| "".to_string()),
        streamer_id: get_str(room_data, "owner_uid").unwrap_or_else(|| "".to_string()),
        avatar_url: avatar_final_url,
        cover_url: get_str(room_data, "room_src"),
        live_status: live_status,
        live_status_detail: if live_status { "LIVE" } else { "OFFLINE" }.to_string(),
        viewer_count: viewer_count,
        viewer_count_str: viewer_count.map(|v| v.to_string()),
        category_name: get_str(room_data, "cate_name"),
        category_id: get_str(room_data, "cate_id"),
        tags: None, // Douyu API doesn't provide tags in this endpoint
        other: Some(room_data.clone()),
        raw: Some(full_json_value.clone()),
    };
    
    logger().log_response_with_raw("fetch_room_info", &room_info, &Some(full_json_value.clone()));

    Ok((room_info, Some(full_json_value)))
}
