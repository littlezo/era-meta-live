use md5;
use reqwest::header::{HeaderMap, HeaderValue, COOKIE, REFERER, USER_AGENT};
use serde_json::Value;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use shared::interface::PlatformType;
use shared::logger::Logger;

// 创建静态日志记录器
static LOGGER: std::sync::OnceLock<Logger> = std::sync::OnceLock::new();

fn logger() -> &'static Logger {
    LOGGER.get_or_init(|| {
        Logger::new(Some(PlatformType::Bilibili), "bilibili::room_info")
    })
}

// WBI mixin key mapping table (same as Python implementation)
const MIXIN_KEY_ENC_TAB: [usize; 64] = [
    46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35, 27, 43, 5, 49, 33, 9, 42, 19, 29,
    28, 14, 39, 12, 38, 41, 13, 37, 48, 7, 16, 24, 55, 40, 61, 26, 17, 0, 1, 60, 51, 30, 4, 22, 25,
    54, 21, 56, 59, 6, 63, 57, 62, 11, 36, 20, 34, 44, 52,
];

fn get_mixin_key(origin: &str) -> String {
    let mut out = String::new();
    for &idx in MIXIN_KEY_ENC_TAB.iter() {
        if let Some(ch) = origin.chars().nth(idx) {
            out.push(ch);
        }
    }
    out.chars().take(32).collect()
}

pub async fn get_wbi_keys(
    client: &shared::http_client::HttpClient,
    headers: &HeaderMap,
) -> Result<(String, String), String> {
    let url = "https://api.bilibili.com/x/web-interface/nav";
    let text = client
        .get_text_with_headers(url, Some(headers.clone()))
        .await
        .map_err(|e| format!("Failed to get WBI keys: {}", e))?;
    let json: Value = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse WBI keys JSON: {} | {}", e, text))?;
    let wbi_img = json["data"]["wbi_img"].clone();
    let img_url = wbi_img["img_url"].as_str().unwrap_or("");
    let sub_url = wbi_img["sub_url"].as_str().unwrap_or("");

    let img_key = if let Some(pos) = img_url.rfind('/') {
        let fname = &img_url[pos + 1..];
        fname.split('.').next().unwrap_or("").to_string()
    } else {
        String::new()
    };
    let sub_key = if let Some(pos) = sub_url.rfind('/') {
        let fname = &sub_url[pos + 1..];
        fname.split('.').next().unwrap_or("").to_string()
    } else {
        String::new()
    };

    if img_key.is_empty() || sub_key.is_empty() {
        return Err("WBI keys not found".to_string());
    }
    Ok((img_key, sub_key))
}

fn sanitize_value(value: &str) -> String {
    // Remove characters in the banned set: !'()*
    let banned: [char; 5] = ['!', '\'', '(', ')', '*'];
    value.chars().filter(|c| !banned.contains(c)).collect()
}

pub fn build_wbi_sign(room_id: &str, img_key: &str, sub_key: &str) -> (String, String) {
    let mixin_key = get_mixin_key(&format!("{}{}", img_key, sub_key));
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let wts = now.to_string();

    // Build parameter map with room_id and wts
    let mut qp: HashMap<String, String> = HashMap::new();
    qp.insert("room_id".to_string(), room_id.to_string());
    qp.insert("wts".to_string(), wts.clone());

    // Sort keys and sanitize values
    let mut keys: Vec<String> = qp.keys().cloned().collect();
    keys.sort();
    let mut query_parts: Vec<String> = Vec::new();
    for k in keys.iter() {
        let v = sanitize_value(qp.get(k).map(|s| s.as_str()).unwrap_or(""));
        query_parts.push(format!("{}={}", k, urlencoding::encode(&v)));
    }
    let query = query_parts.join("&");
    let sign_string = format!("{}{}", query, mixin_key);
    let hash = md5::compute(sign_string.as_bytes());
    let w_rid = format!("{:x}", hash);
    (wts, w_rid)
}

