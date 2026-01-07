use shared::http_client::HttpClient;
use crate::a_bogus::generate_a_bogus;
use crate::message::signature::generate_douyin_ms_token;
use shared::interface::PlatformType;
use shared::logger::Logger;
use serde_json::Value;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT_ENCODING, COOKIE, REFERER, USER_AGENT};

// 创建静态日志记录器
static LOGGER: std::sync::OnceLock<Logger> = std::sync::OnceLock::new();

fn logger() -> &'static Logger {
    LOGGER.get_or_init(|| {
        Logger::new(Some(PlatformType::Douyin), "douyin::web_api")
    })
}

// Use the tested cookie from douyin_rust sample to improve API success.
pub const DEFAULT_COOKIE: &str =
    "ttwid=1%7C2iDIYVmjzMcpZ20fcaFde0VghXAA3NaNXE_SLR68IyE%7C1761045455%7Cab35197d5cfb21df6cbb2fa7ef1c9262206b062c315b9d04da746d0b37dfbc7d";
// Align UA with the working Douyin Rust sample to keep a_bogus inputs consistent.
pub const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.5845.97 Safari/537.36 Core/1.116.567.400 QQBrowser/19.7.6764.400";

#[derive(Debug, Clone)]
pub struct DouyinRoomData {
    pub room: Value,
    pub raw_response: Option<Value>,
}

// 直接从返回的 stream_data 中补全 ORIGIN，不依赖 HTML 解析，贴近 douyin_rust 实现。
fn merge_origin_stream(room: &mut Value) {
    let Some(stream_url) = room.get_mut("stream_url") else { return };
    let live_core_sdk_data = stream_url.get("live_core_sdk_data");
    if live_core_sdk_data.is_none() {
        return;
    }

    let pull_datas = stream_url.get("pull_datas").and_then(|v| v.as_object());
    let json_str = if let Some(pd) = pull_datas {
        if let Some((_, entry)) = pd.iter().next() {
            entry
                .get("stream_data")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string())
        } else {
            None
        }
    } else {
        live_core_sdk_data
            .and_then(|d| d.get("pull_data"))
            .and_then(|p| p.get("stream_data"))
            .and_then(|s| s.as_str())
            .map(|s| s.to_string())
    };
    let Some(json_str) = json_str else { return };

    let parsed: Value = serde_json::from_str(&json_str).unwrap_or(Value::Null);
    let origin_main = parsed
        .get("data")
        .and_then(|d| d.get("origin"))
        .and_then(|o| o.get("main"));
    let Some(origin_main) = origin_main else { return };

    let origin_codec = origin_main
        .get("sdk_params")
        .and_then(|s| s.as_str())
        .and_then(|s| serde_json::from_str::<Value>(s).ok())
        .and_then(|v| v.get("VCodec").cloned())
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default();
    let origin_hls = origin_main
        .get("hls")
        .and_then(|v| v.as_str())
        .map(|s| format!("{}&codec={}", s, origin_codec));
    let origin_flv = origin_main
        .get("flv")
        .and_then(|v| v.as_str())
        .map(|s| format!("{}&codec={}", s, origin_codec));

    if let Some(hls_origin) = origin_hls {
        match stream_url.get_mut("hls_pull_url_map") {
            Some(map_val) if map_val.is_object() => {
                if let Some(map) = map_val.as_object_mut() {
                    let existing = map.clone();
                    map.clear();
                    map.insert("ORIGIN".to_string(), Value::String(hls_origin.clone()));
                    map.extend(existing);
                }
            }
            _ => {
                let mut new_map = serde_json::Map::new();
                new_map.insert("ORIGIN".to_string(), Value::String(hls_origin));
                stream_url.as_object_mut().map(|obj| {
                    obj.insert("hls_pull_url_map".to_string(), Value::Object(new_map))
                });
            }
        }
    }

    if let Some(flv_origin) = origin_flv {
        match stream_url.get_mut("flv_pull_url") {
            Some(map_val) if map_val.is_object() => {
                if let Some(map) = map_val.as_object_mut() {
                    let existing = map.clone();
                    map.clear();
                    map.insert("ORIGIN".to_string(), Value::String(flv_origin.clone()));
                    map.extend(existing);
                }
            }
            _ => {
                let mut new_map = serde_json::Map::new();
                new_map.insert("ORIGIN".to_string(), Value::String(flv_origin));
                stream_url.as_object_mut().map(|obj| {
                    obj.insert("flv_pull_url".to_string(), Value::Object(new_map))
                });
            }
        }
    }
}

