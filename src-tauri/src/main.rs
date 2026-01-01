// 在开发模式下允许控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use reqwest;
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;
use platforms;

// Import the platform commands wrapper module
mod platform_commands;
mod proxy;
mod watch;
use platforms::shared::{DouyinDanmakuState, FollowHttpClient, HuyaDanmakuState};
// use platforms::huya::get_huya_stream_url_with_quality; // removed in favor of unified cmd

use tauri::Manager;

// 设置默认语言为中文
fn set_default_language() {
    // 仅在未设置 LANG 环境变量时设置默认值
    if env::var("LANG").is_err() {
        env::set_var("LANG", "zh_CN.UTF-8");
    }
}

#[derive(Default, Clone)]
pub struct StreamUrlStore {
    pub url: Arc<Mutex<String>>,
}

// State for managing Douyu danmaku listener handles (stop signals)
#[derive(Default, Clone)]
pub struct DouyuDanmakuHandles(Arc<Mutex<HashMap<String, oneshot::Sender<()>>>>);

#[tauri::command]
async fn get_stream_url_cmd(room_id: String) -> Result<String, String> {
    // Call the actual function to fetch the stream URL from the new location
    platforms::douyu::get_stream_url(&room_id, None)
        .await
        .map_err(|e| {
            eprintln!(
                "[Rust Error] Failed to get stream URL for room {}: {}",
                room_id,
                e.to_string()
            );
            format!("Failed to get stream URL: {}", e.to_string())
        })
}

#[tauri::command]
async fn get_stream_url_with_quality_cmd(
    room_id: String,
    quality: String,
    line: Option<String>,
) -> Result<String, String> {
    platforms::douyu::get_stream_url_with_quality(&room_id, &quality, line.as_deref())
        .await
        .map_err(|e| {
            eprintln!(
                "[Rust Error] Failed to get stream URL with quality {} for room {}: {}",
                quality,
                room_id,
                e.to_string()
            );
            format!("Failed to get stream URL with quality: {}", e.to_string())
        })
}

// Legacy Huya stream URL command removed in favor of unified command

// This is the command that should be used for setting stream URL if it interacts with StreamUrlStore
#[tauri::command]
async fn set_stream_url_cmd(
    url: String,
    state: tauri::State<'_, StreamUrlStore>,
) -> Result<(), String> {
    let mut current_url = state.url.lock().unwrap();
    *current_url = url;
    Ok(())
}

// Command to start Douyu danmaku listener
#[tauri::command]
async fn start_danmaku_listener(
    room_id: String,
    window: tauri::Window,
    danmaku_handles: tauri::State<'_, DouyuDanmakuHandles>,
) -> Result<(), String> {
    // If a listener for this room_id already exists, stop it first.
    if let Some(existing_sender) = danmaku_handles.0.lock().unwrap().remove(&room_id) {
        let _ = existing_sender.send(());
    }

    let (stop_tx, stop_rx) = oneshot::channel();
    danmaku_handles
        .0
        .lock()
        .unwrap()
        .insert(room_id.clone(), stop_tx);

    let window_clone = window.clone();
    let room_id_clone = room_id.clone();
    tokio::spawn(async move {
        let mut client = platforms::douyu::danmu_start::DanmakuClient::new(
            &room_id_clone,
            window_clone,
            stop_rx, // Pass the receiver part of the oneshot channel
        );
        if let Err(e) = client.start().await {
            eprintln!(
                "[Rust Main] Douyu danmaku client for room {} failed: {}",
                room_id_clone, e
            );
        }
    });

    Ok(())
}

// Command to stop Douyu danmaku listener
#[tauri::command]
async fn stop_danmaku_listener(
    room_id: String,
    danmaku_handles: tauri::State<'_, DouyuDanmakuHandles>,
) -> Result<(), String> {
    if let Some(sender) = danmaku_handles.0.lock().unwrap().remove(&room_id) {
        match sender.send(()) {
            Ok(_) => Ok(()),
            Err(_) => Err(format!(
                "Failed to stop Douyu danmaku listener for room {}: receiver dropped.",
                room_id
            )),
        }
    } else {
        Ok(())
    }
}

// search_anchor seems fine, assuming douyu::search_anchor is correct
#[tauri::command]
async fn search_anchor(keyword: String) -> Result<String, String> {
    platforms::douyu::perform_anchor_search(&keyword)
        .await
        .map_err(|e| e.to_string())
}

// Main function corrected
fn main() {
    // 设置默认语言
    set_default_language();
    // Create a new HTTP client instance to be managed by Tauri
    let client = reqwest::Client::builder()
        // .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .no_proxy()
        .build()
        .expect("Failed to create reqwest client");
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
        .manage(client) // Manage the reqwest client
        .manage(follow_http_client) // 专用关注刷新客户端，避免占用默认连接池
        .manage(DouyuDanmakuHandles::default()) // Manage new DouyuDanmakuHandles
        .manage(DouyinDanmakuState::default()) // Manage DouyinDanmakuState
        .manage(HuyaDanmakuState::default()) // Manage HuyaDanmakuState
        .manage(platforms::shared::BilibiliDanmakuState::default()) // Manage BilibiliDanmakuState
        .manage(StreamUrlStore::default())
        .manage(proxy::ProxyServerHandle::default())
        .manage(platforms::bilibili::state::BilibiliState::default())
        .manage(watch::FollowWatchState::default())
        .invoke_handler(tauri::generate_handler![
            // Douyu legacy commands
            get_stream_url_cmd,
            get_stream_url_with_quality_cmd,
            set_stream_url_cmd,
            search_anchor,
            start_danmaku_listener,      // Douyu danmaku start
            stop_danmaku_listener,       // Douyu danmaku stop
            
            // Platform commands (wrapped from platforms crate)
            // Only include commands that are actually used by the frontend
            // Douyin commands - these are the ones causing errors in the frontend
            platform_commands::fetch_douyin_streamer_info,
            platform_commands::get_douyin_live_stream_url_with_quality,
            platform_commands::start_douyin_danmu_listener,
            
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
