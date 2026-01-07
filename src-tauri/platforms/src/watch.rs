use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::{mpsc, Mutex};
use shared::interface::{PlatformType, PlatformConfig, MessageListener, Message, ListenerStatus as SharedListenerStatus};
use shared::FollowHttpClient;
use crate::factory::get_global_platform_factory;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowStreamerInput {
    pub platform: PlatformType,
    pub id: String,
    pub nickname: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchConfig {
    pub streamers: Vec<FollowStreamerInput>,
    pub interval_ms: u64,
    pub enable_notification: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowStatusUpdate {
    pub platform: PlatformType,
    pub id: String,
    pub live_status: String, // "LIVE" | "OFFLINE" | "REPLAY" | "UNKNOWN"
    pub nickname: Option<String>,
    pub room_title: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationEvent {
    pub title: String,
    pub body: String,
}

// Event emitter trait to replace tauri::Emitter
pub trait EventEmitter {
    fn emit_status_update(&self, update: FollowStatusUpdate);
    fn emit_notification(&self, notification: NotificationEvent);
    fn emit_message(&self, platform: PlatformType, room_id: String, message: serde_json::Value);
}

#[derive(Default)]
pub struct FollowWatchState {
    pub task_handle: StdMutex<Option<tokio::task::JoinHandle<()>>>,
    pub shutdown_tx: StdMutex<Option<mpsc::Sender<()>>>,
    pub last_status: Arc<Mutex<HashMap<String, String>>>, // key: "<platform>:<id>", value: live_status
    pub current_config: StdMutex<Option<WatchConfig>>,
    pub message_listeners: Arc<Mutex<HashMap<String, SharedListenerStatus>>>, // key: "<platform>:<id>"
    pub message_listener_instances: Arc<Mutex<HashMap<String, Box<dyn MessageListener>>>>, // key: "<platform>:<id>", value: listener instance
    pub listener_shutdowns: Arc<Mutex<HashMap<String, mpsc::Sender<()>>>>, // key: "<platform>:<id>", value: shutdown channel
}

fn key_of(s: &FollowStreamerInput) -> String {
    format!("{0}:{1}", s.platform.to_string(), s.id)
}

// Follow list persistence functions
use std::env;



pub fn get_follow_list_path() -> Result<std::path::PathBuf, String> {
    // Get the application data directory
    let app_dir = if cfg!(target_os = "windows") {
        env::var("APPDATA")
            .map_err(|e| format!("Failed to get APPDATA: {}", e))?
            .parse()
            .map_err(|e| format!("Failed to parse APPDATA: {}", e))
    } else if cfg!(target_os = "macos") {
        Ok(Path::new(&env::var("HOME").map_err(|e| e.to_string())?) 
            .join("Library") 
            .join("Application Support") 
            .join("era-meta-live"))
    } else {
        Ok(Path::new(&env::var("HOME").map_err(|e| e.to_string())?) 
            .join(".config") 
            .join("era-meta-live"))
    }?;
    
    // Create the directory if it doesn't exist
    if !app_dir.exists() {
        std::fs::create_dir_all(&app_dir).map_err(|e| format!("Failed to create app directory: {}", e))?;
    }
    
    Ok(app_dir.join("follow_list.json"))
}

pub fn load_follow_list() -> Result<WatchConfig, String> {
    let path = get_follow_list_path()?;
    if !path.exists() {
        return Ok(WatchConfig {
            streamers: Vec::new(),
            interval_ms: 5000,
            enable_notification: true,
        });
    }
    
    let mut file = File::open(&path).map_err(|e| format!("Failed to open follow list file: {}", e))?;
    let mut content = String::new();
    file.read_to_string(&mut content).map_err(|e| format!("Failed to read follow list file: {}", e))?;
    
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse follow list: {}", e))
}

pub fn save_follow_list(config: &WatchConfig) -> Result<(), String> {
    let path = get_follow_list_path()?;
    let content = serde_json::to_string_pretty(config).map_err(|e| format!("Failed to serialize follow list: {}", e))?;
    let mut file = File::create(&path).map_err(|e| format!("Failed to create follow list file: {}", e))?;
    file.write_all(content.as_bytes()).map_err(|e| format!("Failed to write follow list file: {}", e))
}

// Start follow watch service - no tauri dependencies
pub async fn start_follow_watch_service<T: EventEmitter + Send + Sync + 'static>(
    emitter: Arc<T>,
    _follow_http: Arc<FollowHttpClient>,
    state: Arc<FollowWatchState>,
    config: WatchConfig,
) -> Result<(), String> {
    // Save config to local file
    save_follow_list(&config)?;
    
    let (tx, mut rx) = mpsc::channel::<()>(1);

    // stop existing
    if let Some(prev) = state.task_handle.lock().unwrap().take() {
        prev.abort();
    }
    if let Some(prev_tx) = state.shutdown_tx.lock().unwrap().take() {
        let _ = prev_tx.try_send(());
    }
    *state.shutdown_tx.lock().unwrap() = Some(tx.clone());
    *state.current_config.lock().unwrap() = Some(config.clone());

    let emitter_clone = emitter.clone();
    let status_map = state.last_status.clone();

    let handle = tokio::spawn(async move {
        // Get platform factory
        let factory = get_global_platform_factory();
        
        loop {
            // shutdown check
            if let Ok(_) = rx.try_recv() {
                break;
            }
            // read config snapshot
            let cfg = config.clone();
            for s in cfg.streamers.iter() {
                let platform = &s.platform;
                let id = s.id.clone();
                let mut live_status = "UNKNOWN".to_string();
                let mut nickname: Option<String> = s.nickname.clone();
                let mut room_title: Option<String> = None;
                let mut avatar_url: Option<String> = None;

                // Create platform config
                let platform_config = PlatformConfig::default();
                
                // Get platform instance
                match factory.get_platform(platform.clone(), platform_config).await {
                    Ok(platform_instance) => {
                        // Get room info using unified interface
                        match platform_instance.fetch_room_info(&id).await {
                            Ok(room_info) => {
                                live_status = if room_info.live_status {
                                    if room_info.live_status_detail == "REPLAY" {
                                        "REPLAY".to_string()
                                    } else {
                                        "LIVE".to_string()
                                    }
                                } else {
                                    "OFFLINE".to_string()
                                };
                                
                                nickname = Some(room_info.streamer_name.clone()).or(nickname);
                                room_title = Some(room_info.title.clone());
                                avatar_url = room_info.avatar_url.clone();
                            }
                            Err(e) => {
                                live_status = "UNKNOWN".to_string();
                                eprintln!("[FollowWatch] Failed to fetch room info for {}:{}: {:?}", 
                                    platform, id, e);
                            }
                        }
                    }
                    Err(e) => {
                        live_status = "UNKNOWN".to_string();
                        eprintln!("[FollowWatch] Failed to get platform instance for {}: {:?}", 
                            platform, e);
                    }
                }

                // emit update event
                let update = FollowStatusUpdate {
                    platform: platform.clone(),
                    id: id.clone(),
                    live_status: live_status.clone(),
                    nickname: nickname.clone(),
                    room_title: room_title.clone(),
                    avatar_url: avatar_url.clone(),
                };
                emitter_clone.emit_status_update(update.clone());

                // notification on LIVE transition
                if cfg.enable_notification {
                    let key = key_of(s);
                    let mut map = status_map.lock().await;
                    let prev = map.get(&key).cloned().unwrap_or_default();
                    if prev != "LIVE" && live_status == "LIVE" {
                        let title = "开播提醒";
                        let body = format!(
                            "{0} 开播了！\n{1}",
                            nickname.clone().unwrap_or_else(|| id.clone()),
                            room_title.clone().unwrap_or_default()
                        );
                        emitter_clone.emit_notification(NotificationEvent {
                            title: title.to_string(),
                            body: body.to_string(),
                        });
                    }
                    map.insert(key, live_status.clone());
                }
                
                // Message listener management: Start/Stop based on live status using MessageManager
                let is_live = live_status == "LIVE";
                
                // Get the global message manager
                let message_manager = crate::message_manager::MessageManager::get_global_manager();
                
                // Check if a listener already exists for this streamer
                let has_active_listener = message_manager.has_active_listener(platform.clone(), &id).await;
                
                if is_live && !has_active_listener {
                    // Start message listener
                    println!("[FollowWatch] Starting message listener for {}:{} (live_status: {})
", 
                        platform.to_string(), id, live_status);
                    
                    // Create a message callback that emits to our event system and processes with message manager
                    let emitter_clone = emitter_clone.clone();
                    let callback = Box::new(move |message: Message| {
                        // Process message with message manager (updates counts, broadcasts to subscribers)
                        let message_manager = crate::message_manager::MessageManager::get_global_manager();
                        let message_clone = message.clone();
                        
                        // Spawn a task to process the message to avoid blocking the callback
                        tokio::spawn(async move {
                            message_manager.process_message(message_clone).await;
                        });
                        
                        // Convert to our internal message format and emit
                        let json_message = serde_json::to_value(&message).unwrap_or_default();
                        emitter_clone.emit_message(message.platform, message.room_id.clone(), json_message);
                    });
                    
                    // Start the message listener using the message manager
                    match message_manager.start_message_listener(platform.clone(), &id, callback).await {
                        Ok(_) => {
                            println!("[FollowWatch] Successfully started message listener for {}:{}
", 
                                platform.to_string(), id);
                        },
                        Err(e) => {
                            eprintln!("[FollowWatch] Failed to start message listener for {}:{}: {}
", 
                                platform.to_string(), id, e);
                        }
                    }
                } else if !is_live && has_active_listener {
                    // Stop message listener
                    println!("[FollowWatch] Stopping message listener for {}:{} (live_status: {})
", 
                        platform.to_string(), id, live_status);
                    
                    match message_manager.stop_message_listener(platform.clone(), &id).await {
                        Ok(_) => {
                            println!("[FollowWatch] Successfully stopped message listener for {}:{}
", 
                                platform.to_string(), id);
                        },
                        Err(e) => {
                            eprintln!("[FollowWatch] Failed to stop message listener for {}:{}: {}
", 
                                platform.to_string(), id, e);
                        }
                    }
                }
            }

            // sleep for interval
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_millis(config.interval_ms)) => {},
                _ = rx.recv() => { break; }
            }
        }
    });

    *state.task_handle.lock().unwrap() = Some(handle);
    Ok(())
}

// Stop follow watch service - no tauri dependencies
pub async fn stop_follow_watch_service(state: Arc<FollowWatchState>) -> Result<(), String> {
    // Stop the main watch service task
    if let Some(handle) = state.task_handle.lock().unwrap().take() {
        handle.abort();
    }
    
    // Send shutdown signal to the main watch loop
    let maybe_tx = { state.shutdown_tx.lock().unwrap().take() };
    if let Some(tx) = maybe_tx {
        let _ = tx.send(()).await;
    }
    
    // Stop all message listeners using the message manager
    let message_manager = crate::message_manager::MessageManager::get_global_manager();
    match message_manager.stop_all_listeners().await {
        Ok(_) => println!("[FollowWatch] Successfully stopped all message listeners\n"),
        Err(e) => eprintln!("[FollowWatch] Failed to stop all message listeners: {}\n", e),
    }
    
    Ok(())
}
