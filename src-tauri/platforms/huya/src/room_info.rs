use serde::Serialize;
use shared::http_client::HttpClient;
use shared::interface::{PlatformError, PlatformType};
use shared::logger::Logger;

// 创建静态日志记录器
static LOGGER: std::sync::OnceLock<Logger> = std::sync::OnceLock::new();

fn logger() -> &'static Logger {
    LOGGER.get_or_init(|| {
        Logger::new(Some(PlatformType::Huya), "huya::room_info")
    })
}

#[derive(Clone, Debug, Serialize)]
pub struct RoomDetail {
    pub status: bool,
    pub title: Option<String>,
    pub nick: Option<String>,
    pub avatar180: Option<String>,
}

pub async fn fetch_room_info(
    client: &HttpClient,
    room_id: &str
) -> Result<(RoomDetail, Option<serde_json::Value>), PlatformError> {
    let url = format!(
        "https://mp.huya.com/cache.php?m=Live&do=profileRoom&roomid={}&showSecret=1",
        room_id
    );
    
    let params = serde_json::json!({"roomid": room_id, "showSecret": 1});
    logger().log_request("fetch_room_info", &url, &Some(params));

    let text = client
        .get_text(&url)
        .await
        .map_err(|e| PlatformError::Network(e))?;
    
    logger().debug(format!("Huya API response text: {}", text));
    
    let v: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| PlatformError::Api(e.to_string()))?;
    
    let raw_data = Some(v.clone());
    logger().log_structured(shared::logger::LogLevel::Debug, "Huya房间数据", &v);

    let status_code = v.get("status").and_then(|x| x.as_i64()).unwrap_or(0);
    logger().debug(format!("Huya API status code: {}", status_code));
    
    if status_code != 200 {
        logger().info(format!("Huya API returned non-200 status: {}, returning empty room detail", status_code));
        return Ok((RoomDetail {
            status: false,
            title: None,
            nick: None,
            avatar180: None,
        }, raw_data));
    }

    let Some(data) = v.get("data") else {
        logger().warn("Huya API response missing 'data' field, returning empty room detail");
        return Ok((RoomDetail {
            status: false,
            title: None,
            nick: None,
            avatar180: None,
        }, raw_data));
    };

    let stream_ok = data.get("stream").is_some();
    logger().debug(format!("Huya room stream status: {}", stream_ok));

    let title = data
        .get("liveData")
        .and_then(|ld| ld.get("introduction"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    
    let nick = data
        .get("liveData")
        .and_then(|ld| ld.get("nick"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    
    let avatar180 = data
        .get("liveData")
        .and_then(|ld| ld.get("avatar180"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    
    logger().debug(format!("Extracted Huya room info: title={:?}, nick={:?}, avatar180={:?}", title, nick, avatar180));

    let result = RoomDetail {
        status: stream_ok,
        title,
        nick,
        avatar180,
    };
    
    logger().log_response_with_raw("fetch_room_info", &result, &raw_data);
    
    Ok((result, raw_data))
}