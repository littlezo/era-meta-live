use tokio::sync::broadcast::{self, error::RecvError};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};

use crate::interface::{Message, PlatformType};

// 定义事件类型
#[derive(Clone)]
pub enum PlatformEvent {
    Message(Message),
    RoomStatusChanged {
        platform: PlatformType,
        room_id: String,
        live_status: bool,
        live_status_detail: String,
    },
    StreamerStatusChanged {
        platform: PlatformType,
        streamer_id: String,
        live_status: bool,
        live_status_detail: String,
    },
    Error {
        platform: PlatformType,
        message: String,
        code: Option<u32>,
    },
    Other {
        event_type: String,
        data: serde_json::Value,
    },
}

// 事件广播器
#[derive(Clone)]
pub struct EventBroadcaster {
    inner: Arc<broadcast::Sender<PlatformEvent>>,
}

impl EventBroadcaster {
    // 创建新的事件广播器
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(100);
        Self {
            inner: Arc::new(sender),
        }
    }

    // 获取全局事件广播器实例
    pub fn global() -> &'static Self {
        static BROADCASTER: OnceLock<EventBroadcaster> = OnceLock::new();
        BROADCASTER.get_or_init(|| EventBroadcaster::new())
    }

    // 发布事件
    pub fn publish(&self, event: PlatformEvent) {
        // 忽略接收者数量为0的情况
        let _ = self.inner.send(event);
    }

    // 订阅事件
    pub fn subscribe(&self) -> broadcast::Receiver<PlatformEvent> {
        self.inner.subscribe()
    }

    // 订阅特定类型的事件
    pub fn subscribe_filtered<F>(&self, filter: F) -> FilteredEventReceiver
    where
        F: Fn(&PlatformEvent) -> bool + Send + Sync + 'static,
    {
        let receiver = self.inner.subscribe();
        FilteredEventReceiver::new(receiver, filter)
    }
}

// 默认实现
impl Default for EventBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

// 过滤后的事件接收器
pub struct FilteredEventReceiver {
    receiver: broadcast::Receiver<PlatformEvent>,
    filter: Arc<dyn Fn(&PlatformEvent) -> bool + Send + Sync + 'static>,
}

impl FilteredEventReceiver {
    // 创建新的过滤事件接收器
    pub fn new(receiver: broadcast::Receiver<PlatformEvent>, filter: impl Fn(&PlatformEvent) -> bool + Send + Sync + 'static) -> Self {
        Self {
            receiver,
            filter: Arc::new(filter),
        }
    }

    // 异步接收事件
    pub async fn recv(&mut self) -> Result<PlatformEvent, RecvError> {
        loop {
            match self.receiver.recv().await {
                Ok(event) => {
                    if (self.filter)(&event) {
                        return Ok(event);
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface::{Message, MessageType, PlatformType};
    
    #[tokio::test]
    async fn test_event_broadcaster() {
        // 创建事件广播器
        let broadcaster = EventBroadcaster::new();
        
        // 创建订阅者
        let mut subscriber1 = broadcaster.subscribe();
        let mut subscriber2 = broadcaster.subscribe();
        
        // 创建测试消息
        let message = Message {
            id: Some("test_id".to_string()),
            message_type: MessageType::Danmaku,
            user: "test_user".to_string(),
            content: "test content".to_string(),
            user_level: Some(10),
            fans_level: Some(5),
            fans_club_level: Some(3),
            badge_name: Some("test_badge".to_string()),
            badge_level: Some(2),
            uid: Some("test_uid".to_string()),
            color: Some("#ffffff".to_string()),
            timestamp: 1234567890,
            room_id: "12345".to_string(),
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
            other: None,
        };
        
        // 发布事件
        broadcaster.publish(PlatformEvent::Message(message.clone()));
        
        // 接收事件
        let event1 = subscriber1.recv().await.unwrap();
        let event2 = subscriber2.recv().await.unwrap();
        
        // 验证事件
        assert!(matches!(event1, PlatformEvent::Message(_)));
        assert!(matches!(event2, PlatformEvent::Message(_)));
        
        if let PlatformEvent::Message(msg1) = event1 {
            assert_eq!(msg1.id, message.id);
            assert_eq!(msg1.content, message.content);
        }
        
        if let PlatformEvent::Message(msg2) = event2 {
            assert_eq!(msg2.id, message.id);
            assert_eq!(msg2.content, message.content);
        }
    }
    
    #[tokio::test]
    async fn test_filtered_event_receiver() {
        // 创建事件广播器
        let broadcaster = EventBroadcaster::new();
        
        // 创建过滤订阅者，只接收Message事件
        let mut filtered_subscriber = broadcaster.subscribe_filtered(|event| {
            matches!(event, PlatformEvent::Message(_))
        });
        
        // 发布不同类型的事件
        broadcaster.publish(PlatformEvent::Error {
            platform: PlatformType::Douyu,
            message: "test error".to_string(),
            code: Some(500),
        });
        
        let message = Message {
            id: Some("test_id".to_string()),
            message_type: MessageType::Danmaku,
            user: "test_user".to_string(),
            content: "test content".to_string(),
            user_level: Some(10),
            fans_level: Some(5),
            fans_club_level: Some(3),
            badge_name: Some("test_badge".to_string()),
            badge_level: Some(2),
            uid: Some("test_uid".to_string()),
            color: Some("#ffffff".to_string()),
            timestamp: 1234567890,
            room_id: "12345".to_string(),
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
            other: None,
        };
        
        broadcaster.publish(PlatformEvent::Message(message.clone()));
        
        // 接收事件，应该只收到Message事件
        let event = filtered_subscriber.recv().await.unwrap();
        
        // 验证事件
        assert!(matches!(event, PlatformEvent::Message(_)));
        
        if let PlatformEvent::Message(msg) = event {
            assert_eq!(msg.id, message.id);
            assert_eq!(msg.content, message.content);
        }
    }
}