pub async fn fetch_room_info(
    client: &shared::http_client::HttpClient,
    room_id: &str,
    cookie: Option<&str>,
) -> Result<(shared::LiveStreamInfo, Option<Value>), String> {
    logger().info(format!("fetch_room_info called: room_id={}", room_id));
    
    let requested_id = room_id.trim().to_string();
    if requested_id.is_empty() {
        logger().warn("Empty room_id provided");
        return Ok((shared::LiveStreamInfo {
            title: None,
            anchor_name: None,
            avatar: None,
            stream_url: None,
            status: None,
            error_message: Some("房间ID未提供".to_string()),
            upstream_url: None,
            available_streams: None,
            normalized_room_id: None,
            web_rid: None,
            raw: None,
        }, None));
    }

    let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36";

    // Build headers (include optional cookie)
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_str(ua).unwrap());
    headers.insert(
        REFERER,
        HeaderValue::from_static("https://live.bilibili.com/"),
    );
    if let Some(c) = cookie {
        if !c.is_empty() {
            headers.insert(
                COOKIE,
                HeaderValue::from_str(c).unwrap_or(HeaderValue::from_static("")),
            );
        }
    }
    logger().debug("Built headers for Bilibili API request");

    // Get WBI keys and build sign
    logger().debug("Getting WBI keys for Bilibili API");
    let (img_key, sub_key) = get_wbi_keys(client, &headers).await?;
    logger().debug(format!("Got WBI keys: img_key={}, sub_key={}", img_key, sub_key));
    
    let (wts, w_rid) = build_wbi_sign(&room_id, &img_key, &sub_key);
    logger().debug(format!("Built WBI sign: wts={}, w_rid={}", wts, w_rid));

    // Call getInfoByRoom API with signed params
    let base = "https://api.live.bilibili.com/xlive/web-room/v1/index/getInfoByRoom";
    let params = vec![("room_id", room_id), ("wts", &wts), ("w_rid", &w_rid)];
    
    // Build query string manually
    let query_string = params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");
    let url = format!("{}?{}", base, query_string);
    
    let params_value = serde_json::json!(params);
    logger().log_request("fetch_room_info", &url, &Some(params_value));

    let text = client
        .get_text_with_headers(&url, Some(headers.clone()))
        .await
        .map_err(|e| format!("Room info request failed: {}", e))?;
    
    logger().debug(format!("Bilibili API response text: {}", text));
    
    let status = reqwest::StatusCode::OK; // HttpClient always returns 200 if successful, otherwise returns error
    if !status.is_success() {
        logger().error(format!("Bilibili API returned error status: {}", status));
        return Ok((shared::LiveStreamInfo {
            title: None,
            anchor_name: None,
            avatar: None,
            stream_url: None,
            status: None,
            error_message: Some(format!("Room info status: {} body: {}", status, text)),
            upstream_url: None,
            available_streams: None,
            normalized_room_id: None,
            web_rid: None,
            raw: serde_json::from_str(&text).ok(),
        }, None));
    }
    
    let j: Value = serde_json::from_str(&text)
        .map_err(|e| format!("Room info JSON parse failed: {} | {}", e, text))?;
    
    logger().log_structured(shared::logger::LogLevel::Debug, "Bilibili room info API response", &j);
    
    let data = j["data"].clone();

    let base_info = data["anchor_info"]["base_info"].clone();
    let room_info = data["room_info"].clone();

    let title = room_info["title"].as_str().map(|s| s.to_string());
    let anchor_name = base_info["uname"].as_str().map(|s| s.to_string());
    let avatar = base_info["face"].as_str().map(|s| s.to_string());
    let live_status = room_info["live_status"].as_i64().unwrap_or(0) as i32;
    
    logger().debug(format!("Extracted room info: title={:?}, anchor_name={:?}, live_status={}", title, anchor_name, live_status));

    let result = shared::LiveStreamInfo {
        title,
        anchor_name,
        avatar,
        stream_url: None,
        status: Some(live_status),
        error_message: None,
        upstream_url: None,
        available_streams: None,
        normalized_room_id: None,
        web_rid: None,
        raw: Some(j.clone()),
    };
    
    logger().log_response_with_raw("fetch_room_info", &result, &Some(j.clone()));
    
    Ok((result, Some(j.clone())))
}