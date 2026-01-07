use std::sync::{Arc, Mutex as StdMutex};
use std::collections::HashMap;
use tokio::sync::Mutex;
use shared::interface::{PlatformType, MessageCallback, MessageListener, Message, ListenerStatus, PlatformError, PlatformConfig};
use crate::factory::{get_global_platform_factory, PlatformFactory};

// 定义消息订阅者类型
pub type MessageSubscriber = Box<dyn Fn(Message) + Send + Sync + 'static>;

// 消息管理器状态
#[derive(Default)]
pub struct MessageManagerState {
    // 存储消息监听器，key: "platform:room_id"
    message_listeners: HashMap<String, Box<dyn MessageListener>>,
    // 存储监听器状态，key: "platform:room_id"
    listener_statuses: HashMap<String, ListenerStatus>,
    // 消息计数，key: "platform:room_id"
    message_counts: HashMap<String, u32>,
    // 消息订阅者，使用全局唯一ID标识
    subscribers: HashMap<u64, MessageSubscriber>,
    // 下一个订阅者ID
    next_subscriber_id: u64,
}

// 消息管理器
pub struct MessageManager {
    state: Arc<Mutex<MessageManagerState>>,
    factory: Arc<PlatformFactory>,
}

impl MessageManager {
    // 创建新的消息管理器实例
    pub fn new() -> Self {
        let factory = get_global_platform_factory();
        Self {
            state: Arc::new(Mutex::new(MessageManagerState::default())),
            factory,
        }
    }
    
    // 获取全局消息管理器实例
    pub fn get_global_manager() -> Arc<Self> {
        static GLOBAL_MANAGER: StdMutex<Option<Arc<MessageManager>>> = StdMutex::new(None);
        GLOBAL_MANAGER.lock().unwrap()
            .get_or_insert_with(|| Arc::new(MessageManager::new()))
            .clone()
    }
    
    // 生成监听器键
    fn listener_key(platform: &PlatformType, room_id: &str) -> String {
        format!("{0}:{1}", platform.to_string(), room_id)
    }
    
    // 启动消息监听器
    pub async fn start_message_listener(
        &self,
        platform: PlatformType,
        room_id: &str,
        callback: MessageCallback
    ) -> Result<(), PlatformError> {
        let key = Self::listener_key(&platform, room_id);
        let mut state = self.state.lock().await;
        
        // 检查是否已经存在监听器
        if state.message_listeners.contains_key(&key) {
            return Ok(()); // 监听器已存在，直接返回
        }
        
        // 创建平台配置
        let config = PlatformConfig::default();
        
        // 获取平台实例
        let platform_instance = self.factory.get_platform(platform.clone(), config).await?;
        
        // 启动消息监听器
        let listener = platform_instance.start_message_listener(room_id, callback).await?;
        
        // 获取初始状态
        let status = listener.status();
        
        // 存储监听器和状态
        state.message_listeners.insert(key.clone(), listener);
        state.listener_statuses.insert(key.clone(), status);
        state.message_counts.insert(key, 0);
        
        Ok(())
    }
    
    // 停止消息监听器
    pub async fn stop_message_listener(
        &self,
        platform: PlatformType,
        room_id: &str
    ) -> Result<(), PlatformError> {
        let key = Self::listener_key(&platform, room_id);
        let mut state = self.state.lock().await;
        
        // 检查监听器是否存在
        if let Some(mut listener) = state.message_listeners.remove(&key) {
            // 停止监听器
            listener.stop().await?;
            
            // 清理状态
            state.listener_statuses.remove(&key);
            state.message_counts.remove(&key);
        }
        
        Ok(())
    }
    
    // 获取监听器状态
    pub async fn get_listener_status(
        &self,
        platform: Option<PlatformType>,
        room_id: Option<&str>
    ) -> Vec<ListenerStatus> {
        let state = self.state.lock().await;
        
        match (platform, room_id) {
            // 获取特定平台特定房间的监听器状态
            (Some(platform), Some(room_id)) => {
                let key = Self::listener_key(&platform, room_id);
                if let Some(status) = state.listener_statuses.get(&key) {
                    vec![status.clone()]
                } else {
                    vec![]
                }
            },
            // 获取特定平台所有房间的监听器状态
            (Some(platform), None) => {
                state.listener_statuses.values()
                    .filter(|status| status.platform == platform)
                    .cloned()
                    .collect()
            },
            // 获取所有平台所有房间的监听器状态
            (None, None) => {
                state.listener_statuses.values()
                    .cloned()
                    .collect()
            },
            // 获取所有平台特定房间的监听器状态（不太常用，但支持）
            (None, Some(room_id)) => {
                state.listener_statuses.values()
                    .filter(|status| status.room_id == room_id)
                    .cloned()
                    .collect()
            },
        }
    }
    
