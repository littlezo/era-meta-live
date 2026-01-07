use serde::{Deserialize, Serialize};
use std::fmt::Display;
use thiserror::Error;

use super::types::StreamVariant;
use chrono::Utc;

// 平台类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PlatformType {
    Bilibili,
    Douyin,
    Douyu,
    Huya,
}

impl Display for PlatformType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlatformType::Bilibili => write!(f, "bilibili"),
            PlatformType::Douyin => write!(f, "douyin"),
            PlatformType::Douyu => write!(f, "douyu"),
            PlatformType::Huya => write!(f, "huya"),
        }
    }
}

// 实现 FromStr trait，允许从字符串解析 PlatformType
impl std::str::FromStr for PlatformType {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bilibili" => Ok(PlatformType::Bilibili),
            "douyin" => Ok(PlatformType::Douyin),
            "douyu" => Ok(PlatformType::Douyu),
            "huya" => Ok(PlatformType::Huya),
            _ => Err(format!("Unsupported platform: {}", s)),
        }
    }
}

// 直播流质量枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamQuality {
    UltraHD,  // 超清
    HD,       // 高清
    SD,       // 标清
    LD,       // 流畅
    Auto,     // 自动
    Custom(String), // 自定义质量
    Original, // 原画
    K4,       // 4K
    K2,       // 2K
    P1080_60,  // 1080P 60fps
    P720_60,   // 720P 60fps
}

// 直播列表查询参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveListParams {
    pub category_id: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub sort: Option<String>,
    pub other: Option<serde_json::Value>,
}

// 直播间信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInfo {
    pub room_id: String,
    pub title: String,
    pub streamer_name: String,
    pub streamer_id: String,
    pub avatar_url: Option<String>,
    pub cover_url: Option<String>,
    pub live_status: bool,
    pub live_status_detail: String, // "LIVE" | "OFFLINE" | "REPLAY" | "UNKNOWN"
    pub viewer_count: Option<u64>,
    pub viewer_count_str: Option<String>,
    pub category_name: Option<String>,
    pub category_id: Option<String>,
    pub tags: Option<Vec<String>>,
    pub other: Option<serde_json::Value>,
    pub raw: Option<serde_json::Value>,
}

// 主播信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamerInfo {
    pub streamer_id: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub follower_count: Option<u64>,
    pub live_status: bool,
    pub live_status_detail: String, // "LIVE" | "OFFLINE" | "REPLAY" | "UNKNOWN"
    pub room_id: Option<String>,
    pub room_title: Option<String>,
    pub other: Option<serde_json::Value>,
    pub raw: Option<serde_json::Value>,
}

// 直播列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveList {
    pub items: Vec<RoomInfo>,
    pub total: Option<u64>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub has_more: bool,
    pub other: Option<serde_json::Value>,
    pub raw: Option<serde_json::Value>,
}

// 直播流URL响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamUrl {
    pub primary_url: String,
    pub upstream_url: Option<String>,
    pub available_streams: Vec<StreamVariant>,
    pub room_info: Option<RoomInfo>,
    pub streamer_info: Option<StreamerInfo>,
    pub other: Option<serde_json::Value>,
    pub raw: Option<serde_json::Value>,
}



use std::error::Error;

// 平台错误类型
#[derive(Error, Debug)]
pub enum PlatformError {
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Parse error: {0}")]
    Parse(String),
    
    #[error("API error: {0}")]
    Api(String),
    
