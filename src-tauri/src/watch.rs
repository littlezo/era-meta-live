// This file provides a Tauri wrapper for the platforms::watch module
use tauri::{command, AppHandle, Emitter, State};
use platforms::shared::FollowHttpClient;
use platforms::watch::{self, WatchConfig};
use platforms::PlatformType;

// Tauri EventEmitter wrapper that implements the watch::EventEmitter trait
struct TauriEventEmitter {
    app: AppHandle,
}

impl watch::EventEmitter for TauriEventEmitter {
    fn emit_status_update(&self, update: watch::FollowStatusUpdate) {
        let _ = self.app.emit("follow_status_update", update);
    }
    
    fn emit_notification(&self, notification: watch::NotificationEvent) {
        let _ = self.app.emit("notify", serde_json::json!({ 
            "title": notification.title, 
            "body": notification.body 
        }));
    }
    
    fn emit_message(&self, platform: PlatformType, room_id: String, message: serde_json::Value) {
        let _ = self.app.emit("message", serde_json::json!({ 
            "platform": platform.to_string(), 
            "room_id": room_id, 
            "message": message 
        }));
    }
}

#[command]
pub async fn start_follow_watch_service(
    app: AppHandle,
    follow_http: State<'_, FollowHttpClient>,
    config: WatchConfig,
) -> Result<(), String> {
    // Create a Tauri event emitter wrapper
    let emitter = TauriEventEmitter { app };
    
    // Use the platforms::watch implementation
    watch::start_follow_watch_service(
        std::sync::Arc::new(emitter),
        std::sync::Arc::new((*follow_http).clone()),
        std::sync::Arc::new(watch::FollowWatchState::default()),
        config
    ).await
}

#[command]
pub async fn stop_follow_watch_service() -> Result<(), String> {
    // Use the platforms::watch implementation with a new state
    watch::stop_follow_watch_service(std::sync::Arc::new(watch::FollowWatchState::default())).await
}

#[command]
pub async fn send_test_notification(app: AppHandle) -> Result<(), String> {
    let _ = app.emit("notify", serde_json::json!({
        "title": "测试通知",
        "body": "这是一条测试通知，如果您能看到这条消息，说明通知功能正常工作。"
    }));
    Ok(())
}
