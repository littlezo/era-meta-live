pub mod message;
pub mod live_list;
pub mod search;
pub mod stream_url;
pub mod room_info;
pub mod models;

use std::sync::Arc;

use shared::interface::{
    LivePlatform,
    PlatformType,
    StreamQuality,
    LiveListParams,
    LiveList,
    RoomInfo,
    StreamerInfo,
    StreamUrl,
    PlatformError,
    MessageCallback,
    PlatformConfig,
    Category,
    MessageListener,
};
use shared::http_client::HttpClient;
use shared::logger::Logger;

// Huya 平台实现
pub struct HuyaPlatform {
    http_client: Arc<HttpClient>,
    logger: Logger,
}

impl HuyaPlatform {
    pub fn new(_config: PlatformConfig) -> Result<Self, PlatformError> {
        let http_client = HttpClient::new()
            .map_err(|e| PlatformError::Network(e.to_string()))?;
        
        let logger = Logger::new(Some(PlatformType::Huya), "huya");
        
        Ok(Self {
            http_client: Arc::new(http_client),
            logger,
        })
    }
}

#[async_trait::async_trait]
impl LivePlatform for HuyaPlatform {
    fn platform_type(&self) -> PlatformType {
        PlatformType::Huya
    }
    
    async fn fetch_live_list(
        &self,
        params: LiveListParams
    ) -> Result<LiveList, PlatformError> {
        self.logger.info(format!("fetch_live_list called with params: {:?}", params));
        
        // 调用 live_list 模块的实现，获取原始数据
        let (mut live_list, raw_data) = live_list::fetch_huya_live_list(self.http_client.clone(), params)
            .await
            .map_err(|e| {
                self.logger.error(format!("fetch_huya_live_list failed: {}", e));
                PlatformError::Api(e)
            })?;
        
        // 设置raw字段
        live_list.raw = raw_data;
        
        // 为每个RoomInfo设置raw字段
        for item in &mut live_list.items {
            item.raw = Some(serde_json::json!({})); // 暂时使用空对象，后续可以根据实际情况填充
        }
        
        self.logger.debug(format!("fetch_live_list succeeded: {:?}", live_list));
        
        Ok(live_list)
    }
    