    #[error("Authentication error: {0}")]
    Auth(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Platform error: {0}")]
    Platform(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Unsupported operation: {0}")]
    Unsupported(String),
    
    #[error("Other error: {0}")]
    Other(String),
    
    #[error("Message listener error: {0}")]
    MessageListenerError(String),
    
    #[error("Wrapped error: {0}")]
    Wrapped(anyhow::Error),
    
    #[error("API error with raw data: {0}")]
    ApiWithRaw(String, Option<serde_json::Value>),
    
    #[error("Parse error with raw data: {0}")]
    ParseWithRaw(String, Option<serde_json::Value>),
}

impl From<anyhow::Error> for PlatformError {
    fn from(error: anyhow::Error) -> Self {
        PlatformError::Wrapped(error)
    }
}

impl PlatformError {
    /// 创建一个包含原始错误的平台错误
    pub fn wrap<E>(error: E, message: &str) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Wrapped(anyhow::anyhow!("{}: {:?}", message, error))
    }
    
    /// 创建带有原始数据的API错误
    pub fn api_with_raw(message: &str, raw: Option<serde_json::Value>) -> Self {
        Self::ApiWithRaw(message.to_string(), raw)
    }
    
    /// 创建带有原始数据的解析错误
    pub fn parse_with_raw(message: &str, raw: Option<serde_json::Value>) -> Self {
        Self::ParseWithRaw(message.to_string(), raw)
    }
    
    /// 获取完整的错误信息，包括错误链
    pub fn full_error_message(&self) -> String {
        let mut message = self.to_string();
        
        // 处理错误链
        if let PlatformError::Wrapped(ref e) = self {
            let mut cause = e.source();
            let mut depth = 1;
            
            while let Some(c) = cause {
                message.push_str(&format!("\n  Cause {}: {}", depth, c));
                cause = c.source();
                depth += 1;
            }
        }
        
        message
    }
    
    /// 获取原始API响应数据
    pub fn raw(&self) -> Option<&serde_json::Value> {
        match self {
            PlatformError::ApiWithRaw(_, ref raw) => raw.as_ref(),
            PlatformError::ParseWithRaw(_, ref raw) => raw.as_ref(),
            _ => None,
        }
    }
}

// 消息监听器回调
pub type MessageCallback = Box<dyn Fn(Message) + Send + Sync + 'static>;

// 消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Danmaku,       // 弹幕
    Gift,          // 礼物
    SuperChat,     // 超级弹幕
    EnterRoom,     // 进入房间
    Follow,        // 关注
    Unfollow,      // 取消关注
    Like,          // 点赞
    Share,         // 分享
    System,        // 系统消息
    GiftCombo,     // 礼物连击
    GuardBuy,      // 购买守护
    RoomChange,    // 房间信息变更
    Other(String), // 其他类型
}

// 消息内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Option<String>,
    pub message_type: MessageType,
    pub user: String,
    pub content: String,
    pub user_level: Option<i64>,
    pub fans_level: Option<i32>,
    pub fans_club_level: Option<i32>,
    pub badge_name: Option<String>,
    pub badge_level: Option<i32>,
    pub uid: Option<String>,
    pub color: Option<String>,
    pub timestamp: u64,
    pub room_id: String,
    pub platform: PlatformType,
    
    // 礼物相关字段
    pub gift_name: Option<String>,
    pub gift_count: Option<u32>,
    pub gift_price: Option<f64>,
    pub gift_total: Option<f64>,
    
    // 超级弹幕相关字段
    pub super_chat_price: Option<f64>,
    pub super_chat_duration: Option<u32>,
    
    // 连击相关字段
    pub combo_count: Option<u32>,
    pub combo_user: Option<String>,
    
    // 原始数据
    pub raw: Option<serde_json::Value>,
    
    // 其他扩展字段
    pub other: Option<serde_json::Value>,
}

// 消息监听器状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListenerStatus {
    pub platform: PlatformType,
    pub room_id: String,
    pub status: String, // "CONNECTED" | "DISCONNECTING" | "DISCONNECTED" | "CONNECTING"
    pub message_count: u32,
    pub last_update: u64,
}

// 消息监听器 trait，用于停止消息监听
#[async_trait::async_trait]
pub trait MessageListener: Send + Sync {
    // 停止消息监听
    async fn stop(&mut self) -> Result<(), PlatformError>;
    
    // 获取监听器状态
    fn status(&self) -> ListenerStatus;
    
    // 获取消息计数
    fn message_count(&self) -> u32;
}

