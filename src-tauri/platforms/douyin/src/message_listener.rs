use crate::web_api::normalize_douyin_live_id;
use shared::interface::Message as PlatformMessage;
use tokio::sync::mpsc as tokio_mpsc;
use shared::interface::MessageCallback;
use std::sync::Arc;
use tokio::sync::Mutex;
use shared::interface::{PlatformType, MessageListener, ListenerStatus, PlatformError};
use shared::logger::Logger;

// 创建静态日志记录器
static LOGGER: std::sync::OnceLock<Logger> = std::sync::OnceLock::new();

fn logger() -> &'static Logger {
    LOGGER.get_or_init(|| {
        Logger::new(Some(PlatformType::Douyin), "douyin::message_listener")
    })
}


// Douyin message listener state
#[derive(Default)]
pub struct DouyinMessageListenerState {
    shutdown_tx: Option<tokio_mpsc::Sender<()>>,
}

impl DouyinMessageListenerState {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Douyin消息监听器实现
pub struct DouyinMessageListener {
    room_id: String,
    state: Arc<Mutex<DouyinMessageListenerState>>,
    status: ListenerStatus,
    message_count: u32,
}

#[async_trait::async_trait]
impl MessageListener for DouyinMessageListener {
    // 停止消息监听
    async fn stop(&mut self) -> Result<(), PlatformError> {
        logger().info(format!("stop called for room_id: {}", self.room_id));
        
        // 更新状态
        self.status.status = "DISCONNECTING".to_string();
        self.status.last_update = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // 发送停止信号
        let previous_tx = {
            let mut lock = self.state.lock().await;
            lock.shutdown_tx.take()
        };
        
        if let Some(tx) = previous_tx {
            logger().info("Sending shutdown to Douyin listener task.");
            if let Err(e) = tx.send(()).await {
                logger().error(format!("Failed to send shutdown signal: {}", e));
            }
        }
        
        // 更新状态
        self.status.status = "DISCONNECTED".to_string();
        self.status.last_update = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        Ok(())
    }
    
    // 获取监听器状态
    fn status(&self) -> ListenerStatus {
        self.status.clone()
    }
    