async fn fetch_room_from_api(
    http_client: &HttpClient,
    web_id: &str,
    cookies: Option<&str>,
) -> Result<DouyinRoomData, String> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(DEFAULT_USER_AGENT));
    headers.insert(REFERER, HeaderValue::from_str(&format!("https://live.douyin.com/{web_id}")).map_err(|e| format!("Invalid Referer: {e}"))?);
    headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("identity"));
    headers.insert(COOKIE, HeaderValue::from_str(cookies.unwrap_or(DEFAULT_COOKIE)).map_err(|e| format!("Invalid cookie header value: {}", e))?);

    // 生成抖音msToken
    let ms_token = generate_douyin_ms_token();
    let params = vec![
        ("aid", "6383"),
        ("app_name", "douyin_web"),
        ("live_id", "1"),
        ("device_platform", "web"),
        ("language", "zh-CN"),
        ("browser_language", "zh-CN"),
        ("browser_platform", "Win32"),
        ("browser_name", "Chrome"),
        ("browser_version", "116.0.0.0"),
        ("web_rid", web_id),
        ("msToken", &ms_token),
    ];
    let query = serde_urlencoded::to_string(&params)
        .map_err(|e| format!("Failed to encode Douyin enter params: {}", e))?;
    let sign = generate_a_bogus(&query, DEFAULT_USER_AGENT);
    let api = format!(
        "https://live.douyin.com/webcast/room/web/enter/?{}&a_bogus={}",
        query,
        sign
    );
    
    let params_value = serde_json::json!(params);
    logger().log_request("fetch_room_from_api", &api, &Some(params_value));
    logger().debug(format!("生成的a_bogus签名: {}", sign));
    
    // 重试机制：最多重试2次
    let max_retries = 2;
    for attempt in 1..=max_retries {
        logger().debug(format!("第 {}/{} 次尝试请求抖音API", attempt, max_retries));
        logger().debug(format!("请求头: {:?}", headers));
        
        // 使用直接的 http_client.inner 调用，与原始实现保持一致
        match http_client.inner
            .get(&api)
            .headers(headers.clone())
            .send()
            .await
        {
            Ok(resp) => {
                logger().debug(format!("第 {}/{} 次尝试: 收到抖音API响应，状态码: {}", attempt, max_retries, resp.status()));
                
                // 检查响应状态码
                if !resp.status().is_success() {
                    logger().error(format!("第 {}/{} 次尝试: 抖音API返回错误状态码: {}", attempt, max_retries, resp.status()));
                    
                    // 如果不是最后一次尝试，等待1秒后重试
                    if attempt < max_retries {
                        logger().warn(format!("第 {}/{} 次尝试失败，1秒后重试", attempt, max_retries));
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        continue;
                    }
                    return Err(format!("Failed to fetch Douyin room data after {} attempts: API returned error status code {}", max_retries, resp.status()));
                }
                
                // 先获取原始响应文本
                let text = resp.text().await.map_err(|e| format!("Failed to read response text: {}", e))?;
                
                // 尝试解析JSON
                match serde_json::from_str::<Value>(&text) {
                    Ok(json) => {
                        logger().debug(format!("第 {}/{} 次尝试: 成功解析抖音API响应JSON", attempt, max_retries));
                        logger().log_structured(shared::logger::LogLevel::Debug, "抖音API响应", &json);
                        
                        // 检查响应结构是否符合预期
                        let room = json
                            .get("data")
                            .and_then(|d| d.get("data"))
                            .and_then(|arr| arr.get(0))
                            .cloned()
                            .unwrap_or_else(|| {
                                // 如果data.data为空，创建一个默认的room对象
                                serde_json::json!({})
                            });
                            
                        // 从data.user获取主播信息
                        let user = json
                            .get("data")
                            .and_then(|d| d.get("user"))
                            .cloned();
                            
                        // 提取主播名称
                        let anchor_name = user
                            .as_ref()
                            .and_then(|u| u.get("nickname"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .or_else(|| {
                                // 备选方案：从原始json路径获取
                                json
                                    .get("data")
                                    .and_then(|d| d.get("user"))
                                    .and_then(|u| u.get("nickname"))
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                            });
                            
                        // 提取主播头像
                        let avatar_url = user
                            .as_ref()
                            .and_then(|u| u.get("avatar_thumb"))
                            .and_then(|thumb| thumb.get("url_list"))
                            .and_then(|list| list.get(0))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        
                        logger().debug(format!("解析到的直播间数据: {:?}", room));
                        logger().debug(format!("解析到的主播名称: {:?}", anchor_name));
                        logger().debug(format!("解析到的主播头像: {:?}", avatar_url));
                        
                        // 创建可修改的room对象
                        let mut room_mut = room;
                        
                        // 将主播信息添加到room对象中
                        if let Some(obj) = room_mut.as_object_mut() {
                            // 添加主播名称
                            if let Some(name) = anchor_name {
                                obj.insert("anchor_name".to_string(), Value::String(name));
                            }
                            
                            // 添加主播头像
                            if let Some(url) = avatar_url {
                                obj.insert("avatar_url".to_string(), Value::String(url));
                            }
                            
                            // 添加用户信息
                            if let Some(user_data) = user {
                                obj.insert("user_info".to_string(), user_data);
                            }
                        }
                        
                        // 合并流信息
                        merge_origin_stream(&mut room_mut);
                        
                        logger().debug(format!("处理后的直播间数据: {:?}", room_mut));
                        logger().log_response_with_raw("fetch_room_from_api", &room_mut, &Some(json.clone()));
                        
                        // 无论room是否存在，只要有主播信息就返回成功
                        return Ok(DouyinRoomData { room: room_mut, raw_response: Some(json) });
                    },
                    Err(e) => {
                        logger().error(format!("第 {}/{} 次尝试: 抖音API响应JSON解析失败: {} - 原始响应文本: {}", attempt, max_retries, e, text));
                        
                        // 如果不是最后一次尝试，等待1秒后重试
                        if attempt < max_retries {
                            logger().warn(format!("第 {}/{} 次尝试失败，1秒后重试", attempt, max_retries));
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        } else {
                            return Err(format!("Failed to fetch Douyin room data after {} attempts: JSON parse error: {} - Raw response: {}", max_retries, e, text));
                        }
                    }
                }
            },
            Err(e) => {
                logger().error(format!("第 {}/{} 次尝试: 抖音API网络请求失败: {}", attempt, max_retries, e));
                
                // 如果不是最后一次尝试，等待1秒后重试
                if attempt < max_retries {
                    logger().warn(format!("第 {}/{} 次尝试失败，1秒后重试", attempt, max_retries));
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                } else {
                    return Err(format!("Failed to fetch Douyin room data after {} attempts: Network error: {}", max_retries, e));
                }
            }
        }
    }
    
    // 所有尝试都失败了
    Err(format!("Failed to fetch Douyin room data after {} attempts: API response structure is invalid", max_retries))
}

/// Normalize user input into a Douyin web_id. Supports raw IDs and full URLs such as
/// `https://live.douyin.com/123456` or `https://www.douyin.com/follow/live/123456`.
pub fn normalize_douyin_live_id(id_or_url: &str) -> String {
    let trimmed = id_or_url.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // Prefer explicit room/query parameters if present.
    if let Some(qpos) = trimmed.find('?') {
        let query = &trimmed[qpos + 1..];
        for kv in query.split('&') {
            if let Some(val) = kv
                .strip_prefix("room_id=")
                .or_else(|| kv.strip_prefix("roomId="))
                .or_else(|| kv.strip_prefix("web_rid="))
                .or_else(|| kv.strip_prefix("webId="))
            {
                let cleaned = val
                    .split(['&', '#'])
                    .find(|s| !s.is_empty())
                    .unwrap_or(val);
                if !cleaned.is_empty() {
                    return cleaned.to_string();
                }
            }
        }
    }

    // Handle any douyin.com URL (live.douyin.com, www.douyin.com/follow/live/xxx, etc.).
    if let Some(pos) = trimmed.find("douyin.com/") {
        let start = pos + "douyin.com/".len();
        let remainder = &trimmed[start..];
        let path_only = remainder.split(['?', '#']).next().unwrap_or(remainder);
        if let Some(segment) = path_only
            .rsplit('/')
            .find(|segment| !segment.is_empty())
        {
            return segment
                .split(['?', '&', '#'])
                .find(|s| !s.is_empty())
                .unwrap_or(segment)
                .to_string();
        }
    }

    // Fallback: strip trailing query/hash from raw input.
    trimmed
        .split(['?', '&', '#'])
        .find(|s| !s.is_empty())
        .unwrap_or(trimmed)
        .to_string()
}

pub async fn fetch_room_data(
    http_client: &HttpClient,
    raw_id: &str,
    cookies: Option<&str>,
) -> Result<DouyinRoomData, String> {
    let web_id = normalize_douyin_live_id(raw_id);
    logger().debug(format!("归一化后的抖音直播ID: {}", web_id));
    // 简化逻辑：直接走网页版接口 + a_bogus，避免 HTML 解析失败。
    fetch_room_from_api(http_client, &web_id, cookies).await
}

pub fn choose_flv_stream(room: &Value, desired_quality: &str) -> Option<(String, String)> {
    let flv_map = room
        .get("stream_url")
        .and_then(|v| v.get("flv_pull_url"))
        .and_then(|v| v.as_object())?;

    const QUALITY_ORDER: [&str; 6] = ["OD", "BD", "UHD", "HD", "SD", "LD"];

    let mut entries: Vec<(String, String)> = flv_map
        .iter()
        .filter_map(|(key, value)| value.as_str().map(|url| (key.clone(), url.to_string())))
        .collect();

    if entries.is_empty() {
        return None;
    }

    while entries.len() < QUALITY_ORDER.len() {
        if let Some(last) = entries.last().cloned() {
            entries.push(last);
        } else {
            break;
        }
    }

    let desired = desired_quality.trim().to_uppercase();
    let idx = QUALITY_ORDER
        .iter()
        .position(|q| q.eq_ignore_ascii_case(&desired))
        .unwrap_or(0);

    entries
        .get(idx)
        .cloned()
        .or_else(|| entries.last().cloned())
}
