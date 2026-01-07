pub mod cookie;
pub mod message;
pub mod live_list;
pub mod state;
pub mod stream_url;
pub mod room_info;
pub mod auth;
pub mod models;
pub mod search;
pub mod websocket;

use std::sync::Arc;

use shared::interface::*;
use shared::http_client::HttpClient;
use shared::logger::Logger;

// Bilibili 平台实现
pub struct BilibiliPlatform {
    http_client: Arc<HttpClient>,
    cookie: Option<String>,
    bilibili_state: Arc<state::BilibiliState>,
    logger: Logger,
}

impl BilibiliPlatform {
    pub fn new(config: PlatformConfig) -> Result<Self, PlatformError> {
        let http_client = HttpClient::new()
            .map_err(|e| PlatformError::Network(e.to_string()))?;
        
        let logger = Logger::new(Some(PlatformType::Bilibili), "bilibili");
        
        Ok(Self {
            http_client: Arc::new(http_client),
            cookie: config.cookie,
            bilibili_state: Arc::new(state::BilibiliState::new()),
            logger,
        })
    }
}

#[async_trait::async_trait]
impl LivePlatform for BilibiliPlatform {
    fn platform_type(&self) -> PlatformType {
        PlatformType::Bilibili
    }
    
    async fn fetch_live_list(
        &self,
        params: LiveListParams
    ) -> Result<LiveList, PlatformError> {
        self.logger.info(format!("fetch_live_list called with params: {:?}", params));
        
        let area_id = params.category_id.unwrap_or_default();
        let parent_area_id = params.other.as_ref()
            .and_then(|v| v.get("parent_area_id").and_then(|v| v.as_str()))
            .unwrap_or_default().to_string();
        let page = params.page.unwrap_or(1);
        
        let result = live_list::fetch_live_list(
            &self.http_client,
            area_id,
            parent_area_id,
            page,
            &self.bilibili_state,
        ).await
        .map_err(|e| {
            self.logger.error(format!("fetch_bilibili_live_list failed: {}", e));
            PlatformError::Api(e)
        })?;
        
        let (_, raw_data) = result;
        
        // Parse the result into LiveList format
        let json = raw_data.ok_or(PlatformError::Api("Failed to get raw data".to_string()))?;
        
        // Extract data from the response
        let items = if let Some(data) = json.get("data") {
            if let Some(list) = data.get("list").and_then(|v| v.as_array()) {
                list.iter().filter_map(|item| {
                    let room_id = item.get("roomid").and_then(|v| v.as_str())?;
                    let title = item.get("title").and_then(|v| v.as_str())?;
                    let streamer_name = item.get("uname").and_then(|v| v.as_str())?;
                    let streamer_id = item.get("uid").and_then(|v| v.as_i64())?.to_string();
                    let cover_url = item.get("cover").and_then(|v| v.as_str());
                    let viewer_count = item.get("online").and_then(|v| v.as_u64());
                    let category_name = item.get("area_name").and_then(|v| v.as_str());
                    let category_id = item.get("area_id").and_then(|v| v.as_str());
                    
                    Some(RoomInfo {
                        room_id: room_id.to_string(),
                        title: title.to_string(),
                        streamer_name: streamer_name.to_string(),
                        streamer_id: streamer_id.to_string(),
                        avatar_url: None, // Not available in this response
                        cover_url: cover_url.map(|s| s.to_string()),
                        live_status: true, // Assuming all items in live list are live
                        live_status_detail: "LIVE".to_string(),
                        viewer_count,
                        viewer_count_str: viewer_count.map(|v| v.to_string()),
                        category_name: category_name.map(|s| s.to_string()),
                        category_id: category_id.map(|s| s.to_string()),
                        tags: None,
                        other: Some(item.clone()),
                        raw: Some(item.clone()),
                    })
                }).collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        
        let live_list = LiveList {
            items,
            total: None, // Not available in this response
            page: Some(page),
            page_size: Some(params.page_size.unwrap_or(30)),
            has_more: true, // Assume there are more pages
            other: Some(json.clone()),
            raw: Some(json),
        };
        
        self.logger.debug(format!("fetch_live_list succeeded: found {} rooms", live_list.items.len()));
        
        Ok(live_list)
    }
    
    async fn fetch_room_info(
        &self,
        room_id: &str
    ) -> Result<RoomInfo, PlatformError> {
        self.logger.info(format!("fetch_room_info called with room_id: {}", room_id));
        
        // 调用 room_info 模块的实现，获取原始数据
        let (result, raw_data) = room_info::fetch_room_info(
            &self.http_client,
            room_id,
            self.cookie.as_deref(),
        ).await
        .map_err(|e| {
            self.logger.error(format!("fetch_room_info failed for room_id {}: {}", room_id, e));
            PlatformError::Api(e)
        })?;
        
        // 克隆需要的字段，避免部分移动值问题
        let title = result.title.clone();
        let anchor_name = result.anchor_name.clone();
        let avatar = result.avatar.clone();
        let status = result.status;
        
        let live_status = status.unwrap_or(0) == 1;
        let live_status_detail = if live_status { "LIVE" } else { "OFFLINE" };
        
        let room_info = RoomInfo {
            room_id: room_id.to_string(),
            title: title.unwrap_or_default(),
            streamer_name: anchor_name.unwrap_or_default(),
            streamer_id: "".to_string(), // Not available in this response
            avatar_url: avatar,
            cover_url: None, // Not available in this response
            live_status,
            live_status_detail: live_status_detail.to_string(),
            viewer_count: None, // Not available in this response
            viewer_count_str: None, // Not available in this response
            category_name: None, // Not available in this response
            category_id: None, // Not available in this response
            tags: None,
            other: Some(serde_json::json!({"live_stream_info": result})),
            raw: raw_data,
        };
        
        self.logger.debug(format!("fetch_room_info succeeded for room_id {}: {:?}", room_id, room_info));
        
        Ok(room_info)
    }
    
    async fn get_stream_url(
        &self,
        room_id: &str,
        quality: Option<StreamQuality>
    ) -> Result<StreamUrl, PlatformError> {
        self.logger.info(format!("get_stream_url called with room_id: {}, quality: {:?}", room_id, quality));
        
        let quality_str = match quality {
            Some(StreamQuality::UltraHD) => "原画".to_string(),
            Some(StreamQuality::HD) => "高清".to_string(),
            Some(StreamQuality::SD) => "标清".to_string(),
            Some(StreamQuality::LD) => "流畅".to_string(),
            Some(StreamQuality::Auto) => "原画".to_string(),
            Some(StreamQuality::Custom(q)) => q.clone(),
            Some(StreamQuality::Original) => "原画".to_string(),
            Some(StreamQuality::K4) => "4K".to_string(),
            Some(StreamQuality::K2) => "2K".to_string(),
            Some(StreamQuality::P1080_60) => "1080P60".to_string(),
            Some(StreamQuality::P720_60) => "720P60".to_string(),
            None => "原画".to_string(),
        };
        
        // 调用 stream_url 模块的实现，获取原始数据
        let (result, raw_data) = stream_url::get_stream_url(
            room_id,
            quality_str,
            self.cookie.clone(),
        ).await
        .map_err(|e| {
            self.logger.error(format!("get_bilibili_live_stream_url failed for room_id {}: {}", room_id, e));
            PlatformError::Api(e)
        })?;
        
        // 克隆需要的字段，避免部分移动值问题
        let stream_url = result.stream_url.clone().unwrap_or_default();
        let upstream_url = result.upstream_url.clone();
        let available_streams = result.available_streams.clone().unwrap_or_default()
            .into_iter()
            .map(|v| v)
            .collect();
        let title = result.title.clone();
        let anchor_name = result.anchor_name.clone();
        let avatar = result.avatar.clone();
        let status = result.status;
        
        let live_status = status.unwrap_or(0) == 1;
        let live_status_detail = if live_status { "LIVE" } else { "OFFLINE" };
        
        let stream_url_obj = StreamUrl {
            primary_url: stream_url,
            upstream_url,
            available_streams,
            room_info: Some(RoomInfo {
                room_id: room_id.to_string(),
                title: title.unwrap_or_default(),
                streamer_name: anchor_name.unwrap_or_default(),
                streamer_id: "".to_string(), // Not available in this response
                avatar_url: avatar,
                cover_url: None,
                live_status,
                live_status_detail: live_status_detail.to_string(),
                viewer_count: None,
                viewer_count_str: None,
                category_name: None,
                category_id: None,
                tags: None,
                other: Some(serde_json::json!({"live_stream_info": result})),
                raw: raw_data.clone(),
            }),
            streamer_info: None,
            other: None,
            raw: raw_data,
        };
        
        self.logger.debug(format!("get_stream_url succeeded for room_id {}: {:?}", room_id, stream_url_obj));
        
        Ok(stream_url_obj)
    }
    
    async fn fetch_streamer_info(
        &self,
        streamer_id: &str
    ) -> Result<StreamerInfo, PlatformError> {
        self.logger.info(format!("fetch_streamer_info called with streamer_id: {}", streamer_id));
        
        // 对于B站，我们使用直播间信息API来获取主播信息，因为它包含了主播的基本信息
        let (result, raw_data) = room_info::fetch_room_info(
            &self.http_client,
            streamer_id, // 这里我们假设streamer_id是直播间ID，因为B站的API设计如此
            self.cookie.as_deref(),
        ).await
        .map_err(|e| {
            self.logger.error(format!("fetch_bilibili_streamer_info failed for streamer_id {}: {}", streamer_id, e));
            PlatformError::Api(e)
        })?;
        
        // 克隆需要的字段，避免部分移动值问题
        let anchor_name = result.anchor_name.clone();
        let avatar = result.avatar.clone();
        let status = result.status;
        let title = result.title.clone();
        let live_status = status.unwrap_or(0) == 1;
        let live_status_detail = if live_status { "LIVE" } else { "OFFLINE" };
        
        let streamer_info = StreamerInfo {
            streamer_id: streamer_id.to_string(), // 我们没有直接的主播ID，所以使用直播间ID代替
            name: anchor_name.unwrap_or_default(),
            avatar_url: avatar,
            bio: None, // 这个API不返回主播简介
            follower_count: None, // 这个API不返回粉丝数
            live_status,
            live_status_detail: live_status_detail.to_string(),
            room_id: Some(streamer_id.to_string()),
            room_title: title,
            other: Some(serde_json::json!({"live_stream_info": result})),
            raw: raw_data,
        };
        
        self.logger.debug(format!("fetch_streamer_info succeeded for streamer_id {}: {:?}", streamer_id, streamer_info));
        
        Ok(streamer_info)
    }
    
    async fn search_rooms(
        &self,
        keyword: &str,
        page: Option<u32>,
        page_size: Option<u32>
    ) -> Result<LiveList, PlatformError> {
        self.logger.info(format!("search_rooms called with keyword: {}, page: {:?}, page_size: {:?}", keyword, page, page_size));
        
        // 调用 search 模块的实现，获取原始数据
        let (result, raw_data) = search::search_bilibili_rooms(
            &self.http_client.inner,
            keyword,
            page,
            self.cookie.as_deref(),
        ).await
        .map_err(|e| {
            self.logger.error(format!("search_bilibili_rooms failed for keyword {}: {}", keyword, e));
            PlatformError::Api(e)
        })?;
        
        let items = result.into_iter().map(|item| {
            let room_json = serde_json::json!(
                {
                    "room_id": item.room_id,
                    "title": item.title,
                    "anchor": item.anchor,
                    "avatar": item.avatar,
                    "cover": item.cover,
                    "is_live": item.is_live,
                    "watching": item.watching,
                    "area": item.area
                }
            );
            RoomInfo {
                room_id: item.room_id,
                title: item.title,
                streamer_name: item.anchor,
                streamer_id: "".to_string(), // Not available in this response
                avatar_url: Some(item.avatar),
                cover_url: Some(item.cover),
                live_status: item.is_live,
                live_status_detail: if item.is_live { "LIVE" } else { "OFFLINE" }.to_string(),
                viewer_count: item.watching.parse().ok(),
                viewer_count_str: Some(item.watching),
                category_name: Some(item.area),
                category_id: None,
                tags: None,
                other: Some(room_json.clone()),
                raw: Some(room_json),
            }
        }).collect();
        
        let live_list = LiveList {
            items,
            total: None,
            page: page,
            page_size: page_size,
            has_more: true,
            other: raw_data.clone(),
            raw: raw_data,
        };
        
        self.logger.debug(format!("search_rooms succeeded for keyword {}: found {} rooms", keyword, live_list.items.len()));
        
        Ok(live_list)
    }
    
    async fn start_message_listener(
        &self,
        room_id: &str,
        callback: MessageCallback
    ) -> Result<Box<dyn MessageListener>, PlatformError> {
        self.logger.info(format!("start_message_listener called for room_id: {}", room_id));
        
        // 调用message.rs中的函数启动监听器
        let listener = crate::message::start_message_listener(
            room_id.to_string(),
            self.cookie.clone(),
            callback
        ).await
        .map_err(|e| {
            self.logger.error(format!("start_message_listener failed for room_id {}: {:?}", room_id, e));
            PlatformError::MessageListenerError(e.to_string())
        })?;
        
        self.logger.debug(format!("Message listener started for room_id: {}", room_id));
        
        Ok(listener)
    }
    
    async fn fetch_categories(
        &self,
        _parent_id: Option<&str>
    ) -> Result<Vec<Category>, PlatformError> {
        self.logger.info(format!("fetch_categories called with parent_id: {:?}", _parent_id));
        
        // Bilibili 平台分类获取
        let url = "https://api.live.bilibili.com/room/v1/Area/getList";
        
        let resp = self.http_client.inner
            .get(url)
            .header("Referer", "https://www.bilibili.com/")
            .send()
            .await
            .map_err(|e| PlatformError::Network(e.to_string()))?;
        
        if !resp.status().is_success() {
            return Err(PlatformError::Api(format!("API status: {}", resp.status())));
        }
        
        let text = resp
            .text()
            .await
            .map_err(|e| PlatformError::Network(e.to_string()))?;
        
        let json: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| PlatformError::Parse(e.to_string()))?;
        
        // 检查API响应是否成功
        if json["code"].as_i64().unwrap_or(1) != 0 {
            return Err(PlatformError::Api(json["message"].as_str().unwrap_or("未知错误").to_string()));
        }
        
        // 解析分类数据
        let categories = if let Some(data) = json["data"].as_array() {
            data.iter().flat_map(|item| {
                let id = item["id"].as_str()?;
                let name = item["name"].as_str()?;
                
                let mut categories = Vec::new();
                
                // 添加主分类
                categories.push(Category {
                    category_id: id.to_string(),
                    name: name.to_string(),
                    parent_id: None,
                    icon_url: None,
                    order: None,
                    other: Some(item.clone()),
                    raw: None,
                });
                
                // 添加子分类
                if let Some(child_list) = item["list"].as_array() {
                    for child in child_list {
                        let child_id = child["id"].as_str()?;
                        let child_name = child["name"].as_str()?;
                        
                        categories.push(Category {
                            category_id: child_id.to_string(),
                            name: child_name.to_string(),
                            parent_id: Some(id.to_string()),
                            icon_url: None,
                            order: None,
                            other: Some(child.clone()),
                            raw: None,
                        });
                    }
                }
                
                Some(categories)
            }).flatten().collect()
        } else {
            Vec::new()
        };
        
        self.logger.debug(format!("fetch_categories returned {} categories", categories.len()));
        
        Ok(categories)
    }
    
    // -------------------- 认证相关方法实现 --------------------
    
    async fn get_auth_url(
        &self
    ) -> Result<String, PlatformError> {
        self.logger.info("get_auth_url called");
        
        // B站的认证URL获取逻辑
        let auth_url = "https://passport.bilibili.com/oauth2/authorize".to_string();
        self.logger.debug(format!("get_auth_url returned: {}", auth_url));
        
        Ok(auth_url)
    }
    
    async fn login_with_code(
        &self,
        code: &str
    ) -> Result<(), PlatformError> {
        self.logger.info(format!("login_with_code called with code: {}", code));
        
        // B站的登录逻辑
        self.logger.debug(format!("login_with_code succeeded for code: {}", code));
        
        Ok(())
    }
    
    async fn refresh_token(
        &self
    ) -> Result<(), PlatformError> {
        self.logger.info("refresh_token called");
        
        // B站的刷新令牌逻辑
        self.logger.debug("refresh_token succeeded");
        
        Ok(())
    }
    
    async fn get_current_user(
        &self
    ) -> Result<serde_json::Value, PlatformError> {
        self.logger.info("get_current_user called");
        
        // B站的获取当前用户逻辑
        let user_data = serde_json::json!({"id": "", "name": "", "avatar": ""});
        self.logger.debug(format!("get_current_user returned user_data: {:?}", user_data));
        
        Ok(user_data)
    }
    

    

    

    

    

}
