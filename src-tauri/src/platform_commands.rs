// Wrapper functions for platform commands from the platforms crate
// These wrapper functions allow the frontend to call commands from the platforms crate

use log::{debug, error, info};
use platforms::{PlatformType, PlatformConfig, get_global_platform_factory};
use platforms::interface::PlatformMode;
use serde::Deserialize;



// New unified commands
#[tauri::command]
#[allow(dead_code)]
pub async fn fetch_categories(
    platform: String,
    parent_id: Option<String>,
) -> Result<serde_json::Value, String> {
    info!("fetch_categories called: platform={}, parent_id={:?}", platform, parent_id);
    
    // 解析平台类型
    let platform_type: PlatformType = match platform.parse() {
        Ok(pt) => {
            debug!("Successfully parsed platform type: {:?}", pt);
            pt
        },
        Err(e) => {
            error!("Invalid platform '{}': {}", platform, e);
            return Err(format!("Invalid platform: {}", e));
        }
    };
    
    // 创建平台配置
    let config = PlatformConfig {
        cookie: None,
        user_agent: None,
        proxy: None,
        auth_token: None,
        with_auth: false,
        mode: PlatformMode::Local,
        remote_url: None,
        other: None,
    };
    debug!("Created platform config: {:?}", config);
    
    // 获取平台工厂
    let factory = get_global_platform_factory();
    debug!("Got global platform factory");
    
    // 创建平台实例
    let platform = match factory.create_platform(platform_type.clone(), config).await {
        Ok(p) => {
            debug!("Successfully created platform instance for {:?}", platform_type);
            p
        },
        Err(e) => {
            error!("Failed to create platform instance for {:?}: {}", platform_type, e);
            return Err(e.to_string());
        }
    };
    
    // 调用统一API获取分类列表
    let categories = match platform.fetch_categories(parent_id.as_deref()).await {
        Ok(cats) => {
            debug!("Successfully fetched categories for {:?}: {} items", platform_type, cats.len());
            cats
        },
        Err(e) => {
            error!("Failed to fetch categories for {:?}: {}", platform_type, e);
            return Err(e.to_string());
        }
    };
    
    // 转换为JSON返回给前端
    let json = match serde_json::to_value(categories) {
        Ok(j) => {
            debug!("Successfully serialized categories to JSON");
            j
        },
        Err(e) => {
            error!("Failed to serialize categories: {}", e);
            return Err(format!("Failed to serialize categories: {}", e));
        }
    };
    
    info!("fetch_categories completed successfully: platform={:?}", platform_type);
    Ok(json)
}

#[tauri::command]
pub async fn fetch_live_list(
    platform: String,
    category_id: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
) -> Result<serde_json::Value, String> {
    info!("fetch_live_list called: platform={}, category_id={:?}, page={:?}, page_size={:?}", 
          platform, category_id, page, page_size);
    
    // 解析平台类型
    let platform_type: PlatformType = match platform.parse() {
        Ok(pt) => {
            debug!("Successfully parsed platform type: {:?}", pt);
            pt
        },
        Err(e) => {
            error!("Invalid platform '{}': {}", platform, e);
            return Err(format!("Invalid platform: {}", e));
        }
    };
    
    // 创建平台配置
    let config = PlatformConfig {
        cookie: None,
        user_agent: None,
        proxy: None,
        auth_token: None,
        with_auth: false,
        mode: PlatformMode::Local,
        remote_url: None,
        other: None,
    };
    debug!("Created platform config: {:?}", config);
    
    // 获取平台工厂
    let factory = get_global_platform_factory();
    debug!("Got global platform factory");
    
    // 创建平台实例
    let platform = match factory.create_platform(platform_type.clone(), config).await {
        Ok(p) => {
            debug!("Successfully created platform instance for {:?}", platform_type);
            p
        },
        Err(e) => {
            error!("Failed to create platform instance for {:?}: {}", platform_type, e);
            return Err(e.to_string());
        }
    };
    
    // 构建查询参数
    let params = platforms::interface::LiveListParams {
        category_id,
        page,
        page_size,
        sort: None,
        other: None,
    };
    debug!("Built live list params: {:?}", params);
    
    // 调用统一API获取直播列表
    let live_list = match platform.fetch_live_list(params).await {
        Ok(list) => {
            debug!("Successfully fetched live list for {:?}: {} items", platform_type, list.items.len());
            list
        },
        Err(e) => {
            error!("Failed to fetch live list for {:?}: {}", platform_type, e);
            return Err(e.to_string());
        }
    };
    
    // 转换为JSON返回给前端
    let json = match serde_json::to_value(live_list) {
        Ok(j) => {
            debug!("Successfully serialized live list to JSON");
            j
        },
        Err(e) => {
            error!("Failed to serialize live list: {}", e);
            return Err(format!("Failed to serialize live list: {}", e));
        }
    };
    
    info!("fetch_live_list completed successfully: platform={:?}", platform_type);
    Ok(json)
}

