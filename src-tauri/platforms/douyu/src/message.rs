use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::Mutex;
use tokio::time::Duration;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{connect_async_tls_with_config, tungstenite::Message as WsMessage};
use url::Url;

// 使用统一的MessageCallback类型
use shared::interface::{MessageCallback, Message, MessageType, PlatformType, MessageListener, ListenerStatus};
use shared::logger::Logger;
use shared::EventBroadcaster;
use shared::PlatformEvent;
use shared::interface::PlatformError;

// 斗鱼消息监听器，实现MessageListener trait
pub struct DouyuMessageListener {
    pub room_id: String,
    stop_sender: Option<oneshot::Sender<()>>,
    status: ListenerStatus,
    message_count: u32,
}

#[async_trait::async_trait]
impl MessageListener for DouyuMessageListener {
    // 停止消息监听
    async fn stop(&mut self) -> Result<(), PlatformError> {
        if let Some(stop_sender) = self.stop_sender.take() {
            let _ = stop_sender.send(());
        }
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



pub struct MessageClient {
    room_id: String,
    callback: MessageCallback,
    stop_signal_rx: oneshot::Receiver<()>,
    logger: Logger,
}

// 启动斗鱼消息监听器，返回一个可以停止监听的监听器实例
pub async fn start_message_listener(
    room_id: &str,
    callback: MessageCallback
) -> Result<Box<dyn MessageListener>, String> {
    let logger = Logger::new(Some(PlatformType::Douyu), "douyu_message");
    logger.info(format!("start_message_listener called for room_id: {}", room_id));
    
    // 创建初始监听器状态
    let initial_status = ListenerStatus {
        platform: PlatformType::Douyu,
        room_id: room_id.to_string(),
        status: "CONNECTING".to_string(),
        message_count: 0,
        last_update: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    };
    
    // 创建 oneshot channel 用于停止监听器
    let (stop_sender, stop_signal_rx) = oneshot::channel();
    
    // 创建监听器实例
    let listener = Arc::new(Mutex::new(DouyuMessageListener {
        room_id: room_id.to_string(),
        stop_sender: Some(stop_sender),
        status: initial_status,
        message_count: 0,
    }));
    
    // 创建消息客户端
    let mut client = MessageClient {
        room_id: room_id.to_string(),
        callback,
        stop_signal_rx,
        logger: logger.clone(),
    };
    
    // 启动消息监听器
    let logger_clone = logger.clone();
    let room_id_str = room_id.to_string();
    
    tokio::spawn(async move {
        if let Err(e) = client.start().await {
            logger_clone.error(format!("Message Listener Error for room_id {}: {}", room_id_str, e));
        }
    });
    
    logger.debug(format!("Message listener started for room_id: {}", room_id));
    
    // 返回一个包装后的监听器，它内部持有 Arc<Mutex<DouyuMessageListener>>
    Ok(Box::new(DouyuMessageListenerWrapper::new(listener)))  
}

// 包装 DouyuMessageListener，使其实现 MessageListener trait
struct DouyuMessageListenerWrapper {
    inner: Arc<Mutex<DouyuMessageListener>>,
}

impl DouyuMessageListenerWrapper {
    fn new(inner: Arc<Mutex<DouyuMessageListener>>) -> Self {
        Self {
            inner,
        }
    }
}

#[async_trait::async_trait]
impl MessageListener for DouyuMessageListenerWrapper {
    // 停止消息监听
    async fn stop(&mut self) -> Result<(), PlatformError> {
        let mut listener = self.inner.lock().await;
        listener.stop().await
    }
    
    // 获取监听器状态
    fn status(&self) -> ListenerStatus {
        // 由于需要获取锁，这里使用 block_in_place 来执行异步代码
        tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current().block_on(async move {
                let listener = self.inner.lock().await;
                listener.status()
            })
        })
    }
    
    // 获取消息计数
    fn message_count(&self) -> u32 {
        // 由于需要获取锁，这里使用 block_in_place 来执行异步代码
        tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current().block_on(async move {
                let listener = self.inner.lock().await;
                listener.message_count()
            })
        })
    }
}

