use shared::http_client::HttpClient;
use shared::LiveStreamInfo;
use crate::web_api::{fetch_room_data, normalize_douyin_live_id, DouyinRoomData};
use shared::interface::PlatformType;
use shared::logger::Logger;

// 创建静态日志记录器
static LOGGER: std::sync::OnceLock<Logger> = std::sync::OnceLock::new();

fn logger() -> &'static Logger {
    LOGGER.get_or_init(|| {
        Logger::new(Some(PlatformType::Douyin), "douyin::room_info")
    })
}

pub async fn fetch_room_info(
    room_id: &str,
    http_client: &HttpClient,
) -> Result<LiveStreamInfo, String> {
    logger().info(format!("fetch_room_info called: room_id={}", room_id));
    
    let requested_id = room_id.trim().to_string();
    logger().debug(format!("Trimmed room_id: '{}' -> '{}'", room_id, requested_id));
    
    if requested_id.is_empty() {
        logger().warn("Empty room_id provided");
        return Ok(LiveStreamInfo {
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

    let normalized_id = normalize_douyin_live_id(&requested_id);
    logger().debug(format!("Normalized room_id: '{}' -> '{}'", requested_id, normalized_id));

    logger().info(format!("Fetching room data for normalized_id: {}", normalized_id));
    match fetch_room_data(http_client, &normalized_id, None).await {
        Ok(DouyinRoomData { room, raw_response }) => {
            logger().debug(format!("Successfully fetched room data for {}: {:?}", normalized_id, room));
            logger().log_structured(shared::logger::LogLevel::Debug, "抖音房间数据", &room);
            
            let web_rid = super::stream_url::extract_web_rid(&room)
                .unwrap_or_else(|| normalized_id.clone());
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
            
            let anchor_name = super::stream_url::extract_anchor_name(&room);
            logger().debug(format!("Extracted anchor_name: {:?}", anchor_name));
            
            let avatar = super::stream_url::extract_avatar(&room);
            logger().debug(format!("Extracted avatar: {:?}", avatar));
            
            let available_streams = super::stream_url::collect_available_streams(&room);
            let streams_count = available_streams.as_ref().map(|streams| streams.len()).unwrap_or(0);
            logger().debug(format!("Collected available_streams: {} items", streams_count));
            
            logger().info(format!("Successfully processed room info for {}: title={:?}, anchor={:?}, status={}", 
                  normalized_id, title, anchor_name, status));
            
            let result = LiveStreamInfo {
                title,
                anchor_name,
                avatar,
                stream_url: None,
                status: Some(status),
                error_message: None,
                upstream_url: None,
                available_streams,
                normalized_room_id: None,
                web_rid: Some(web_rid),
                raw: raw_response.clone(),
            };
            
            Ok(result)
        }
        Err(e) => {
            logger().error(format!("Failed to fetch room data for {}: {}", normalized_id, e));
            Ok(LiveStreamInfo {
                title: None,
                anchor_name: None,
                avatar: None,
                stream_url: None,
                status: None,
                error_message: Some(format!("获取抖音房间信息失败: {}", e)),
                upstream_url: None,
                available_streams: None,
                normalized_room_id: None,
                web_rid: Some(normalized_id),
                raw: None,
            })
        },
    }
}