// 统一的直播平台接口
#[async_trait::async_trait]
pub trait LivePlatform: Send + Sync + 'static {
    // 获取平台类型
    fn platform_type(&self) -> PlatformType;
    
    // 获取直播列表
    async fn fetch_live_list(
        &self,
        params: LiveListParams
    ) -> Result<LiveList, PlatformError>;
    
    // 获取直播间信息
    async fn fetch_room_info(
        &self,
        room_id: &str
    ) -> Result<RoomInfo, PlatformError>;
    
    // 获取直播流URL
    async fn get_stream_url(
        &self,
        room_id: &str,
        quality: Option<StreamQuality>
    ) -> Result<StreamUrl, PlatformError>;
    
    // 获取主播信息
    async fn fetch_streamer_info(
        &self,
        streamer_id: &str
    ) -> Result<StreamerInfo, PlatformError>;
    
    // 搜索直播间
    async fn search_rooms(
        &self,
        keyword: &str,
        page: Option<u32>,
        page_size: Option<u32>
    ) -> Result<LiveList, PlatformError>;
    
    // 启动消息监听，返回一个可以停止监听的监听器实例
    async fn start_message_listener(
        &self,
        room_id: &str,
        callback: MessageCallback
    ) -> Result<Box<dyn MessageListener>, PlatformError>;
    
    // 检查直播间是否在线
    async fn check_room_status(
        &self,
        room_id: &str
    ) -> Result<bool, PlatformError> {
        let room_info = self.fetch_room_info(room_id).await?;
        Ok(room_info.live_status)
    }
    
    // 获取直播分类列表（可选实现）
    async fn fetch_categories(
        &self,
        _parent_id: Option<&str>
    ) -> Result<Vec<Category>, PlatformError> {
        Err(PlatformError::Unsupported("fetch_categories not implemented".to_string()))
    }
    
    // -------------------- 新增认证相关方法 --------------------
    
    // 获取认证URL（用于 OAuth 或其他认证方式）
    async fn get_auth_url(
        &self
    ) -> Result<String, PlatformError> {
        Err(PlatformError::Unsupported("get_auth_url not implemented".to_string()))
    }
    
    // 使用认证码登录
    async fn login_with_code(
        &self,
        _code: &str
    ) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported("login_with_code not implemented".to_string()))
    }
    
    // 刷新认证令牌
    async fn refresh_token(
        &self
    ) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported("refresh_token not implemented".to_string()))
    }
    
    // 获取当前用户信息
    async fn get_current_user(
        &self
    ) -> Result<serde_json::Value, PlatformError> {
        Err(PlatformError::Unsupported("get_current_user not implemented".to_string()))
    }
    

}

// 分类信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub category_id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub icon_url: Option<String>,
    pub order: Option<u32>,
    pub other: Option<serde_json::Value>,
    pub raw: Option<serde_json::Value>,
}

// 平台模式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlatformMode {
    Local,
    Remote,
}

impl Default for PlatformMode {
    fn default() -> Self {
        PlatformMode::Local
    }
}

// 平台配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlatformConfig {
    pub user_agent: Option<String>,
    pub cookie: Option<String>,
    pub proxy: Option<String>,
    pub auth_token: Option<String>,
    pub with_auth: bool,
    pub mode: PlatformMode,
    pub remote_url: Option<String>,
    pub other: Option<serde_json::Value>,
}

