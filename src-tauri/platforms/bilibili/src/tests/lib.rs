use super::*;
use shared::interface::{PlatformConfig, LiveListParams, PlatformType};
use shared::logger::init_logger;

#[tokio::test]
async fn test_bilibili_platform_creation() {
    // 初始化日志系统
    init_logger().unwrap();
    
    // 测试创建 BilibiliPlatform 实例
    let config = PlatformConfig::default();
    let platform = BilibiliPlatform::new(config);
    assert!(platform.is_ok());
    
    // 测试平台类型
    let platform = platform.unwrap();
    assert_eq!(platform.platform_type(), PlatformType::Bilibili);
}

#[tokio::test]
async fn test_bilibili_fetch_live_list() {
    // 初始化日志系统
    init_logger().unwrap();
    
    // 创建平台实例
    let config = PlatformConfig::default();
    let platform = BilibiliPlatform::new(config).unwrap();
    
    // 测试获取直播列表
    let params = LiveListParams {
        category_id: Some("1".to_string()), // 游戏分区
        page: Some(1),
        page_size: Some(10),
        sort: None,
        other: None,
    };
    
    let result = platform.fetch_live_list(params).await;
    // 注意：这个测试可能会失败，因为它需要实际的网络连接
    // 在实际项目中，应该使用 mock 来模拟 HTTP 请求
    println!("fetch_live_list result: {:?}", result);
}

#[tokio::test]
async fn test_bilibili_fetch_room_info() {
    // 初始化日志系统
    init_logger().unwrap();
    
    // 创建平台实例
    let config = PlatformConfig::default();
    let platform = BilibiliPlatform::new(config).unwrap();
    
    // 测试获取直播间信息
    let room_id = "10086";
    let result = platform.fetch_room_info(room_id).await;
    // 注意：这个测试可能会失败，因为它需要实际的网络连接
    // 在实际项目中，应该使用 mock 来模拟 HTTP 请求
    println!("fetch_room_info result: {:?}", result);
}
