use shared::http_client::HttpClient;
use shared::interface::PlatformType;
use shared::logger::Logger;
use crate::web_api::{
    fetch_room_data, normalize_douyin_live_id, DouyinRoomData, DEFAULT_USER_AGENT, DEFAULT_COOKIE,
};
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

// 创建静态日志记录器
static LOGGER: std::sync::OnceLock<Logger> = std::sync::OnceLock::new();

fn logger() -> &'static Logger {
    LOGGER.get_or_init(|| {
        Logger::new(Some(PlatformType::Douyin), "douyin::message::web_fetcher")
    })
}

use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, REFERER, USER_AGENT};

// New struct for frontend
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DouyinFollowListRoomInfo {
    pub web_rid: String,
    pub nickname: String,
    pub room_name: String, // Title of the room
    pub avatar_url: String,
    pub status: i32, // 0 for live, other values indicate not live or error
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ResolvedRoomInfo {
    pub web_rid: Option<String>,
    pub room_id: String,
    pub nickname: String,
    pub room_name: String,
    pub avatar_url: String,
    pub status: i32,
}

pub struct DouyinLiveWebFetcher {
    pub live_id: String,
    pub room_id: Option<String>,
    resolved_info: Option<ResolvedRoomInfo>,
    pub user_agent: String,
    pub http_client: HttpClient,
    pub(crate) _ws_stream: Option<Arc<Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
    // 新增字段：用于 WebSocket 和签名所需
    pub dy_cookie: Option<String>,
    pub user_unique_id: Option<String>,
}

impl DouyinLiveWebFetcher {
    pub fn new(live_id: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // 使用直连HTTP客户端，绕过所有代理设置
        let http_client = HttpClient::new_direct_connection()
            .map_err(|e| format!("Failed to create direct connection HttpClient: {}", e))?;
        let normalized_live_id = normalize_douyin_live_id(live_id);

        Ok(DouyinLiveWebFetcher {
            live_id: normalized_live_id,
            room_id: None,
            resolved_info: None,
            user_agent: DEFAULT_USER_AGENT.to_string(),
            http_client,
            _ws_stream: None,
            dy_cookie: None,
            user_unique_id: None,
        })
    }

    async fn resolve_room_info(
        &mut self,
    ) -> Result<ResolvedRoomInfo, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(info) = &self.resolved_info {
            return Ok(info.clone());
        }

        let live_id = self.live_id.clone();
        let cookies = self.dy_cookie.as_deref();
        // 直接使用和 douyin_rust 相同的接口 + a_bogus，避免 HTML 解析失败。
        match fetch_room_data(&self.http_client, &live_id, cookies).await {
            Ok(DouyinRoomData { room, .. }) => {
                let room_id = room
                    .get("id_str")
                    .and_then(|v| v.as_str())
                    .or_else(|| room.get("id").and_then(|v| v.as_str()))
                    .unwrap_or(&live_id)
                    .to_string();
                let status = room.get("status").and_then(|v| v.as_i64()).unwrap_or(-1) as i32;
                let room_name = room
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let owner = room.get("owner").cloned().unwrap_or_else(|| Value::Null);
                let anchor = room.get("anchor").cloned().unwrap_or_else(|| Value::Null);
                let nickname = owner
                    .get("nickname")
                    .and_then(|v| v.as_str())
                    .or_else(|| anchor.get("nickname").and_then(|v| v.as_str()))
                    .unwrap_or("")
                    .to_string();
                let avatar_url = owner
                    .get("avatar_thumb")
                    .or_else(|| anchor.get("avatar_thumb"))
                    .and_then(|a| a.get("url_list"))
                    .and_then(|ul| ul.get(0))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let web_rid_val = owner
                    .get("web_rid")
                    .and_then(|v| v.as_str())
                    .or_else(|| anchor.get("web_rid").and_then(|v| v.as_str()))
                    .unwrap_or(&live_id)
                    .to_string();

                let info = ResolvedRoomInfo {
                    web_rid: Some(web_rid_val),
                    room_id: room_id.clone(),
                    nickname,
                    room_name,
                    avatar_url,
                    status,
                };
                self.room_id = Some(room_id);
                self.resolved_info = Some(info.clone());
                Ok(info)
            }
            Err(api_err) => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to resolve via web enter API: {}", api_err),
            ))),
        }
    }

    pub async fn collect_cookies_and_ids(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Ensure room_id has been resolved before collecting cookies
        self.resolve_room_info().await?;

        let homepage_url = "https://live.douyin.com/";
        logger().info("开始收集 Douyin Cookie 和用户 ID");
        
        // 设置 HttpClient 的 User-Agent
        if let Err(e) = self.http_client.insert_header(USER_AGENT, &self.user_agent) {
            logger().error(format!("【X】设置 User-Agent 失败: {}", e));
            return Err(e.into());
        }

        // 准备请求头
        let mut headers = HeaderMap::new();
        headers.insert(REFERER, HeaderValue::from_static("https://live.douyin.com"));
        headers.insert(ACCEPT, HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"));
        headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("zh-CN,zh;q=0.9,en;q=0.8"));

        // 先通过 GET 请求收集 Cookie（使用 HttpClient 包装器）
        let mut dy_cookie = String::new();
        
        // 发送 GET 请求获取初始 Cookie
        logger().debug("发送 GET 请求获取初始 Cookie");
        let get_response = self.http_client.get_with_cookies(homepage_url).await?;
        
        // 收集 Cookie
        self.extract_and_update_cookies(&get_response, &mut dy_cookie);
        
        // 再发送一次 GET 请求到直播间页面，补全 Cookie
        let live_url = format!("https://live.douyin.com/{}", self.live_id);
        logger().debug(format!("发送 GET 请求到直播间页面: {}", live_url));
        let live_response = self.http_client.get_with_cookies(&live_url).await?;
        
        // 收集更多 Cookie
        self.extract_and_update_cookies(&live_response, &mut dy_cookie);
        
        // 验证并处理收集到的 Cookie
        self.validate_and_process_cookies(&mut dy_cookie)?;
        
        // 从 Cookie 中提取 user_unique_id（优先使用 s_v_web_id），失败则回退到 ttwid，最后生成一个临时值
        let user_unique_id = self.extract_user_unique_id(&dy_cookie);
        logger().info(format!("✅ 成功提取用户唯一ID: {}", user_unique_id));
        
        // 设置收集到的 Cookie 和 user_unique_id
        self.dy_cookie = Some(dy_cookie);
        self.user_unique_id = Some(user_unique_id);
        
        logger().info("✅ Cookie 和用户 ID 收集完成");
        Ok(())
    }
    
    /// 从响应中提取并更新 Cookie
    fn extract_and_update_cookies(&self, response: &reqwest::Response, cookie_str: &mut String) {
        for val in response.headers().get_all("set-cookie").iter() {
            if let Ok(s) = val.to_str() {
                let first = s.split(';').next().unwrap_or("");
                if first.contains("ttwid")
                    || first.contains("__ac_nonce")
                    || first.contains("msToken")
                    || first.contains("s_v_web_id")
                    || first.contains("tt_scid")
                    || first.contains("__ac_signature")
                {
                    if !cookie_str.contains(first) {
                        cookie_str.push_str(first);
                        cookie_str.push(';');
                        logger().debug(format!("📦 收集到 Cookie: {}", first));
                    }
                }
            }
        }
    }
    
    /// 验证并处理收集到的 Cookie
    fn validate_and_process_cookies(&self, dy_cookie: &mut String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 确保 Cookie 字符串不为空
        if dy_cookie.is_empty() {
            logger().warn("⚠️  未收集到任何 Cookie，使用默认 Cookie");
            // 使用默认 Cookie 作为后备
            *dy_cookie = DEFAULT_COOKIE.to_string();
        } else {
            // 检查是否包含关键 Cookie
            let has_ttwid = dy_cookie.contains("ttwid");
            let has_s_v_web_id = dy_cookie.contains("s_v_web_id");
            
            if !has_ttwid {
                logger().warn("⚠️  未收集到 ttwid Cookie，添加默认 ttwid");
                // 添加默认 ttwid
                let default_ttwid = DEFAULT_COOKIE
                    .split(';')
                    .find(|s| s.starts_with("ttwid="))
                    .map(|s| &s[6..]) // 跳过 "ttwid=" 前缀
                    .unwrap_or("");
                dy_cookie.push_str(&format!("ttwid={};", default_ttwid));
            }
            
            logger().info(format!("📋 Cookie 验证结果: ttwid={}, s_v_web_id={}", has_ttwid, has_s_v_web_id));
        }
        
        Ok(())
    }
    
    /// 从 Cookie 中提取用户唯一 ID
    fn extract_user_unique_id(&self, dy_cookie: &str) -> String {
        // 优先使用 s_v_web_id
        for kv in dy_cookie.split(';') {
            let kv = kv.trim();
            if let Some(v) = kv.strip_prefix("s_v_web_id=") {
                return v.to_string();
            }
        }
        
        // 其次使用 ttwid
        for kv in dy_cookie.split(';') {
            let kv = kv.trim();
            if let Some(v) = kv.strip_prefix("ttwid=") {
                return v.to_string();
            }
        }
        
        // 最后生成一个基于当前时间戳的临时 ID
        logger().warn("⚠️  未能从 Cookie 中提取用户唯一 ID，生成临时 ID");
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        format!("{}", millis)
    }

    pub async fn fetch_room_details(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 仅收集 Cookie 与识别 user_unique_id，不再解析房间 HTML。
        self.collect_cookies_and_ids().await?;
        Ok(())
    }

    pub async fn get_room_id(
        &mut self,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(room_id) = &self.room_id {
            return Ok(room_id.clone());
        }
        // Ensure identifiers resolved
        self.resolve_room_info().await?;
        self.room_id
            .clone()
            .ok_or_else(|| "room_id not set after cookie collection".into())
    }

    pub async fn get_user_unique_id(
        &mut self,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(uid) = &self.user_unique_id {
            return Ok(uid.clone());
        }
        self.collect_cookies_and_ids().await?;
        self.user_unique_id
            .clone()
            .ok_or_else(|| "user_unique_id not set after cookie collection".into())
    }

    pub async fn get_dy_cookie(
        &mut self,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(cookie) = &self.dy_cookie {
            return Ok(cookie.clone());
        }
        self.collect_cookies_and_ids().await?;
        self.dy_cookie
            .clone()
            .ok_or_else(|| "cookie not set after cookie collection".into())
    }

    #[allow(dead_code)]
    pub async fn get_room_status(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Ensure we have room_id and cookies collected
        let room_id_val = self.get_room_id().await?;
        let dy_cookie = self.get_dy_cookie().await?;
        let user_unique_id = self.get_user_unique_id().await?;

        // Parse msToken from collected cookie string (format: "key=value; key2=value2; ...")
        let ms_token = dy_cookie
            .split(';')
            .filter_map(|kv| {
                let kv = kv.trim();
                if kv.starts_with("msToken=") {
                    Some(kv.trim_start_matches("msToken=").to_string())
                } else {
                    None
                }
            })
            .next()
            .unwrap_or_default();

        // Build minimal and consistent URL using new scheme
        let base_url = "https://live.douyin.com/webcast/room/web/enter/?aid=6383&app_name=douyin_web&live_id=1&device_platform=web&language=zh-CN&cookie_enabled=true";
        let url = if ms_token.is_empty() {
            // Fallback: omit msToken if not present (server may read from Cookie)
            format!(
                "{}&room_id={}&user_unique_id={}",
                base_url, room_id_val, user_unique_id
            )
        } else {
            format!(
                "{}&room_id={}&msToken={}&user_unique_id={}",
                base_url, room_id_val, ms_token, user_unique_id
            )
        };

        // Prepare per-request headers: Accept, Accept-Language, Referer, User-Agent, Cookie
        // Update default User-Agent to the one stored in fetcher to avoid override by HttpClient::send_request
        if let Err(e) = self.http_client.insert_header(USER_AGENT, &self.user_agent) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to set USER_AGENT header: {}", e),
            )
            .into());
        }

        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json, text/plain, */*"));
        headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("zh-CN,zh;q=0.9"));
        headers.insert(
            REFERER,
            HeaderValue::from_str(&format!("https://live.douyin.com/{}", self.live_id))
                .unwrap_or_else(|_| HeaderValue::from_static("https://live.douyin.com")),
        );
        headers.insert(
            reqwest::header::HeaderName::from_static("cookie"),
            HeaderValue::from_str(&dy_cookie).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Invalid Cookie header: {}", e),
                )
            })?,
        );

        let data: serde_json::Value = match self
            .http_client
            .get_json_with_headers(&url, Some(headers))
            .await
        {
            Ok(v) => v,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to get room status: {}", e),
                )
                .into())
            }
        };

        // Parse and log basic room and owner info
        if let Some(room_data_top) = data.get("data") {
            if let Some(room_info) = room_data_top.get("room") {
                let room_status_val = room_info.get("status").and_then(|s| s.as_i64());
                if let Some(user_data) = room_info.get("owner") {
                    let user_id = user_data.get("id_str").and_then(|s| s.as_str());
                    let nickname = user_data.get("nickname").and_then(|s| s.as_str());

                    if let (Some(status), Some(id), Some(nick)) = (room_status_val, user_id, nickname) {
                        let status_text = if status == 0 {
                            "正在直播"
                        } else {
                            "已结束"
                        };
                        logger().info(format!("【{}】[{}]直播间：{}.", nick, id, status_text));
                    } else {
                        logger().error("【X】无法解析直播间信息的部分字段 (status, id, nick)");
                    }
                } else {
                    logger().error("【X】未找到用户信息 (owner data in room_data.room)");
                }
            } else {
                logger().error("【X】未找到房间信息 (room object in room_data_top)");
            }
        } else {
            logger().error("【X】未找到顶层房间数据 (data object in response)");
        }
        Ok(())
    }

    // fetch_room_details moved earlier to only collect cookies and HTML IDs; old implementation removed.

    // pub async fn connect_websocket_placeholder(&mut self, _room_id_param: &str, _ttwid_param: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    //     println!("Connect_websocket logic will be moved elsewhere.");
    //     Ok(())
    // }
}

// New Tauri command
pub async fn fetch_douyin_room_info(live_id: String) -> Result<DouyinFollowListRoomInfo, String> {
    logger().info(format!(
        "Fetching details for web_id: {}",
        live_id
    ));
    let normalized_id = normalize_douyin_live_id(&live_id);

    let http_client = HttpClient::new_direct_connection()
        .map_err(|e| format!("Failed to create direct connection HttpClient: {}", e))?;

    let DouyinRoomData { room, .. } = fetch_room_data(&http_client, &normalized_id, None)
        .await
        .map_err(|e| format!("Failed to fetch Douyin room data: {}", e))?;

    let web_rid = crate::stream_url::extract_web_rid(&room)
        .unwrap_or_else(|| normalized_id.clone());
    let nickname = crate::stream_url::extract_anchor_name(&room)
        .unwrap_or_else(|| format!("主播{}", web_rid));
    let room_name = room
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let avatar_url =
        crate::stream_url::extract_avatar(&room).unwrap_or_default();
    let status = room
        .get("status")
        .and_then(|v| v.as_i64())
        .unwrap_or_default() as i32;

    Ok(DouyinFollowListRoomInfo {
        web_rid,
        nickname,
        room_name,
        avatar_url,
        status,
    })
}