// 平台创建器
pub type PlatformCreator = Box<dyn Fn(PlatformConfig) -> Result<Box<dyn LivePlatform>, PlatformError> + Send + Sync + 'static>;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_test::{assert_de_tokens, assert_ser_tokens, Token};
    
    #[test]
    fn test_platform_type_conversion() {
        // 测试 PlatformType 到字符串的转换
        assert_eq!(PlatformType::Bilibili.to_string(), "bilibili");
        assert_eq!(PlatformType::Douyin.to_string(), "douyin");
        assert_eq!(PlatformType::Douyu.to_string(), "douyu");
        assert_eq!(PlatformType::Huya.to_string(), "huya");
        
        // 测试从字符串解析 PlatformType
        assert_eq!("bilibili".parse::<PlatformType>().unwrap(), PlatformType::Bilibili);
        assert_eq!("douyin".parse::<PlatformType>().unwrap(), PlatformType::Douyin);
        assert_eq!("douyu".parse::<PlatformType>().unwrap(), PlatformType::Douyu);
        assert_eq!("huya".parse::<PlatformType>().unwrap(), PlatformType::Huya);
        
        // 测试无效平台字符串
        assert!("invalid_platform".parse::<PlatformType>().is_err());
    }
    
    #[test]
    fn test_stream_quality_serialization() {
        // 测试 StreamQuality 序列化
        assert_ser_tokens(&StreamQuality::UltraHD, &[Token::UnitVariant { name: "StreamQuality", variant: "UltraHD" }]);
        assert_ser_tokens(&StreamQuality::HD, &[Token::UnitVariant { name: "StreamQuality", variant: "HD" }]);
        assert_ser_tokens(&StreamQuality::SD, &[Token::UnitVariant { name: "StreamQuality", variant: "SD" }]);
        assert_ser_tokens(&StreamQuality::Auto, &[Token::UnitVariant { name: "StreamQuality", variant: "Auto" }]);
        assert_ser_tokens(&StreamQuality::Custom("1080p".to_string()), &[
            Token::NewtypeVariant { name: "StreamQuality", variant: "Custom" },
            Token::Str("1080p")
        ]);
    }
    
    #[test]
    fn test_message_type_serialization() {
        // 测试 MessageType 序列化
        assert_ser_tokens(&MessageType::Danmaku, &[Token::UnitVariant { name: "MessageType", variant: "Danmaku" }]);
        assert_ser_tokens(&MessageType::Gift, &[Token::UnitVariant { name: "MessageType", variant: "Gift" }]);
        assert_ser_tokens(&MessageType::SuperChat, &[Token::UnitVariant { name: "MessageType", variant: "SuperChat" }]);
        assert_ser_tokens(&MessageType::EnterRoom, &[Token::UnitVariant { name: "MessageType", variant: "EnterRoom" }]);
        assert_ser_tokens(&MessageType::Follow, &[Token::UnitVariant { name: "MessageType", variant: "Follow" }]);
        assert_ser_tokens(&MessageType::Unfollow, &[Token::UnitVariant { name: "MessageType", variant: "Unfollow" }]);
        assert_ser_tokens(&MessageType::Like, &[Token::UnitVariant { name: "MessageType", variant: "Like" }]);
        assert_ser_tokens(&MessageType::Share, &[Token::UnitVariant { name: "MessageType", variant: "Share" }]);
        assert_ser_tokens(&MessageType::System, &[Token::UnitVariant { name: "MessageType", variant: "System" }]);
        assert_ser_tokens(&MessageType::GiftCombo, &[Token::UnitVariant { name: "MessageType", variant: "GiftCombo" }]);
        assert_ser_tokens(&MessageType::GuardBuy, &[Token::UnitVariant { name: "MessageType", variant: "GuardBuy" }]);
        assert_ser_tokens(&MessageType::RoomChange, &[Token::UnitVariant { name: "MessageType", variant: "RoomChange" }]);
        assert_ser_tokens(&MessageType::Other("custom_type".to_string()), &[
            Token::NewtypeVariant { name: "MessageType", variant: "Other" },
            Token::Str("custom_type")
        ]);
    }
    
    #[test]
    fn test_platform_error_handling() {
        // 测试 PlatformError 构造
        let network_err = PlatformError::Network("Connection refused".to_string());
        let parse_err = PlatformError::Parse("Invalid JSON".to_string());
        let api_err = PlatformError::Api("Internal Server Error".to_string());
        let auth_err = PlatformError::Auth("Unauthorized".to_string());
        let not_found_err = PlatformError::NotFound("Room not found".to_string());
        let platform_err = PlatformError::Platform("Platform-specific error".to_string());
        let internal_err = PlatformError::Internal("Internal error".to_string());
        let unsupported_err = PlatformError::Unsupported("Operation not supported".to_string());
        let other_err = PlatformError::Other("Other error".to_string());
        
        // 测试错误消息
        assert_eq!(network_err.to_string(), "Network error: Connection refused");
        assert_eq!(parse_err.to_string(), "Parse error: Invalid JSON");
        assert_eq!(api_err.to_string(), "API error: Internal Server Error");
        assert_eq!(auth_err.to_string(), "Authentication error: Unauthorized");
        assert_eq!(not_found_err.to_string(), "Not found: Room not found");
        assert_eq!(platform_err.to_string(), "Platform error: Platform-specific error");
        assert_eq!(internal_err.to_string(), "Internal error: Internal error");
        assert_eq!(unsupported_err.to_string(), "Unsupported operation: Operation not supported");
        assert_eq!(other_err.to_string(), "Other error: Other error");
        
        // 测试 anyhow::Error 转换
        let anyhow_err = anyhow::anyhow!("Anyhow error");
        let platform_err_from_anyhow = PlatformError::from(anyhow_err);
        assert!(matches!(platform_err_from_anyhow, PlatformError::Wrapped(_)));
        
        // 测试错误链
        let wrapped_err = PlatformError::wrap(std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"), "Test error");
        assert!(wrapped_err.full_error_message().contains("Test error"));
        assert!(wrapped_err.full_error_message().contains("File not found"));
    }
    
    #[test]
    fn test_message_construction() {
        // 测试创建 Message 对象
        let message = Message {
            id: None,
            message_type: MessageType::Danmaku,
            user: "test_user".to_string(),
            content: "Hello, world!".to_string(),
            user_level: Some(10),
            fans_level: Some(5),
            fans_club_level: None,
            badge_name: None,
            badge_level: None,
            uid: None,
            color: None,
            timestamp: 1234567890,
            room_id: "12345".to_string(),
            platform: PlatformType::Douyu,
            gift_name: None,
            gift_count: None,
            gift_price: None,
            gift_total: None,
            super_chat_price: None,
            super_chat_duration: None,
            combo_count: None,
            combo_user: None,
            raw: None,
            other: None,
        };
        
        // 测试礼物消息
        let gift_message = Message {
            id: None,
            message_type: MessageType::Gift,
            user: "test_user".to_string(),
            content: "送出了礼物".to_string(),
            user_level: Some(10),
            fans_level: Some(5),
            fans_club_level: None,
            badge_name: None,
            badge_level: None,
            uid: None,
            color: None,
            timestamp: 1234567890,
            room_id: "12345".to_string(),
            platform: PlatformType::Douyu,
            gift_name: Some("火箭".to_string()),
            gift_count: Some(1),
            gift_price: Some(100.0),
            gift_total: Some(100.0),
            super_chat_price: None,
            super_chat_duration: None,
            combo_count: None,
            combo_user: None,
            raw: None,
            other: None,
        };
        
        assert_eq!(gift_message.gift_name, Some("火箭".to_string()));
        assert_eq!(gift_message.gift_count, Some(1));
        assert_eq!(gift_message.gift_price, Some(100.0));
        assert_eq!(gift_message.gift_total, Some(100.0));
    }
    
    #[test]
    fn test_category_construction() {
        // 测试创建 Category 对象
        let category = Category {
            category_id: "1".to_string(),
            name: "游戏".to_string(),
            parent_id: None,
            icon_url: Some("https://example.com/icon.png".to_string()),
            order: Some(1),
            other: Some(serde_json::json!({"key": "value"})),
            raw: Some(serde_json::json!({"raw": "data"})),
        };
        
        assert_eq!(category.category_id, "1");
        assert_eq!(category.name, "游戏");
        assert_eq!(category.parent_id, None);
        assert_eq!(category.icon_url, Some("https://example.com/icon.png".to_string()));
        assert_eq!(category.order, Some(1));
    }
    
    #[test]
    fn test_platform_config_default() {
        // 测试 PlatformConfig 默认值
        let config = PlatformConfig::default();
        assert_eq!(config.user_agent, None);
        assert_eq!(config.cookie, None);
        assert_eq!(config.proxy, None);
        assert_eq!(config.auth_token, None);
        assert_eq!(config.with_auth, false);
        assert_eq!(config.mode, PlatformMode::Local);
        assert_eq!(config.remote_url, None);
        assert_eq!(config.other, None);
    }
}
