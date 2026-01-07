use shared::http_client::HttpClient;
use shared::types::StreamVariant;
use shared::interface::PlatformType;
use shared::logger::Logger;

use shared::LiveStreamInfo as CommonLiveStreamInfo;
use crate::web_api::{
    choose_flv_stream, fetch_room_data, normalize_douyin_live_id, DouyinRoomData,
};
// use crate::proxy::ProxyServerHandle; // Proxy is now in main crate
// use crate::StreamUrlStore; // Use the one from common types instead
use serde_json::Value;

// 创建静态日志记录器
static LOGGER: std::sync::OnceLock<Logger> = std::sync::OnceLock::new();

fn logger() -> &'static Logger {
    LOGGER.get_or_init(|| {
        Logger::new(Some(PlatformType::Douyin), "douyin::stream_url")
    })
}

const QUALITY_OD: &str = "OD";
const QUALITY_BD: &str = "BD";
const QUALITY_UHD: &str = "UHD";

pub async fn get_douyin_live_stream_url(
    room_id: &str,
    cookie: Option<&str>,
) -> Result<CommonLiveStreamInfo, String> {
    get_douyin_live_stream_url_with_quality(
        room_id,
        QUALITY_OD.to_string(),
        cookie,
    )
    .await
}

pub async fn get_douyin_live_stream_url_with_quality(
    room_id: &str,
    quality: String,
    _cookie: Option<&str>, // Marked as unused
) -> Result<CommonLiveStreamInfo, String> {
    let requested_id = room_id.trim().to_string();
    if requested_id.is_empty() {
        logger().warn("Empty room_id provided");
        return Ok(CommonLiveStreamInfo {
            title: None,
            anchor_name: None,
            avatar: None,
            stream_url: None,
            status: None,
            error_message: Some("Douyin web_id cannot be empty.".to_string()),
            upstream_url: None,
            available_streams: None,
            normalized_room_id: None,
            web_rid: None,
            raw: None,
        });
    }

    logger().info(format!(
        "Fetching stream for '{}' with requested quality '{}'",
        requested_id, quality
    ));

    let http_client = HttpClient::new_direct_connection()
        .map_err(|e| format!("Failed to create direct connection HttpClient: {}", e))?;

    let normalized_id = normalize_douyin_live_id(&requested_id);
    logger().debug(format!("Normalized room_id: '{}' -> '{}'", requested_id, normalized_id));
    
    let DouyinRoomData { room, raw_response } = fetch_room_data(&http_client, &normalized_id, None).await?;
    logger().log_structured(shared::logger::LogLevel::Debug, "抖音房间数据", &room);
    
    let web_rid = extract_web_rid(&room).unwrap_or_else(|| normalized_id.clone());
    logger().debug(format!("Extracted web_rid: {:?}", web_rid));
    
    let status = room
        .get("status")
        .and_then(|v| v.as_i64())
        .unwrap_or_default() as i32;
    logger().debug(format!("Extracted status: {}", status));
    
    let title = room
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    logger().debug(format!("Extracted title: {:?}", title));
    
    let anchor_name = extract_anchor_name(&room);
    logger().debug(format!("Extracted anchor_name: {:?}", anchor_name));
    
    let avatar = extract_avatar(&room);
    logger().debug(format!("Extracted avatar: {:?}", avatar));
    
    let available_streams = collect_available_streams(&room);
    let streams_count = available_streams.as_ref().map(|streams| streams.len()).unwrap_or(0);
    logger().debug(format!("Collected available_streams: {} items", streams_count));

    if status != 2 {
        logger().info(format!(
            "Room '{}' is not live (status={}). Returning metadata only.",
            web_rid, status
        ));
        return Ok(CommonLiveStreamInfo {
            title,
            anchor_name,
            avatar,
            stream_url: None,
            status: Some(status),
            error_message: None,
            upstream_url: None,
            available_streams: available_streams.clone(),
            normalized_room_id: None,
            web_rid: Some(web_rid),
            raw: raw_response.clone(),
        });
    }

    let target_quality = normalize_quality_tag(&quality);
    logger().debug(format!("Normalized quality: '{}' -> '{}'", quality, target_quality));
    
    let selected = choose_flv_stream(&room, target_quality)
        .or_else(|| first_flv_stream(&room))
        .ok_or_else(|| {
            "No FLV streams available in stream_url.flv_pull_url".to_string()
        })?;
    
    let (selected_key, real_url) = selected;
    logger().info(format!(
        "Selected FLV stream key='{}' url='{}'",
        selected_key, real_url
    ));

    let sanitized_url = enforce_https(&real_url);
    logger().debug(format!("Sanitized URL: '{}' -> '{}'", real_url, sanitized_url));

    let result = CommonLiveStreamInfo {
        title,
        anchor_name,
        avatar,
        stream_url: Some(sanitized_url.clone()),
        status: Some(status),
        error_message: None,
        upstream_url: Some(sanitized_url),
        available_streams,
        normalized_room_id: None,
        web_rid: Some(web_rid),
        raw: raw_response.clone(),
    };
    
    logger().log_structured(shared::logger::LogLevel::Debug, "抖音直播流信息", &result);
    Ok(result)
}

