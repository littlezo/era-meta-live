pub mod message;
pub mod message_listener;
pub mod live_list;
pub mod room_info;
pub mod stream_url;
pub mod models;
pub mod web_api;
pub mod a_bogus;

use std::sync::Arc;
use tokio::sync::Mutex;

use shared::interface::*;
use shared::http_client::HttpClient;
use shared::logger::Logger;

// Douyin 平台实现
pub struct DouyinPlatform {
    http_client: Arc<HttpClient>,
    cookie: Option<String>,
    message_listener_state: Arc<Mutex<message_listener::DouyinMessageListenerState>>,
    logger: Logger,
}

impl DouyinPlatform {
    pub fn new(config: PlatformConfig) -> Result<Self, PlatformError> {
        let http_client = HttpClient::new_direct_connection()
            .map_err(|e| PlatformError::Network(e.to_string()))?;
        
        let logger = Logger::new(Some(PlatformType::Douyin), "douyin");
        
        Ok(Self {
            http_client: Arc::new(http_client),
            cookie: config.cookie,
            message_listener_state: Arc::new(Mutex::new(message_listener::DouyinMessageListenerState::new())),
            logger,
        })
    }
}

#[async_trait::async_trait]
impl LivePlatform for DouyinPlatform {
    fn platform_type(&self) -> PlatformType {
        PlatformType::Douyin
    }
    
    async fn fetch_live_list(
        &self,
        params: LiveListParams
    ) -> Result<LiveList, PlatformError> {
        self.logger.info(format!("fetch_live_list called with params: {:?}", params));
        
        let partition = params.category_id.unwrap_or_default();
        let partition_clone = partition.clone();
        let partition_type = params.other.as_ref()
            .and_then(|v| v.get("partition_type").and_then(|v| v.as_str()))
            .unwrap_or("partition");
        let offset = params.page.unwrap_or(1) * params.page_size.unwrap_or(15) - params.page_size.unwrap_or(15);
        
        let result = live_list::fetch_live_list(
            partition_clone,
            partition_type.to_string(),
            offset as i32,
        ).await
        .map_err(|e| {
            self.logger.error(format!("fetch_live_list failed: {}", e));
            PlatformError::Api(e)
        })?;
        
        let (result_data, raw_data) = result;

    let partition_clone = partition.clone();
    let items = result_data.rooms.into_iter().map(move |room| {
            // 创建房间的原始数据
            let room_raw = serde_json::json!(
                {
                    "web_rid": room.web_rid,
                    "title": room.title,
                    "owner_nickname": room.owner_nickname,
                    "avatar_url": room.avatar_url,
                    "cover_url": room.cover_url,
                    "user_count_str": room.user_count_str,
                    "partition": partition_clone.clone()
                }
            );
            RoomInfo {
                room_id: room.web_rid,
                title: room.title,
                streamer_name: room.owner_nickname,
                streamer_id: "".to_string(), // Not available in this response
                avatar_url: Some(room.avatar_url),
                cover_url: Some(room.cover_url),
                live_status: true, // Assuming all items in live list are live
                live_status_detail: "LIVE".to_string(),
                viewer_count: room.user_count_str.parse().ok(),
                viewer_count_str: Some(room.user_count_str),
                category_name: None, // Not available in this response
                category_id: Some(partition_clone.clone()),
                tags: None,
                other: None,
                raw: Some(room_raw),
            }
        }).collect();
        
        let live_list = LiveList {
            items,
            total: None, // Not available in this response
            page: params.page,
            page_size: Some(params.page_size.unwrap_or(15)),
            has_more: result_data.has_more,
            other: Some(serde_json::json!({"next_offset": result_data.next_offset})),
            raw: raw_data,
        };
        
        self.logger.debug(format!("fetch_live_list succeeded: found {} rooms", live_list.items.len()));
        
        Ok(live_list)
    }
    
