pub mod message;
pub mod categories;
pub mod room_info;
pub mod live_list;
pub mod search;
pub mod stream_url;
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
    Category,
    PlatformConfig,
    MessageListener,
};
use shared::http_client::HttpClient;
use shared::logger::Logger;

// Douyu 平台实现
pub struct DouyuPlatform {
    http_client: Arc<HttpClient>,
    logger: Logger,
}

impl DouyuPlatform {
    pub fn new(_config: PlatformConfig) -> Result<Self, PlatformError> {
        let http_client = HttpClient::new()
            .map_err(|e| PlatformError::Network(e.to_string()))?;
        
        let logger = Logger::new(Some(PlatformType::Douyu), "douyu");
        
        Ok(Self {
            http_client: Arc::new(http_client),
            logger,
        })
    }
}

#[async_trait::async_trait]
impl LivePlatform for DouyuPlatform {
    fn platform_type(&self) -> PlatformType {
        PlatformType::Douyu
    }
    
    async fn fetch_live_list(
        &self,
        params: LiveListParams
    ) -> Result<LiveList, PlatformError> {
        self.logger.info(format!("fetch_live_list called with params: {:?}", params));
        
        // 调用 live_list 模块的实现，获取包含原始数据的结果
        let result = live_list::fetch_live_list_with_params(self.http_client.clone(), params)
            .await
            .map_err(|e: String| {
                self.logger.error(format!("fetch_live_list_with_params failed: {}", e));
                PlatformError::Api(e)
            })?;
        
        let (mut live_list, raw_data) = result;
        
        // 设置raw字段
        live_list.raw = raw_data;
        
        self.logger.debug(format!("fetch_live_list succeeded: found {} rooms", live_list.items.len()));
        
        Ok(live_list)
    }
    
    async fn fetch_room_info(
        &self,
        room_id: &str
    ) -> Result<RoomInfo, PlatformError> {
        self.logger.info(format!("fetch_room_info called with room_id: {}", room_id));
        
        // 调用 fetch_room_info 模块的实现，获取原始数据
        let (mut room_info, raw_data) = room_info::fetch_room_info(
            &self.http_client,
            room_id.to_string(),
        ).await
        .map_err(|e| {
            self.logger.error(format!("fetch_room_info failed: {:?}", e));
            PlatformError::Api(e.to_string())
        })?;
        
        // 设置raw字段
        room_info.raw = raw_data;
        
        self.logger.debug(format!("fetch_room_info succeeded for room_id: {}, result: {:?}", room_id, room_info));
        
        Ok(room_info)
    }
    
    async fn get_stream_url(
        &self,
        room_id: &str,
        quality: Option<StreamQuality>
    ) -> Result<StreamUrl, PlatformError> {
        self.logger.info(format!("get_stream_url called with room_id: {}, quality: {:?}", room_id, quality));
        
        // 调用 stream_url 模块的实现，获取原始数据
        let (mut stream_url, raw_data) = stream_url::get_stream_url_for_platform(&self.http_client, room_id, quality)
            .await
            .map_err(|e| {
                self.logger.error(format!("get_stream_url_for_platform failed: {:?}", e));
                e
            })?;
        
        // 设置raw字段
        stream_url.raw = raw_data;
        
        self.logger.debug(format!("get_stream_url succeeded for room_id: {}, result: {:?}", room_id, stream_url));
        
        Ok(stream_url)
    }
    