impl MessageClient {
    pub fn new(room_id: &str, callback: MessageCallback, stop_signal_rx: oneshot::Receiver<()>) -> Self {
        Self {
            room_id: room_id.to_string(),
            callback,
            stop_signal_rx,
            logger: Logger::new(Some(PlatformType::Douyu), "douyu_websocket"),
        }
    }

    fn encode_msg(&self, msg: &str) -> Vec<u8> {
        let msg_bytes = msg.as_bytes();
        let packet_len = msg_bytes.len() + 9;

        let mut result = Vec::new();
        result.extend_from_slice(&(packet_len as u32).to_le_bytes());
        result.extend_from_slice(&(packet_len as u32).to_le_bytes());
        result.extend_from_slice(&689u16.to_le_bytes());
        result.push(0);
        result.push(0);
        result.extend_from_slice(msg_bytes);
        result.push(0);

        result
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.logger.info(format!("Connecting to Douyu WebSocket server for room_id: {}", self.room_id));
        
        let url = Url::parse("wss://danmuproxy.douyu.com:8506/")?;
        let mut request = url.into_client_request()?;
        request
            .headers_mut()
            .insert("Sec-WebSocket-Protocol", "binary".parse()?);

        let (ws_stream, _) = connect_async_tls_with_config(request, None, false, None).await?;
        self.logger.debug(format!("Connected to Douyu WebSocket server for room_id: {}", self.room_id));

        let (mut write, mut read) = ws_stream.split();

        // 发送登录请求
        let login_msg = format!("type@=loginreq/roomid@={}/", self.room_id);
        let login_data = self.encode_msg(&login_msg);
        write.send(WsMessage::Binary(login_data)).await?;
        self.logger.debug(format!("Sent login request for room_id: {}", self.room_id));

        // 发送加入房间请求
        let join_msg = format!("type@=joingroup/rid@={}/gid@=1/", self.room_id);
        let join_data = self.encode_msg(&join_msg);
        write.send(WsMessage::Binary(join_data)).await?;
        self.logger.debug(format!("Sent join group request for room_id: {}", self.room_id));

        // 创建消息通道
        let (tx, mut rx) = mpsc::channel(32);

        // 启动心跳任务
        let heartbeat_msg = "type@=mrkl/";
        let heartbeat_data = self.encode_msg(heartbeat_msg);
        let tx_clone = tx.clone();
        let room_id_clone = self.room_id.clone();
        let logger_clone = self.logger.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(45)).await;
                logger_clone.debug(format!("Sending heartbeat for room_id: {}", room_id_clone));
                if let Err(e) = tx_clone.send(WsMessage::Binary(heartbeat_data.clone())).await {
                    logger_clone.error(format!("Failed to send heartbeat for room_id {}: {}", room_id_clone, e));
                    break;
                }
            }
        });

        // Keep a reference to self.stop_signal_rx to move into tasks
        // We need to select between receiving a message from the websocket and the stop signal
        let mut stop_rx = std::mem::replace(&mut self.stop_signal_rx, oneshot::channel().1);

        // Message sending task
        let send_task = tokio::spawn(async move {
            while let Some(msg_to_send) = rx.recv().await {
                if let Err(_) = write.send(msg_to_send).await {
                    break;
                }
            }
        });

        let callback = &self.callback;
        let room_id_clone = self.room_id.clone();
        let logger = &self.logger;

        // Processing incoming messages
        self.logger.info(format!("Started processing messages for room_id: {}", self.room_id));
        
        loop {
            tokio::select! {
                _ = &mut stop_rx => {
                    logger.info(format!("Stop signal received for room_id: {}, terminating listener.", room_id_clone));
                    break;
                }
                msg_option = read.next() => {
                    match msg_option {
                        Some(Ok(WsMessage::Binary(data))) => {
                            if data.len() < 13 {
                                logger.debug(format!("Received invalid message (too short) for room_id: {}", room_id_clone));
                                continue;
                            }

                            let content = String::from_utf8_lossy(&data[12..data.len()-1]);
                            let mut result = HashMap::new();
                            for item in content.split('/') {
                                if item.is_empty() {
                                    continue;
                                }
                                if let Some((key, value)) = item.split_once("@=") {
                                    result.insert(
                                        key.to_string(),
                                        value.replace("@S", "/").replace("@A", "@")
                                    );
                                }
                            }

                            // 处理聊天消息
                            if result.get("type").map_or(false, |t| t == "chatmsg") {
                                let unknown = "unknown".to_string();
                                let empty = "".to_string();
                                let zero = "0".to_string();

                                // 构建统一的Message结构体
                                let message = Message {
                                    id: None,
                                    message_type: MessageType::Danmaku,
                                    user: result.get("nn").unwrap_or(&unknown).to_string(),
                                    content: result.get("txt").unwrap_or(&empty).to_string(),
                                    user_level: result
                                        .get("level")
                                        .unwrap_or(&zero)
                                        .parse::<i64>()
                                        .ok(),
                                    fans_level: result
                                        .get("bl")
                                        .unwrap_or(&zero)
                                        .parse::<i32>()
                                        .ok(),
                                    fans_club_level: None,
                                    badge_name: None,
                                    badge_level: None,
                                    uid: result.get("uid").map(|uid| uid.to_string()),
                                    color: result.get("col").map(|c| c.to_string()),
                                    timestamp: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_millis() as u64,
                                    room_id: room_id_clone.clone(),
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
                                    other: Some(serde_json::json!({
                                        "color": result.get("col").map(|c| c.to_string())
                                    })),
                                };

                                // 调用回调函数
                                callback(message.clone());
                                // 使用事件广播器发布消息
                                EventBroadcaster::global().publish(PlatformEvent::Message(message));
                            } 
                            // 处理进入房间消息
                            else if result.get("type").map_or(false, |t| t == "uenter") {
                                let unknown = "unknown".to_string();
                                let empty = "".to_string();
                                let zero = "0".to_string();

                                // 构建统一的Message结构体
                                let message = Message {
                                    id: None,
                                    message_type: MessageType::EnterRoom,
                                    user: result.get("nn").unwrap_or(&unknown).to_string(),
                                    content: format!("用户 {} 进入房间", result.get("nn").unwrap_or(&unknown)),
                                    user_level: result
                                        .get("level")
                                        .unwrap_or(&zero)
                                        .parse::<i64>()
                                        .ok(),
                                    fans_level: result
                                        .get("bl")
                                        .unwrap_or(&zero)
                                        .parse::<i32>()
                                        .ok(),
                                    fans_club_level: None,
                                    badge_name: result.get("bnn").map(|bnn| bnn.to_string()),
                                    badge_level: result.get("bl").and_then(|bl| bl.parse::<i32>().ok()),
                                    uid: result.get("uid").map(|uid| uid.to_string()),
                                    color: None,
                                    timestamp: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_millis() as u64,
                                    room_id: room_id_clone.clone(),
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
                                    other: Some(serde_json::json!({
                                        "uid": result.get("uid").unwrap_or(&empty),
                                        "badge_name": result.get("bnn").unwrap_or(&empty),
                                    })),
                                };

                                // 调用回调函数
                                callback(message.clone());
                                // 使用事件广播器发布消息
                                EventBroadcaster::global().publish(PlatformEvent::Message(message));
                            }
                            else {
                                logger.debug(format!("Received unknown message type: {:?} for room_id: {}", result.get("type"), room_id_clone));
                            }
                        }
                        Some(Ok(WsMessage::Close(_))) => {
                            logger.info(format!("WebSocket connection closed for room_id: {}", room_id_clone));
                            break;
                        }
                        Some(Err(e)) => {
                            logger.error(format!("WebSocket error for room_id {}: {}", room_id_clone, e));
                            break;
                        }
                        None => {
                            logger.info(format!("WebSocket stream ended for room_id: {}", room_id_clone));
                            break;
                        }
                        _ => {
                            logger.debug(format!("Received unexpected message type for room_id: {}", room_id_clone));
                        }
                    }
                }
            }
        }
        send_task.abort();
        self.logger.info(format!("Message listener stopped for room_id: {}", self.room_id));
        Ok(())
    }
}