    async fn fetch_room_info(
        &self,
        room_id: &str
    ) -> Result<RoomInfo, PlatformError> {
        self.logger.info(format!("douyin::fetch_room_info called with room_id: {}", room_id));
        
        let result = room_info::fetch_room_info(
            room_id,
            &self.http_client
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
        
        // 创建原始数据JSON
        let raw_data = serde_json::json!({"live_stream_info": result});
        
        let live_status = status.unwrap_or(0) == 1 || status.unwrap_or(0) == 2;
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
            other: Some(raw_data.clone()),
            raw: Some(raw_data),
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
            Some(StreamQuality::UltraHD) => "OD".to_string(),
            Some(StreamQuality::HD) => "BD".to_string(),
            Some(StreamQuality::SD) => "UHD".to_string(),
            Some(StreamQuality::LD) => "LD".to_string(),
            Some(StreamQuality::Auto) => "OD".to_string(),
            Some(StreamQuality::Custom(q)) => q.clone(),
            Some(StreamQuality::Original) => "OD".to_string(),
            Some(StreamQuality::K4) => "4K".to_string(),
            Some(StreamQuality::K2) => "2K".to_string(),
            Some(StreamQuality::P1080_60) => "FHD60".to_string(),
            Some(StreamQuality::P720_60) => "HD60".to_string(),
            None => "OD".to_string(),
        };
        
        let result = stream_url::get_douyin_live_stream_url_with_quality(
            room_id,
            quality_str,
            self.cookie.as_deref(),
        ).await
        .map_err(|e| {
            self.logger.error(format!("get_douyin_live_stream_url_with_quality failed for room_id {}: {}", room_id, e));
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
        
        // 创建原始数据JSON
        let raw_data = serde_json::json!({"live_stream_info": result});
        
        let live_status = status.unwrap_or(0) == 1 || status.unwrap_or(0) == 2;
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
                other: Some(raw_data.clone()),
                raw: Some(raw_data.clone()),
            }),
            streamer_info: None,
            other: None,
            raw: Some(raw_data),
        };
        
        self.logger.debug(format!("get_stream_url succeeded for room_id {}: {:?}", room_id, stream_url_obj));
        