    async fn fetch_streamer_info(
        &self,
        streamer_id: &str
    ) -> Result<StreamerInfo, PlatformError> {
        self.logger.info(format!("fetch_streamer_info called with streamer_id: {}", streamer_id));
        
        // 对于斗鱼，我们使用直播间信息API来获取主播信息，因为它包含了主播的基本信息
        // 注意：这里我们假设streamer_id是直播间ID，因为斗鱼的API设计如此
        
        // 调用fetch_room_info来获取直播间信息，从中提取主播信息，获取原始数据
        let (room_info, raw_data) = room_info::fetch_room_info(
            &self.http_client,
            streamer_id.to_string(),
        ).await
        .map_err(|e| {
            self.logger.error(format!("fetch_room_info failed: {:?}", e));
            PlatformError::Api(e.to_string())
        })?;
        
        // 克隆需要的字段，避免部分移动值问题
        let streamer_name = room_info.streamer_name.clone();
        let avatar_url = room_info.avatar_url.clone();
        let live_status = room_info.live_status;
        let title = room_info.title.clone();
        
        // 从房间信息中提取主播信息
        let live_status_detail = if live_status { "LIVE" } else { "OFFLINE" };
        let streamer_info = StreamerInfo {
            streamer_id: streamer_id.to_string(),
            name: streamer_name,
            avatar_url: avatar_url,
            bio: None, // 这个API不返回主播简介
            follower_count: None, // 这个API不返回粉丝数
            live_status: live_status,
            live_status_detail: live_status_detail.to_string(),
            room_id: Some(streamer_id.to_string()),
            room_title: Some(title),
            other: Some(serde_json::json!({"room_info": room_info})),
            raw: raw_data, // 设置raw字段
        };
        
        self.logger.debug(format!("fetch_streamer_info succeeded for streamer_id: {}, result: {:?}", streamer_id, streamer_info));
        Ok(streamer_info)
    }
    
    async fn search_rooms(
        &self,
        keyword: &str,
        page: Option<u32>,
        page_size: Option<u32>
    ) -> Result<LiveList, PlatformError> {
        self.logger.info(format!("search_rooms called with keyword: {}, page: {:?}, page_size: {:?}", keyword, page, page_size));
        
        // 调用 search_anchor 模块的实现，获取原始数据
        let (result, raw_data) = search::search_anchor(keyword, page, page_size)
            .await
            .map_err(|e| {
                self.logger.error(format!("search_anchor failed: {}", e));
                PlatformError::Api(e.to_string())
            })?;
        
        // 转换为统一的 LiveList 结构
        let items = result
            .into_iter()
            .map(|anchor| RoomInfo {
                room_id: anchor.room_id,
                title: anchor.room_name,
                streamer_name: anchor.nickname,
                streamer_id: anchor.uid,
                avatar_url: Some(anchor.avatar.clone()),
                cover_url: Some(anchor.avatar), // 使用主播头像作为封面，实际应该有专门的封面URL
                live_status: true, // 搜索结果中的主播应该都是在线的
                live_status_detail: "LIVE".to_string(),
                viewer_count: Some(anchor.hot as u64),
                viewer_count_str: Some(anchor.hot.to_string()),
                category_name: None,
                category_id: None,
                tags: None,
                other: None,
                raw: None, // 单个房间没有单独的原始数据
            })
            .collect();
        
        let live_list = LiveList {
            items,
            total: None,
            page,
            page_size,
            has_more: false, // 搜索接口未提供分页信息
            other: None,
            raw: raw_data, // 设置raw字段
        };
        
        self.logger.debug(format!("search_rooms succeeded for keyword: {}, result: {:?}", keyword, live_list));
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
        // 调用 fetch_categories 模块的实现
        categories::fetch_categories(&self.http_client).await
    }
    
    // -------------------- 认证相关方法实现 --------------------
    
    async fn get_auth_url(
        &self
    ) -> Result<String, PlatformError> {
        // 斗鱼的认证URL获取逻辑
        Ok("https://passport.douyu.com/oauth/authorize".to_string())
    }
    
    async fn login_with_code(
        &self,
        _code: &str
    ) -> Result<(), PlatformError> {
        // 斗鱼的登录逻辑
        Ok(())
    }
    
    async fn refresh_token(
        &self
    ) -> Result<(), PlatformError> {
        // 斗鱼的刷新令牌逻辑
        Ok(())
    }
    
    async fn get_current_user(
        &self
    ) -> Result<serde_json::Value, PlatformError> {
        // 斗鱼的获取当前用户逻辑
        let user_data = serde_json::json!({"id": "", "name": "", "avatar": ""});
        Ok(user_data)
    }
    

    

}