#[tauri::command]
pub async fn fetch_room_info(
    platform: String,
    room_id: String,
) -> Result<serde_json::Value, String> {
    info!("fetch_room_info called: platform={}, room_id={}", platform, room_id);
    
    // 解析平台类型
    let platform_type: PlatformType = match platform.parse() {
        Ok(pt) => {
            debug!("Successfully parsed platform type: {:?}", pt);
            pt
        },
        Err(e) => {
            error!("Invalid platform '{}': {}", platform, e);
            return Err(format!("Invalid platform: {}", e));
        }
    };
    
    // 创建平台配置
    let config = PlatformConfig {
        cookie: None,
        user_agent: None,
        proxy: None,
        auth_token: None,
        with_auth: false,
        mode: PlatformMode::Local,
        remote_url: None,
        other: None,
    };
    debug!("Created platform config: {:?}", config);
    
    // 获取平台工厂
    let factory = get_global_platform_factory();
    debug!("Got global platform factory");
    
    // 创建平台实例
    let platform = match factory.create_platform(platform_type.clone(), config).await {
        Ok(p) => {
            debug!("Successfully created platform instance for {:?}", platform_type);
            p
        },
        Err(e) => {
            error!("Failed to create platform instance for {:?}: {}", platform_type, e);
            return Err(e.to_string());
        }
    };
    
    // 调用统一API获取直播间信息
    let room_info = match platform.fetch_room_info(&room_id).await {
        Ok(info) => {
            debug!("Successfully fetched room info for room_id={}: {:?}", room_id, info);
            info
        },
        Err(e) => {
            error!("Failed to fetch room info for room_id={}: {}", room_id, e);
            return Err(e.to_string());
        }
    };
    
    // 转换为JSON返回给前端
    let json = match serde_json::to_value(room_info) {
        Ok(j) => {
            debug!("Successfully serialized room info to JSON");
            j
        },
        Err(e) => {
            error!("Failed to serialize room info: {}", e);
            return Err(format!("Failed to serialize room info: {}", e));
        }
    };
    
    info!("fetch_room_info completed successfully: platform={:?}, room_id={}", platform_type, room_id);
    Ok(json)
}