fn normalize_quality_tag(input: &str) -> &str {
    match input.trim().to_uppercase().as_str() {
        "OD" | "原画" => QUALITY_OD,
        "BD" | "高清" => QUALITY_BD,
        "UHD" | "标清" => QUALITY_UHD,
        _ => QUALITY_OD,
    }
}

pub(crate) fn extract_web_rid(room: &Value) -> Option<String> {
    room.get("owner")
        .and_then(|o| o.get("web_rid"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            room.get("anchor")
                .and_then(|a| a.get("web_rid"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .or_else(|| {
            room.get("web_rid")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
}

pub(crate) fn extract_anchor_name(room: &Value) -> Option<String> {
    room.get("anchor_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            room.get("owner")
                .and_then(|o| o.get("nickname"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .or_else(|| {
            room.get("anchor")
                .and_then(|a| a.get("nickname"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
}

fn enforce_https(url: &str) -> String {
    if url.starts_with("https://") {
        url.to_string()
    } else if url.starts_with("http://") {
        format!("https://{}", &url["http://".len()..])
    } else {
        url.to_string()
    }
}

pub fn extract_avatar(room: &Value) -> Option<String> {
    room.get("owner")
        .and_then(|o| o.get("avatar_thumb"))
        .and_then(|thumb| thumb.get("url_list"))
        .and_then(|list| list.get(0))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            room.get("anchor")
                .and_then(|a| a.get("avatar_thumb"))
                .and_then(|thumb| thumb.get("url_list"))
                .and_then(|list| list.get(0))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
}

pub(crate) fn collect_available_streams(room: &Value) -> Option<Vec<StreamVariant>> {
    let flv_map = room
        .get("stream_url")
        .and_then(|v| v.get("flv_pull_url"))
        .and_then(|v| v.as_object())?;
    let variants = flv_map
        .iter()
        .filter_map(|(k, v)| {
            v.as_str().map(|url| StreamVariant {
                url: url.to_string(),
                format: Some("flv".to_string()),
                desc: Some(k.to_string()),
                qn: None,
                protocol: url.split(':').next().map(|s| s.to_string()),
            })
        })
        .collect::<Vec<_>>();
    if variants.is_empty() {
        None
    } else {
        Some(variants)
    }
}

fn first_flv_stream(room: &Value) -> Option<(String, String)> {
    let flv_map = room
        .get("stream_url")
        .and_then(|v| v.get("flv_pull_url"))
        .and_then(|v| v.as_object())?;
    flv_map
        .iter()
        .find_map(|(k, v)| v.as_str().map(|url| (k.to_string(), url.to_string())))
}
