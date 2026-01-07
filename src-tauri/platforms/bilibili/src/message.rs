use tokio::sync::oneshot;

use shared::interface::{Message, MessageCallback, MessageType, PlatformType, MessageListener, ListenerStatus, PlatformError};
use shared::logger::Logger;
use shared::EventBroadcaster;
use shared::PlatformEvent;
use crate::models::BiliMessage;
use crate::websocket::BiliLiveClient;

// 创建静态日志记录器
static LOGGER: std::sync::OnceLock<Logger> = std::sync::OnceLock::new();

fn logger() -> &'static Logger {
    LOGGER.get_or_init(|| {
        Logger::new(Some(PlatformType::Bilibili), "bilibili::message")
    })
}

/// Bilibili消息监听器实现
struct BiliMessageListener {
    room_id: String,
    stop_tx: Option<oneshot::Sender<()>>,
    status: ListenerStatus,
    message_count: u32,
}

#[async_trait::async_trait]
impl MessageListener for BiliMessageListener {
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
        if let Some(tx) = self.stop_tx.take() {
            if tx.send(()).is_err() {
                logger().error("Failed to send stop signal, listener may have exited.");
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

/// 启动Bilibili消息监听器
/// 
/// # 参数
/// - `room_id`: 直播间ID
/// - `cookie`: 可选的Cookie
/// - `callback`: 消息回调函数
pub async fn start_message_listener(
    room_id: String,
    cookie: Option<String>,
    callback: MessageCallback,
) -> Result<Box<dyn MessageListener>, PlatformError> {
    logger().info(format!("start_message_listener called for room_id: {}", room_id));
    
    // 创建停止信号通道
    let (stop_tx, stop_rx) = oneshot::channel();
    let room_id_clone = room_id.clone();
    let cookie_clone = cookie.clone();

    // 创建初始监听器状态
    let initial_status = ListenerStatus {
        platform: PlatformType::Bilibili,
        room_id: room_id.clone(),
        status: "CONNECTING".to_string(),
        message_count: 0,
        last_update: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    };
    
    // 创建监听器实例
    let listener = BiliMessageListener {
        room_id: room_id.clone(),
        stop_tx: Some(stop_tx),
        status: initial_status.clone(),
        message_count: 0,
    };
    
    // Spawn tokio task to run async message listener
    tokio::spawn(async move {
        // 更新状态为连接中
        let mut status = initial_status;
        status.status = "CONNECTING".to_string();
        status.last_update = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // 创建异步任务来接收消息
        let message_task = tokio::task::spawn_blocking(move || {
            let mut client = match cookie_clone.as_ref() {
                Some(c) => BiliLiveClient::new_with_cookie(c.as_str(), &room_id_clone),
                None => BiliLiveClient::new_without_cookie(&room_id_clone),
            };
            client.send_auth();

            loop {
                if let Some(msg) = client.read_once() {
                    match msg {
                        BiliMessage::Danmu { user, text } => {
                            // 获取当前时间戳，添加错误处理
                            let timestamp = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                                Ok(dur) => dur.as_secs(),
                                Err(e) => {
                                    logger().error(format!("[message] failed to get timestamp: {:?}", e));
                                    0
                                }
                            };
                            
                            // 构建统一的Message对象
                            let message = Message {
                                id: None,
                                message_type: MessageType::Danmaku,
                                user,
                                content: text,
                                user_level: None,
                                fans_level: None,
                                fans_club_level: None,
                                badge_name: None,
                                badge_level: None,
                                uid: None,
                                color: None,
                                timestamp,
                                room_id: room_id_clone.clone(),
                                platform: PlatformType::Bilibili,
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
                            // 调用回调函数
                            callback(message.clone());
                            // 使用事件广播器发布消息
                            EventBroadcaster::global().publish(PlatformEvent::Message(message));
                        }
                        BiliMessage::Gift { user, gift } => {
                            // 获取当前时间戳，添加错误处理
                            let timestamp = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                                Ok(dur) => dur.as_secs(),
                                Err(e) => {
                                    logger().error(format!("[message] failed to get timestamp: {:?}", e));
                                    0
                                }
                            };
                            
                            // 构建统一的Message对象
                            let message = Message {
                                id: None,
                                message_type: MessageType::Gift,
                                user,
                                content: format!("[礼物] {}", gift),
                                user_level: None,
                                fans_level: None,
                                fans_club_level: None,
                                badge_name: None,
                                badge_level: None,
                                uid: None,
                                color: None,
                                timestamp,
                                room_id: room_id_clone.clone(),
                                platform: PlatformType::Bilibili,
                                gift_name: Some(gift),
                                gift_count: Some(1),
                                gift_price: None,
                                gift_total: None,
                                super_chat_price: None,
                                super_chat_duration: None,
                                combo_count: None,
                                combo_user: None,
                                raw: None,
                                other: None,
                            };
                            // 调用回调函数
                            callback(message.clone());
                            // 使用事件广播器发布消息
                            EventBroadcaster::global().publish(PlatformEvent::Message(message));
                        }
                        BiliMessage::Unsupported { cmd } => {
                            logger().debug(format!("[message] unsupported message type: {}", cmd));
                        }
                    }
                }
            }
        });

        // 更新状态为已连接
        status.status = "CONNECTED".to_string();
        status.last_update = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // 等待停止信号
        let _ = stop_rx.await;
        // 取消消息接收任务
        message_task.abort();
        
        // 更新状态为已断开
        status.status = "DISCONNECTED".to_string();
        status.last_update = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
    });

    Ok(Box::new(listener))
}