#[tauri::command]
pub async fn get_stream_url(
    platform: String,
    room_id: String,
    stream_quality: Option<String>,
) -> Result<serde_json::Value, String> {
    info!("get_stream_url called: platform={}, room_id={}, stream_quality={:?}", platform, room_id, stream_quality);
    
    // 解析平台类型
    let platform_type: PlatformType = match platform.parse() {
        Ok(pt) => {
            debug!("Successfully parsed platform type: {:?}", pt);
            pt
        },
        Err(e) => {
            error!("Invalid platform '{}': {}", platform, e);
            return Err(format!("Invalid platform: {}", e));
        }
    };
    
    // 解析直播流质量
    let stream_quality = stream_quality.map(|q| {
        match q.as_str() {
            "UltraHD" => platforms::interface::StreamQuality::UltraHD,
            "HD" => platforms::interface::StreamQuality::HD,
            "SD" => platforms::interface::StreamQuality::SD,
            "Auto" => platforms::interface::StreamQuality::Auto,
            _ => platforms::interface::StreamQuality::Custom(q),
        }
    });
    debug!("Parsed stream quality: {:?}", stream_quality);
    
    // 创建平台配置
    let config = PlatformConfig {
        cookie: None,
        user_agent: None,
        proxy: None,
        auth_token: None,
        with_auth: false,
        mode: PlatformMode::Local,
        remote_url: None,
        other: None,
    };
    debug!("Created platform config: {:?}", config);
    
    // 获取平台工厂
    let factory = get_global_platform_factory();
    debug!("Got global platform factory");
    
    // 创建平台实例
    let platform = match factory.create_platform(platform_type.clone(), config).await {
        Ok(p) => {
            debug!("Successfully created platform instance for {:?}", platform_type);
            p
        },
        Err(e) => {
            error!("Failed to create platform instance for {:?}: {}", platform_type, e);
            return Err(e.to_string());
        }
    };
    
    // 调用统一API获取直播流URL
    let stream_url = match platform.get_stream_url(&room_id, stream_quality).await {
        Ok(url) => {
            debug!("Successfully fetched stream URL for {:?} room {}: primary_url={:?}", 
                   platform_type, room_id, url.primary_url);
            url
        },
        Err(e) => {
            error!("Failed to fetch stream URL for {:?} room {}: {}", platform_type, room_id, e);
            return Err(e.to_string());
        }
    };
    
    // 转换为JSON返回给前端
    let json = match serde_json::to_value(stream_url) {
        Ok(j) => {
            debug!("Successfully serialized stream URL to JSON");
            j
        },
        Err(e) => {
            error!("Failed to serialize stream URL: {}", e);
            return Err(format!("Failed to serialize stream URL: {}", e));
        }
    };
    
    info!("get_stream_url completed successfully: platform={:?}, room_id={}", platform_type, room_id);
    Ok(json)
}

#[tauri::command]
pub async fn search_rooms(
    platform: String,
    keyword: String,
    page: Option<u32>,
    page_size: Option<u32>,
) -> Result<serde_json::Value, String> {
    info!("search_rooms called: platform={}, keyword={}, page={:?}, page_size={:?}", 
          platform, keyword, page, page_size);
    
    // 解析平台类型
    let platform_type: PlatformType = match platform.parse() {
        Ok(pt) => {
            debug!("Successfully parsed platform type: {:?}", pt);
            pt
        },
        Err(e) => {
            error!("Invalid platform '{}': {}", platform, e);
            return Err(format!("Invalid platform: {}", e));
        }
    };
    
    // 创建平台配置
    let config = PlatformConfig {
        cookie: None,
        user_agent: None,
        proxy: None,
        auth_token: None,
        with_auth: false,
        mode: PlatformMode::Local,
        remote_url: None,
        other: None,
    };
    debug!("Created platform config: {:?}", config);
    
    // 获取平台工厂
    let factory = get_global_platform_factory();
    debug!("Got global platform factory");
    
    // 创建平台实例
    let platform = match factory.create_platform(platform_type.clone(), config).await {
        Ok(p) => {
            debug!("Successfully created platform instance for {:?}", platform_type);
            p
        },
        Err(e) => {
            error!("Failed to create platform instance for {:?}: {}", platform_type, e);
            return Err(e.to_string());
        }
    };
    
    // 调用统一API搜索直播间
    let live_list = match platform.search_rooms(&keyword, page, page_size).await {
        Ok(list) => {
            debug!("Successfully searched rooms for {:?} with keyword '{}': {} items", 
                   platform_type, keyword, list.items.len());
            list
        },
        Err(e) => {
            error!("Failed to search rooms for {:?} with keyword '{}': {}", platform_type, keyword, e);
            return Err(e.to_string());
        }
    };
    
    // 转换为JSON返回给前端
    let json = match serde_json::to_value(live_list) {
        Ok(j) => {
            debug!("Successfully serialized search results to JSON");
            j
        },
        Err(e) => {
            error!("Failed to serialize search results: {}", e);
            return Err(format!("Failed to serialize search results: {}", e));
        }
    };
    
    info!("search_rooms completed successfully: platform={:?}, keyword={}", platform_type, keyword);
    Ok(json)
}