    // 获取消息计数
    pub async fn get_message_count(
        &self,
        platform: Option<PlatformType>,
        room_id: Option<&str>
    ) -> HashMap<String, u32> {
        let state = self.state.lock().await;
        
        match (platform, room_id) {
            // 获取特定平台特定房间的消息计数
            (Some(platform), Some(room_id)) => {
                let key = Self::listener_key(&platform, room_id);
                if let Some(count) = state.message_counts.get(&key) {
                    let mut map = HashMap::new();
                    map.insert(key, *count);
                    map
                } else {
                    HashMap::new()
                }
            },
            // 获取特定平台所有房间的消息计数
            (Some(platform), None) => {
                let platform_str = platform.to_string();
                state.message_counts.iter()
                    .filter(|(key, _)| key.starts_with(&platform_str))
                    .map(|(k, v)| (k.clone(), *v))
                    .collect()
            },
            // 获取所有平台所有房间的消息计数
            (None, None) => {
                state.message_counts.clone()
            },
            // 获取所有平台特定房间的消息计数（不太常用，但支持）
            (None, Some(room_id)) => {
                state.message_counts.iter()
                    .filter(|(key, _)| key.ends_with(&format!(":{}", room_id)))
                    .map(|(k, v)| (k.clone(), *v))
                    .collect()
            },
        }
    }
    
    // 订阅消息
    pub async fn subscribe(&self, subscriber: MessageSubscriber) -> u64 {
        let mut state = self.state.lock().await;
        
        // 生成唯一的订阅者ID
        let id = state.next_subscriber_id;
        state.next_subscriber_id += 1;
        
        // 存储订阅者
        state.subscribers.insert(id, subscriber);
        
        id
    }
    
    // 取消订阅
    pub async fn unsubscribe(&self, subscriber_id: u64) -> bool {
        let mut state = self.state.lock().await;
        
        // 移除订阅者
        state.subscribers.remove(&subscriber_id).is_some()
    }
    
    // 处理接收到的消息
    pub async fn process_message(&self, message: Message) {
        let key = Self::listener_key(&message.platform, &message.room_id);
        
        // 打印消息，方便调试
        println!("Message received and broadcasted: {:?}", message);
        
        // 更新消息计数和监听器状态
        {
            let mut state = self.state.lock().await;
            
            // 先更新消息计数
            let entry = state.message_counts.entry(key.clone()).or_insert(0);
            *entry += 1;
            let count_val = *entry;
            
            // 然后更新监听器状态
            if let Some(status) = state.listener_statuses.get_mut(&key) {
                status.message_count = count_val;
                status.last_update = message.timestamp;
            }
            
            // 直接在锁内调用所有订阅者，避免生命周期问题
            // 这样做的好处是避免了复杂的生命周期管理
            // 缺点是如果某个订阅者的回调耗时较长，会阻塞其他订阅者
            // 但考虑到消息处理应该是轻量级的，这个方案是可以接受的
            for (_, subscriber) in state.subscribers.iter() {
                // 克隆消息，避免borrow问题
                let msg_clone = message.clone();
                // 调用订阅者回调
                subscriber(msg_clone);
            }
        }
    }
    
    // 检查是否存在活跃的监听器
    pub async fn has_active_listener(&self, platform: PlatformType, room_id: &str) -> bool {
        let key = Self::listener_key(&platform, room_id);
        let state = self.state.lock().await;
        state.message_listeners.contains_key(&key)
    }
    
    // 停止所有消息监听器
    pub async fn stop_all_listeners(&self) -> Result<(), PlatformError> {
        let mut state = self.state.lock().await;
        
        // 停止所有监听器
        for (_, mut listener) in state.message_listeners.drain() {
            listener.stop().await?;
        }
        
        // 清理状态
        state.listener_statuses.clear();
        state.message_counts.clear();
        
        Ok(())
    }
}

// 全局消息管理器实例
type GlobalMessageManager = StdMutex<Option<Arc<MessageManager>>>;
static GLOBAL_MANAGER: GlobalMessageManager = StdMutex::new(None);

// 获取全局消息管理器实例
pub fn get_global_message_manager() -> Arc<MessageManager> {
    GLOBAL_MANAGER.lock().unwrap()
        .get_or_insert_with(|| Arc::new(MessageManager::new()))
        .clone()
}