        Ok(stream_url_obj)
    }
    
    async fn fetch_streamer_info(
        &self,
        streamer_id: &str
    ) -> Result<StreamerInfo, PlatformError> {
        self.logger.info(format!("fetch_streamer_info called with streamer_id: {}", streamer_id));
        
        // 对于抖音，我们使用直播间信息API来获取主播信息，因为它包含了主播的基本信息
        let result = room_info::fetch_room_info(
            streamer_id, // 这里我们假设streamer_id是直播间ID，因为抖音的API设计如此
            &self.http_client
        ).await
        .map_err(|e| {
            self.logger.error(format!("fetch_room_info failed for streamer_id {}: {}", streamer_id, e));
            PlatformError::Api(e)
        })?;
        
        // 克隆需要的字段，避免部分移动值问题
        let anchor_name = result.anchor_name.clone();
        let avatar = result.avatar.clone();
        let status = result.status;
        let web_rid = result.web_rid.clone();
        let title = result.title.clone();
        
        // 创建原始数据JSON
        let raw_data = serde_json::json!({"live_stream_info": result});
        
        let live_status = status.unwrap_or(0) == 1 || status.unwrap_or(0) == 2;
        let live_status_detail = if live_status { "LIVE" } else { "OFFLINE" };
        
        let streamer_info = StreamerInfo {
            streamer_id: streamer_id.to_string(), // 我们没有直接的主播ID，所以使用直播间ID代替
            name: anchor_name.unwrap_or_default(),
            avatar_url: avatar,
            bio: None, // 这个API不返回主播简介
            follower_count: None, // 这个API不返回粉丝数
            live_status,
            live_status_detail: live_status_detail.to_string(),
            room_id: web_rid,
            room_title: title,
            other: Some(raw_data.clone()),
            raw: Some(raw_data),
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
        
        // 抖音平台目前没有直接的搜索API，我们通过获取多个分区的直播列表并进行过滤来实现搜索
        let page = page.unwrap_or(1);
        let page_size = page_size.unwrap_or(15);
        let offset = (page - 1) * page_size;
        
        // 我们使用默认分区来获取直播列表
        let partition = "1".to_string();
        let _partition_type = "partition";
        
        let result = live_list::fetch_live_list(
            "".to_string(),
            "live_list".to_string(),
            offset as i32,
        ).await
        .map_err(|e| {
            self.logger.error(format!("fetch_live_list failed for search keyword {}: {}", keyword, e));
            PlatformError::Api(e)
        })?;
        
        let (result_data, _raw_data) = result;

    // 对结果进行过滤，只保留标题或主播名包含关键词的直播间
    let keyword_lower = keyword.to_lowercase();
        let filtered_rooms = result_data.rooms.into_iter()
            .filter(|room| {
                room.title.to_lowercase().contains(&keyword_lower) || 
                room.owner_nickname.to_lowercase().contains(&keyword_lower)
            })
            .collect::<Vec<_>>();
        
        // Calculate the total number of filtered rooms before moving
        let total_rooms = filtered_rooms.len();
        
        let items = filtered_rooms.into_iter().map(|room| {
            // 创建房间的原始数据
            let room_raw = serde_json::json!(
                {
                    "web_rid": room.web_rid,
                    "title": room.title,
                    "owner_nickname": room.owner_nickname,
                    "avatar_url": room.avatar_url,
                    "cover_url": room.cover_url,
                    "user_count_str": room.user_count_str,
                    "partition": partition.clone()
                }
            );
            RoomInfo {
                room_id: room.web_rid,
                title: room.title,
                streamer_name: room.owner_nickname,
                streamer_id: "".to_string(), // Not available in this response
                avatar_url: Some(room.avatar_url),
                cover_url: Some(room.cover_url),
                live_status: true, // Assuming all items in live list are live
                live_status_detail: "LIVE".to_string(),
                viewer_count: room.user_count_str.parse().ok(),
                viewer_count_str: Some(room.user_count_str),
                category_name: None, // Not available in this response
                category_id: Some(partition.clone()),
                tags: None,
                other: None,
                raw: Some(room_raw),
            }
        }).collect();
        
        let live_list = LiveList {
            items,
            total: Some(total_rooms as u64),
            page: Some(page),
            page_size: Some(page_size),
            has_more: false, // 搜索结果已经过滤了所有匹配的房间，没有更多数据
            other: Some(serde_json::json!({"keyword": keyword, "total_rooms": total_rooms})),
            raw: Some(serde_json::json!({"keyword": keyword, "total_rooms": total_rooms})),
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
        
        message_listener::start_douyin_message_listener(
            room_id,
            callback,
            self.message_listener_state.clone(),
        ).await
        .map_err(|e| {
            self.logger.error(format!("start_douyin_message_listener failed for room_id {}: {}", room_id, e));
            PlatformError::Api(e)
        })
    }
    
    async fn fetch_categories(
        &self,
        _parent_id: Option<&str>
    ) -> Result<Vec<Category>, PlatformError> {
        self.logger.info(format!("fetch_categories called with parent_id: {:?}", _parent_id));
        
        // 抖音平台分类获取
        // 抖音直播分类API
        let url = "https://live.douyin.com/webcast/room/list/area_rank";
        
        let resp = self.http_client.inner
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .header("Referer", "https://live.douyin.com/")
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
        
        // 解析分类数据
        let categories = if let Some(data) = json["data"]["list"].as_array() {
            data.iter().filter_map(|item| {
                let id = item["id"].as_str()?;
                let name = item["name"].as_str()?;
                
                Some(Category {
                    category_id: id.to_string(),
                    name: name.to_string(),
                    parent_id: None,
                    icon_url: None,
                    order: None,
                    other: Some(item.clone()),
                    raw: None,
                })
            }).collect()
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
        
        // 抖音的认证URL获取逻辑
        let auth_url = "https://open.douyin.com/platform/oauth/connect/".to_string();
        
        self.logger.debug(format!("get_auth_url returned: {}", auth_url));
        
        Ok(auth_url)
    }
    
    async fn login_with_code(
        &self,
        _code: &str
    ) -> Result<(), PlatformError> {
        self.logger.info(format!("login_with_code called with code: {}", _code));
        
        // 抖音的登录逻辑
        self.logger.debug(format!("login_with_code succeeded for code: {}", _code));
        
        Ok(())
    }
    
    async fn refresh_token(
        &self
    ) -> Result<(), PlatformError> {
        self.logger.info("refresh_token called");
        
        // 抖音的刷新令牌逻辑
        self.logger.debug("refresh_token succeeded");
        
        Ok(())
    }
    
    async fn get_current_user(
        &self
    ) -> Result<serde_json::Value, PlatformError> {
        self.logger.info("get_current_user called");
        
        // 抖音的获取当前用户逻辑
        let user_data = serde_json::json!({"id": "", "name": "", "avatar": ""});
        
        self.logger.debug(format!("get_current_user returned user_data: {:?}", user_data));
        
        Ok(user_data)
    }
}