#[tauri::command]
pub async fn fetch_streamer_info(
    platform: String,
    streamer_id: String,
) -> Result<serde_json::Value, String> {
    info!("fetch_streamer_info called: platform={}, streamer_id={}", platform, streamer_id);
    
    // 解析平台类型
    let platform_type: PlatformType = match platform.parse() {
        Ok(pt) => {
            debug!("Successfully parsed platform type: {:?}", pt);
            pt
        },
        Err(e) => {
            error!("Invalid platform '{}': {}", platform, e);
            return Err(format!("Invalid platform: {}", e));
        }
    };
    
    // 创建平台配置
    let config = PlatformConfig {
        cookie: None,
        user_agent: None,
        proxy: None,
        auth_token: None,
        with_auth: false,
        mode: PlatformMode::Local,
        remote_url: None,
        other: None,
    };
    debug!("Created platform config: {:?}", config);
    
    // 获取平台工厂
    let factory = get_global_platform_factory();
    debug!("Got global platform factory");
    
    // 创建平台实例
    let platform = match factory.create_platform(platform_type.clone(), config).await {
        Ok(p) => {
            debug!("Successfully created platform instance for {:?}", platform_type);
            p
        },
        Err(e) => {
            error!("Failed to create platform instance for {:?}: {}", platform_type, e);
            return Err(e.to_string());
        }
    };
    
    // 调用统一API获取主播信息
    let streamer_info = match platform.fetch_streamer_info(&streamer_id).await {
        Ok(info) => {
            debug!("Successfully fetched streamer info for {:?} streamer {}: {:?}", platform_type, streamer_id, info.name);
            info
        },
        Err(e) => {
            error!("Failed to fetch streamer info for {:?} streamer {}: {}", platform_type, streamer_id, e);
            return Err(e.to_string());
        }
    };
    
    // 转换为JSON返回给前端
    let json = match serde_json::to_value(streamer_info) {
        Ok(j) => {
            debug!("Successfully serialized streamer info to JSON");
            j
        },
        Err(e) => {
            error!("Failed to serialize streamer info: {}", e);
            return Err(format!("Failed to serialize streamer info: {}", e));
        }
    };
    
    info!("fetch_streamer_info completed successfully: platform={:?}, streamer_id={}", platform_type, streamer_id);
    Ok(json)
}

// 添加消息监听器状态查询接口
#[tauri::command]
pub async fn get_message_listener_status(
    platform: String,
    room_id: String,
) -> Result<serde_json::Value, String> {
    info!("get_message_listener_status called: platform={}, room_id={}", platform, room_id);
    
    // 解析平台类型
    let platform_type: PlatformType = match platform.parse() {
        Ok(pt) => {
            debug!("Successfully parsed platform type: {:?}", pt);
            pt
        },
        Err(e) => {
            error!("Invalid platform '{}': {}", platform, e);
            return Err(format!("Invalid platform: {}", e));
        }
    };
    
    // 使用消息管理器获取实际的监听器状态
    let message_manager = platforms::message_manager::MessageManager::get_global_manager();
    let statuses = message_manager.get_listener_status(Some(platform_type), Some(&room_id)).await;
    
    // 如果找到状态，返回实际状态，否则返回UNKNOWN状态
    if let Some(status) = statuses.first() {
        let listener_status = serde_json::json!({"platform": platform,
            "room_id": room_id,
            "status": status.status,
            "message_count": status.message_count,
            "last_update": status.last_update
        });
        Ok(listener_status)
    } else {
        // 如果没有找到状态，返回UNKNOWN状态
        let listener_status = serde_json::json!({"platform": platform,
            "room_id": room_id,
            "status": "UNKNOWN",
            "message_count": 0,
            "last_update": 0
        });
        Ok(listener_status)
    }
}

// -------------------- 新增认证相关命令 --------------------

