// 在开发模式下允许控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;
use platforms;
use platforms::shared::logger::init_logger;

// Import the platform commands wrapper module
mod platform_commands;
mod proxy;
mod watch;
use platforms::shared::{DouyinMessageState, FollowHttpClient, HuyaMessageState, BilibiliMessageState};

use tauri::Manager;

// 设置默认语言为中文
fn set_default_language() {
    // 仅在未设置 LANG 环境变量时设置默认值
    if env::var("LANG").is_err() {
        env::set_var("LANG", "zh_CN.UTF-8");
    }
}


// Main function corrected
fn main() {
    // 设置默认语言
    set_default_language();
    
    // 初始化日志系统
    init_logger().expect("Failed to initialize logger");
    
    let follow_http_client = FollowHttpClient::new().expect("Failed to create follow http client");

    // 创建菜单的代码将在 setup 函数中处理，因为需要 app 实例

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // 获取主窗口
            let _window = app.get_webview_window("main").unwrap();
            
            // 暂时不实现菜单功能，因为Tauri 2.x的菜单API变更较大
            // 后续可以根据官方文档重新实现菜单功能
            
            // 开发者工具说明
            println!("开发者工具可以通过键盘快捷键访问：");
            println!("Windows/Linux: Ctrl+Shift+I");
            println!("macOS: Cmd+Shift+I");
            
            println!("应用启动成功！");
            println!("开发者工具快捷键：Ctrl+Shift+I (Windows/Linux) 或 Cmd+Shift+I (macOS)");
            
            // Apply macOS vibrancy to the main window when running on macOS
            #[cfg(target_os = "macos")]
            {
                use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
                if let Some(window) = app.get_webview_window("main") {
                    match apply_vibrancy(&window, NSVisualEffectMaterial::HudWindow, None, None) {
                        Ok(_) => println!("vibrancy applied successfully"),
                        Err(e) => eprintln!("vibrancy error: {:?}", e),
                    }
                }
            }
            Ok(())
        })
        .manage(follow_http_client) // 专用关注刷新客户端，避免占用默认连接池
        .manage(DouyinMessageState::default()) // Manage DouyinMessageState
        .manage(HuyaMessageState::default()) // Manage HuyaMessageState
        .manage(BilibiliMessageState::default()) // Manage BilibiliMessageState
        .manage(proxy::ProxyServerHandle::default())
        .invoke_handler(tauri::generate_handler![
            // Unified platform commands (preferred)
            platform_commands::fetch_live_list,
            platform_commands::fetch_room_info,
            platform_commands::get_stream_url,
            platform_commands::fetch_streamer_info,
            platform_commands::search_rooms,
            platform_commands::fetch_categories,
            platform_commands::get_message_listener_status,  // Unified message status query
            platform_commands::send_follow_list,  // Follow list management
            
            // Proxy commands
            proxy::start_proxy,
            proxy::stop_proxy,
            proxy::start_static_proxy_server,
            
            // Watch service commands
            watch::start_follow_watch_service,
            watch::stop_follow_watch_service,
            watch::send_test_notification,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