    async fn fetch_room_info(
        &self,
        room_id: &str
    ) -> Result<RoomInfo, PlatformError> {
        self.logger.info(format!("fetch_room_info called with room_id: {}", room_id));
        
        // 使用现有的fetch_room_info函数来获取直播间信息，包括原始数据
        let (detail, raw_data) = room_info::fetch_room_info(&self.http_client, room_id)
            .await
            .map_err(|e| {
                self.logger.error(format!("fetch_room_info failed for room_id {}: {}", room_id, e));
                e
            })?;
        
        // 构建统一的RoomInfo结构体
        let detail_clone = detail.clone();
        let live_status = detail.status;
        let live_status_detail = if live_status { "LIVE" } else { "OFFLINE" };
        let room_info = RoomInfo {
            room_id: room_id.to_string(),
            title: detail.title.unwrap_or_default(),
            streamer_name: detail.nick.unwrap_or_default(),
            streamer_id: room_id.to_string(), // 使用room_id作为streamer_id，实际应该有专门的主播ID
            avatar_url: detail.avatar180.clone(),
            cover_url: None, // 这个API不返回封面URL
            live_status,
            live_status_detail: live_status_detail.to_string(),
            viewer_count: None, // 这个API不返回观众数
            viewer_count_str: None, // 这个API不返回观众数字符串
            category_name: None, // 这个API不返回分类名称
            category_id: None, // 这个API不返回分类ID
            tags: None, // 这个API不返回标签
            other: Some(serde_json::json!({"room_detail": detail_clone})),
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
        
        // 调用 stream_url 模块的实现，包括原始数据
        let (stream_url, raw_data) = stream_url::get_stream_url_for_platform(self.http_client.clone(), room_id, quality)
            .await
            .map_err(|e| {
                self.logger.error(format!("get_stream_url_for_platform failed for room_id {}: {}", room_id, e));
                e
            })?;
        
        // 创建新的StreamUrl对象，添加raw字段
        let stream_url_with_raw = StreamUrl {
            primary_url: stream_url.primary_url,
            upstream_url: stream_url.upstream_url,
            available_streams: stream_url.available_streams,
            room_info: stream_url.room_info,
            streamer_info: stream_url.streamer_info,
            other: stream_url.other,
            raw: raw_data,
        };
        
        self.logger.debug(format!("get_stream_url succeeded for room_id {}: {:?}", room_id, stream_url_with_raw));
        
        Ok(stream_url_with_raw)
    }
    
    async fn fetch_streamer_info(
        &self,
        streamer_id: &str
    ) -> Result<StreamerInfo, PlatformError> {
        self.logger.info(format!("fetch_streamer_info called with streamer_id: {}", streamer_id));
        
        // 对于虎牙，我们使用直播间信息API来获取主播信息，因为它包含了主播的基本信息
        // 注意：这里我们假设streamer_id是主播ID，但虎牙的API设计中，房间信息API需要房间ID
        // 实际项目中，应该实现一个专门的主播信息API，或者维护主播ID到房间ID的映射
        
        // 这里我们使用fetch_room_info方法来获取主播信息
        // 在实际项目中，应该实现更完整的主播信息获取逻辑
        let room_info = self.fetch_room_info(streamer_id).await?;
        
        let title_clone = room_info.title.clone();
        let avatar_url_clone = room_info.avatar_url.clone();
        let streamer_name_clone = room_info.streamer_name.clone();
        let live_status_clone = room_info.live_status;
        
        let live_status_detail = if live_status_clone { "LIVE" } else { "OFFLINE" };
        let streamer_info = StreamerInfo {
            streamer_id: streamer_id.to_string(),
            name: streamer_name_clone,
            avatar_url: avatar_url_clone,
            bio: None, // 需要实现专门的主播信息API来获取
            follower_count: None, // 需要实现专门的主播信息API来获取
            live_status: live_status_clone,
            live_status_detail: live_status_detail.to_string(),
            room_id: Some(streamer_id.to_string()),
            room_title: Some(title_clone),
            other: Some(serde_json::json!({"room_info": room_info})),
            raw: room_info.raw.clone(),
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
        
        // 调用 search 模块的实现，包括原始数据
        let (result, raw_data) = search::search_huya_anchors(keyword, page, page_size)
            .await
            .map_err(|e| {
                self.logger.error(format!("search_huya_anchors failed for keyword {}: {}", keyword, e));
                PlatformError::Api(e)
            })?;
        
        // 转换为统一的 LiveList 结构
        let items = result
            .into_iter()
            .map(|anchor| {
                let room_id_clone = anchor.room_id.clone();
                let avatar_clone = anchor.avatar.clone();
                // 将anchor转换为JSON以便作为raw数据
                let anchor_json = serde_json::json!(
                    {
                        "room_id": anchor.room_id,
                        "title": anchor.title,
                        "user_name": anchor.user_name,
                        "avatar": anchor.avatar,
                        "live_status": anchor.live_status
                    }
                );
                RoomInfo {
                    room_id: anchor.room_id,
                    title: anchor.title,
                    streamer_name: anchor.user_name,
                    streamer_id: room_id_clone, // 使用room_id作为streamer_id，实际应该有专门的主播ID
                    avatar_url: Some(avatar_clone),
                    cover_url: Some(anchor.avatar), // 使用主播头像作为封面，实际应该有专门的封面URL
                    live_status: anchor.live_status,
                    live_status_detail: if anchor.live_status { "LIVE" } else { "OFFLINE" }.to_string(),
                    viewer_count: None, // 搜索结果中没有观众数
                    viewer_count_str: None, // 搜索结果中没有观众数字符串
                    category_name: None,
                    category_id: None,
                    tags: None,
                    other: None,
                    raw: Some(anchor_json),
                }
            })
            .collect();
        
        let live_list = LiveList {
            items,
            total: None,
            page,
            page_size,
            has_more: false, // 搜索接口未提供分页信息
            other: None,
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
        
        // 调用 message 模块的实现，获取 listener
        let listener = message::start_message_listener(room_id, callback)
            .await
            .map_err(|e| {
                self.logger.error(format!("start_message_listener failed for room_id {}: {}", room_id, e));
                PlatformError::Api(e)
            })?;
        
        self.logger.debug(format!("Message listener started for room_id: {}", room_id));
        
        Ok(listener)
    }
    
    async fn fetch_categories(
        &self,
        _parent_id: Option<&str>
    ) -> Result<Vec<Category>, PlatformError> {
        self.logger.info(format!("fetch_categories called with parent_id: {:?}", _parent_id));
        
        // Huya 平台分类获取API
        let url = "https://www.huya.com/cache.php?m=LiveList&do=getLiveListByPage&gameId=0&tagAll=0&page=1";
        
        let resp_value: serde_json::Value = self.http_client
            .get_json(&url)
            .await
            .map_err(|e| PlatformError::Network(e.to_string()))?;
        
        // 解析分类数据
        let categories = if let Some(data) = resp_value["data"]["gameList"].as_array() {
            data.iter().filter_map(|item| {
                let id = item["gameId"].as_i64()?.to_string();
                let name = item["gameName"].as_str()?;
                
                Some(Category {
                    category_id: id,
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
        
        // 虎牙的认证URL获取逻辑
        let auth_url = "https://passport.huya.com/oauth2/authorize".to_string();
        
        self.logger.debug(format!("get_auth_url returned: {}", auth_url));
        
        Ok(auth_url)
    }
    
    async fn login_with_code(
        &self,
        code: &str
    ) -> Result<(), PlatformError> {
        self.logger.info(format!("login_with_code called with code: {}", code));
        
        // 虎牙的登录逻辑
        self.logger.debug(format!("login_with_code succeeded for code: {}", code));
        
        Ok(())
    }
    
    async fn refresh_token(
        &self
    ) -> Result<(), PlatformError> {
        self.logger.info("refresh_token called");
        
        // 虎牙的刷新令牌逻辑
        self.logger.debug("refresh_token succeeded");
        
        Ok(())
    }
    
    async fn get_current_user(
        &self
    ) -> Result<serde_json::Value, PlatformError> {
        self.logger.info("get_current_user called");
        
        // 虎牙的获取当前用户逻辑
        let user_data = serde_json::json!({"id": "", "name": "", "avatar": ""});
        
        self.logger.debug(format!("get_current_user returned user_data: {:?}", user_data));
        
        Ok(user_data)
    }
    

    

    

    

}