    // 获取消息计数
    fn message_count(&self) -> u32 {
        self.message_count
    }
}

pub async fn start_douyin_message_listener(
    room_id_or_url: &str,
    callback: MessageCallback,
    state: Arc<Mutex<DouyinMessageListenerState>>,
) -> Result<Box<dyn MessageListener>, String> {
    logger().info(format!(
        "Received request for room_id_or_url: {}",
        room_id_or_url
    ));

    let previous_tx = {
        let mut lock = state.lock().await;
        lock.shutdown_tx.take()
    };

    if let Some(tx) = previous_tx {
        logger().info("Sending shutdown to previous Douyin listener task.");
        let _ = tx.send(()).await;
    }

    if room_id_or_url == "stop_listening" {
        logger().info(
            "Received stop_listening signal. Listener will not be restarted."
        );
        return Err("stop_listening signal received".to_string());
    }

    let normalized_room_id = normalize_douyin_live_id(&room_id_or_url);
    logger().info(format!("Normalized room ID: {}", normalized_room_id));

    let (tx_shutdown, mut rx_shutdown) = tokio_mpsc::channel::<()>(1);
    {
        let mut lock = state.lock().await;
        lock.shutdown_tx = Some(tx_shutdown);
    }

    let room_id_str_clone = normalized_room_id.clone();
    let callback_clone = Arc::new(callback);

    // 创建初始监听器状态
    let initial_status = ListenerStatus {
        platform: PlatformType::Douyin,
        room_id: normalized_room_id.clone(),
        status: "CONNECTING".to_string(),
        message_count: 0,
        last_update: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    };
    
    // 创建监听器实例
    let listener = DouyinMessageListener {
        room_id: normalized_room_id.clone(),
        state: state.clone(),
        status: initial_status.clone(),
        message_count: 0,
    };

    tokio::spawn(async move {
        logger().info(format!(
            "Spawning listener for room: {}",
            room_id_str_clone
        ));

        let task_result = {
            let mut attempt: u32 = 1;
            loop {
                let attempt_result = async {
                    logger().info(format!("尝试连接，第 {}/3 次: 初始化WebFetcher", attempt));
                    let mut fetcher = crate::message::web_fetcher::DouyinLiveWebFetcher::new(&room_id_str_clone)?;
                    logger().info(format!("尝试连接，第 {}/3 次: 获取房间详情", attempt));
                    fetcher
                        .fetch_room_details()
                        .await
                        .map_err(|e| format!("Failed to fetch room details: {}", e))?;

                    logger().info(format!("尝试连接，第 {}/3 次: 提取房间实际ID", attempt));
                    let actual_room_id = fetcher.get_room_id().await?;
                    logger().info(format!("尝试连接，第 {}/3 次: 提取Cookie", attempt));
                    let cookie_header = fetcher.get_dy_cookie().await?;
                    logger().info(format!("尝试连接，第 {}/3 次: 提取用户唯一ID", attempt));
                    let user_unique_id = fetcher.get_user_unique_id().await?;
                    logger().info(format!(
                        "尝试连接，第 {}/3 次: 连接参数: room_id={}, user_unique_id={}",
                        attempt, actual_room_id, user_unique_id
                    ));

                    logger().info(format!("尝试连接，第 {}/3 次: 建立WebSocket连接", attempt));
                    let (read_stream, ack_tx) = crate::message::websocket_connection::connect_and_manage_websocket(
                        &fetcher,
                        &actual_room_id,
                        &cookie_header,
                        &user_unique_id,
                    )
                    .await?;

                    logger().info(format!(
                        "WebSocket connected for room: {}",
                        actual_room_id
                    ));

                    // Create a message callback adapter
                    let callback = callback_clone.clone();
                    let message_handler = move |message: PlatformMessage| {
                        logger().debug(format!("Calling callback for message: {:?}", message.message_type));
                        callback(message);
                    };

                    tokio::select! {
                        res = crate::message::message_handler::handle_received_messages_with_callback(
                            read_stream,
                            ack_tx,
                            message_handler,
                            actual_room_id.clone()
                        ) => {
                            if let Err(e) = res {
                                logger().error(format!("Message handler error: {}", e));
                                return Err(e);
                            }
                        }
                        _ = rx_shutdown.recv() => {
                            logger().info(format!(
                                "Received shutdown signal for room {}.",
                                actual_room_id
                            ));
                        }
                    }

                    Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
                }
                .await;

                match attempt_result {
                    Ok(_) => break Ok(()),
                    Err(e) => {
                        if attempt >= 2 {
                            logger().error(format!("Listener connect/fetch failed after {} attempts: {}", attempt, e));
                            break Err(e);
                        } else {
                            logger().warn(format!(
                                "Attempt {} failed: {}. Retrying...",
                                attempt, e
                            ));
                            attempt += 1;
                            continue;
                        }
                    }
                }
            }
        };

        if let Err(e) = task_result {
            logger().error(format!(
                "Listener task for room {} critically failed: {}",
                room_id_str_clone, e
            ));
            // Send error message to callback
            let error_message = shared::interface::Message {
                id: None,
                message_type: shared::interface::MessageType::System,
                user: "系统消息".to_string(),
                content: format!("弹幕连接发生错误: {}", e),
                user_level: Some(0),
                fans_level: Some(0),
                fans_club_level: None,
                badge_name: None,
                badge_level: None,
                uid: None,
                color: None,
                timestamp: tokio::time::Instant::now().elapsed().as_millis() as u64,
                room_id: room_id_str_clone.clone(),
                platform: shared::interface::PlatformType::Douyin,
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
            logger().info(format!("Sending error message to callback: {}", error_message.content));
            callback_clone(error_message);
        } else {
            logger().info(format!(
                "Listener task for room {} completed.",
                room_id_str_clone
            ));
        }
    });
    
    Ok(Box::new(listener))
}