#[tauri::command]
#[allow(dead_code)]
pub async fn get_auth_url(
    platform: String,
) -> Result<String, String> {
    info!("get_auth_url called: platform={}", platform);
    
    // 解析平台类型
    let platform_type: PlatformType = match platform.parse() {
        Ok(pt) => {
            debug!("Successfully parsed platform type: {:?}", pt);
            pt
        },
        Err(e) => {
            error!("Invalid platform '{}': {}", platform, e);
            return Err(format!("Invalid platform: {}", e));
        }
    };
    
    // 创建平台配置
    let config = PlatformConfig {
        cookie: None,
        user_agent: None,
        proxy: None,
        auth_token: None,
        with_auth: false,
        mode: PlatformMode::Local,
        remote_url: None,
        other: None,
    };
    debug!("Created platform config: {:?}", config);
    
    // 获取平台工厂
    let factory = get_global_platform_factory();
    debug!("Got global platform factory");
    
    // 创建平台实例
    let platform = match factory.create_platform(platform_type.clone(), config).await {
        Ok(p) => {
            debug!("Successfully created platform instance for {:?}", platform_type);
            p
        },
        Err(e) => {
            error!("Failed to create platform instance for {:?}: {}", platform_type, e);
            return Err(e.to_string());
        }
    };
    
    // 调用统一API获取认证URL
    match platform.get_auth_url().await {
        Ok(url) => {
            info!("Successfully got auth URL for {:?}: {:?}", platform_type, url);
            Ok(url)
        },
        Err(e) => {
            error!("Failed to get auth URL for {:?}: {}", platform_type, e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
#[allow(dead_code)]
pub async fn login_with_code(
    platform: String,
    code: String,
) -> Result<serde_json::Value, String> {
    info!("login_with_code called: platform={}, code={}", platform, code);
    
    // 解析平台类型
    let platform_type: PlatformType = match platform.parse() {
        Ok(pt) => {
            debug!("Successfully parsed platform type: {:?}", pt);
            pt
        },
        Err(e) => {
            error!("Invalid platform '{}': {}", platform, e);
            return Err(format!("Invalid platform: {}", e));
        }
    };
    
    // 创建平台配置
    let config = PlatformConfig {
        cookie: None,
        user_agent: None,
        proxy: None,
        auth_token: None,
        with_auth: false,
        mode: PlatformMode::Local,
        remote_url: None,
        other: None,
    };
    debug!("Created platform config: {:?}", config);
    
    // 获取平台工厂
    let factory = get_global_platform_factory();
    debug!("Got global platform factory");
    
    // 创建平台实例
    let platform = match factory.create_platform(platform_type.clone(), config).await {
        Ok(p) => {
            debug!("Successfully created platform instance for {:?}", platform_type);
            p
        },
        Err(e) => {
            error!("Failed to create platform instance for {:?}: {}", platform_type, e);
            return Err(e.to_string());
        }
    };
    
    // 调用统一API使用认证码登录
    let response = match platform.login_with_code(&code).await {
        Ok(res) => {
            debug!("Successfully logged in with code for {:?}", platform_type);
            res
        },
        Err(e) => {
            error!("Failed to login with code for {:?}: {}", platform_type, e);
            return Err(e.to_string());
        }
    };
    
    // 转换为JSON返回给前端
    let json = match serde_json::to_value(response) {
        Ok(j) => {
            debug!("Successfully serialized login response to JSON");
            j
        },
        Err(e) => {
            error!("Failed to serialize login response: {}", e);
            return Err(format!("Failed to serialize login response: {}", e));
        }
    };
    
    info!("login_with_code completed successfully: platform={:?}", platform_type);
    Ok(json)
}

#[tauri::command]
#[allow(dead_code)]
pub async fn refresh_token(
    platform: String,
    _refresh_token: String,
) -> Result<serde_json::Value, String> {
    info!("refresh_token called: platform={}, refresh_token={}", platform, _refresh_token);
    
    // 解析平台类型
    let platform_type: PlatformType = match platform.parse() {
        Ok(pt) => {
            debug!("Successfully parsed platform type: {:?}", pt);
            pt
        },
        Err(e) => {
            error!("Invalid platform '{}': {}", platform, e);
            return Err(format!("Invalid platform: {}", e));
        }
    };
    
    // 创建平台配置
    let config = PlatformConfig {
        cookie: None,
        user_agent: None,
        proxy: None,
        auth_token: None,
        with_auth: false,
        mode: PlatformMode::Local,
        remote_url: None,
        other: None,
    };
    debug!("Created platform config: {:?}", config);
    
    // 获取平台工厂
    let factory = get_global_platform_factory();
    debug!("Got global platform factory");
    
    // 创建平台实例
    let platform = match factory.create_platform(platform_type.clone(), config).await {
        Ok(p) => {
            debug!("Successfully created platform instance for {:?}", platform_type);
            p
        },
        Err(e) => {
            error!("Failed to create platform instance for {:?}: {}", platform_type, e);
            return Err(e.to_string());
        }
    };
    
    // 调用统一API刷新令牌
    let response = match platform.refresh_token().await {
        Ok(res) => {
            debug!("Successfully refreshed token for {:?}", platform_type);
            res
        },
        Err(e) => {
            error!("Failed to refresh token for {:?}: {}", platform_type, e);
            return Err(e.to_string());
        }
    };
    
    // 转换为JSON返回给前端
    let json = match serde_json::to_value(response) {
        Ok(j) => {
            debug!("Successfully serialized refresh token response to JSON");
            j
        },
        Err(e) => {
            error!("Failed to serialize refresh token response: {}", e);
            return Err(format!("Failed to serialize refresh token response: {}", e));
        }
    };
    
    info!("refresh_token completed successfully: platform={:?}", platform_type);
    Ok(json)
}

#[tauri::command]
#[allow(dead_code)]
pub async fn get_current_user(
    platform: String,
) -> Result<serde_json::Value, String> {
    info!("get_current_user called: platform={}", platform);
    
    // 解析平台类型
    let platform_type: PlatformType = match platform.parse() {
        Ok(pt) => {
            debug!("Successfully parsed platform type: {:?}", pt);
            pt
        },
        Err(e) => {
            error!("Invalid platform '{}': {}", platform, e);
            return Err(format!("Invalid platform: {}", e));
        }
    };
    
    // 创建平台配置
    let config = PlatformConfig {
        cookie: None,
        user_agent: None,
        proxy: None,
        auth_token: None,
        with_auth: false,
        mode: PlatformMode::Local,
        remote_url: None,
        other: None,
    };
    debug!("Created platform config: {:?}", config);
    
    // 获取平台工厂
    let factory = get_global_platform_factory();
    debug!("Got global platform factory");
    
    // 创建平台实例
    let platform = match factory.create_platform(platform_type.clone(), config).await {
        Ok(p) => {
            debug!("Successfully created platform instance for {:?}", platform_type);
            p
        },
        Err(e) => {
            error!("Failed to create platform instance for {:?}: {}", platform_type, e);
            return Err(e.to_string());
        }
    };
    
    // 调用统一API获取当前用户
    let response = match platform.get_current_user().await {
        Ok(res) => {
            debug!("Successfully got current user for {:?}", platform_type);
            res
        },
        Err(e) => {
            error!("Failed to get current user for {:?}: {}", platform_type, e);
            return Err(e.to_string());
        }
    };
    
    // 转换为JSON返回给前端
    let json = match serde_json::to_value(response) {
        Ok(j) => {
            debug!("Successfully serialized current user response to JSON");
            j
        },
        Err(e) => {
            error!("Failed to serialize current user response: {}", e);
            return Err(format!("Failed to serialize current user response: {}", e));
        }
    };
    
    info!("get_current_user completed successfully: platform={:?}", platform_type);
    Ok(json)
}

// -------------------- 关注列表相关命令 --------------------

#[derive(Debug, Deserialize)]
pub struct FrontendFollowedStreamer {
    pub platform: String,
    pub room_id: String,
    pub streamer_id: String,
    pub nickname: String,
    pub avatar_url: String,
}

#[tauri::command]
pub async fn send_follow_list(
    follow_list: Vec<FrontendFollowedStreamer>,
) -> Result<(), String> {
    info!("send_follow_list called with {} streamers", follow_list.len());
    
    // 转换前端数据格式为watch模块需要的格式
    let platform_streamers = follow_list.into_iter()
        .map(|streamer| {
            let platform_type = match streamer.platform.parse() {
                Ok(pt) => pt,
                Err(e) => {
                    error!("Invalid platform '{}' in follow list: {}", streamer.platform, e);
                    return Err(format!("Invalid platform: {}", e));
                }
            };
            
            Ok(platforms::watch::FollowStreamerInput {
                platform: platform_type,
                id: streamer.streamer_id,
                nickname: Some(streamer.nickname),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    
    // 创建新的watch配置
    let watch_config = platforms::watch::WatchConfig {
        streamers: platform_streamers,
        interval_ms: 5000,
        enable_notification: true,
    };
    
    // 保存关注列表到文件
    platforms::watch::save_follow_list(&watch_config)?;
    
    info!("Follow list saved successfully");
    Ok(())
}











