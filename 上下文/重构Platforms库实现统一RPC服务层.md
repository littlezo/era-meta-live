## 重构目标
1. 将watch.rs重构到platforms库中
2. 设计统一的平台接口，抹平平台差异
3. 实现直播状态监控功能
4. 实现开播自动启动弹幕监听功能
5. 实现事件广播机制
6. 移除platforms库对Tauri的依赖

## 实现方案

### 1. 目录结构设计
```
src-tauri/platforms/
├── Cargo.toml                  # 主包配置
└── src/
    ├── lib.rs                  # 主入口，导出统一接口
    ├── common/                 # 通用类型和工具
    │   ├── models.rs           # 统一数据模型
    │   ├── http_client.rs      # HTTP客户端
    │   ├── error.rs            # 统一错误类型
    │   └── event.rs            # 事件定义
    ├── platform/               # 平台抽象层
    │   ├── mod.rs              # Platform trait定义
    │   └── factory.rs          # 平台工厂
    ├── services/               # 服务层
    │   ├── mod.rs
    │   ├── watch_service.rs    # 监控服务，从watch.rs迁移
    │   └── danmaku_service.rs  # 弹幕服务
    ├── bilibili/               # Bilibili平台实现
    │   └── mod.rs
    ├── douyin/                 # Douyin平台实现
    │   └── mod.rs
    ├── douyu/                  # Douyu平台实现
    │   └── mod.rs
    └── huya/                   # Huya平台实现
        └── mod.rs
```

### 2. 核心类型设计

#### 统一平台抽象trait
```rust
pub trait Platform {
    /// 获取平台名称
    fn name(&self) -> &str;
    
    /// 获取平台类型
    fn platform_type(&self) -> PlatformType;
    
    /// 获取主播信息和直播状态
    async fn fetch_streamer_info(&self, room_id: &str) -> Result<StreamerInfo, PlatformError>;
    
    /// 获取直播流地址
    async fn fetch_live_stream(&self, room_id: &str, quality: Option<Quality>) -> Result<LiveStream, PlatformError>;
    
    /// 获取直播列表
    async fn fetch_live_list(&self, category: Option<&str>, page: u32) -> Result<Vec<StreamerInfo>, PlatformError>;
    
    /// 搜索主播/直播间
    async fn search(&self, keyword: &str, page: u32) -> Result<Vec<StreamerInfo>, PlatformError>;
    
    /// 启动弹幕监听
    async fn start_danmaku_listener(
        &self,
        room_id: &str,
        callback: Box<dyn FnMut(DanmakuMessage) + Send + 'static>
    ) -> Result<ListenerHandle, PlatformError>;
    
    /// 停止弹幕监听
    async fn stop_danmaku_listener(&self, handle: ListenerHandle) -> Result<(), PlatformError>;
}
```

#### 统一数据模型
```rust
// 平台类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlatformType {
    Bilibili,
    Douyin,
    Douyu,
    Huya,
}

// 直播状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiveStatus {
    Offline,
    Live,
    Replay,
    Unknown,
}

// 主播信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamerInfo {
    pub platform: PlatformType,
    pub room_id: String,
    pub title: String,
    pub anchor_name: String,
    pub avatar: Option<String>,
    pub status: LiveStatus,
    pub category: Option<String>,
    pub viewers: Option<u64>,
}

// 监控配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchConfig {
    pub streamers: Vec<FollowStreamer>,
    pub interval_ms: u64,
    pub enable_notification: bool,
}

// 关注的主播
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowStreamer {
    pub platform: PlatformType,
    pub id: String,
    pub nickname: Option<String>,
}

// 状态更新事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowStatusUpdate {
    pub platform: PlatformType,
    pub id: String,
    pub live_status: LiveStatus,
    pub nickname: Option<String>,
    pub room_title: Option<String>,
    pub avatar_url: Option<String>,
}
```

### 3. 监控服务设计

#### 监控服务接口
```rust
pub struct WatchService {
    platform_factory: PlatformFactory,
    http_client: HttpClient,
    config: WatchConfig,
    state: WatchState,
    event_sender: Option<mpsc::Sender<PlatformEvent>>,
}

impl WatchService {
    pub fn new(platform_factory: PlatformFactory, http_client: HttpClient) -> Self {
        // 初始化
    }
    
    pub async fn start(&mut self, config: WatchConfig) -> Result<(), PlatformError> {
        // 启动监控服务
    }
    
    pub async fn stop(&mut self) -> Result<(), PlatformError> {
        // 停止监控服务
    }
    
    pub fn set_event_sender(&mut self, sender: mpsc::Sender<PlatformEvent>) {
        // 设置事件发送器
    }
}
```

#### 监控服务核心逻辑
```rust
async fn check_streamers(&self) -> Result<(), PlatformError> {
    for streamer in &self.config.streamers {
        let platform = self.platform_factory.create_platform(streamer.platform);
        let info = platform.fetch_streamer_info(&streamer.id).await?;
        
        // 检查状态变化
        let prev_status = self.state.get_prev_status(&streamer);
        if prev_status != info.status {
            // 发送状态更新事件
            let event = PlatformEvent::FollowStatusUpdate(FollowStatusUpdate {
                platform: streamer.platform,
                id: streamer.id.clone(),
                live_status: info.status,
                nickname: info.anchor_name.clone(),
                room_title: info.title.clone(),
                avatar_url: info.avatar.clone(),
            });
            self.send_event(event).await;
            
            // 如果开播，启动弹幕监听
            if info.status == LiveStatus::Live {
                self.start_danmaku_listener(&streamer).await?;
            } else {
                // 如果下播，停止弹幕监听
                self.stop_danmaku_listener(&streamer).await?;
            }
        }
        
        // 更新状态
        self.state.set_prev_status(&streamer, info.status);
    }
    Ok(())
}
```

### 4. 事件系统设计

#### 事件定义
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlatformEvent {
    /// 主播状态更新
    FollowStatusUpdate(FollowStatusUpdate),
    /// 弹幕消息
    DanmakuMessage(DanmakuMessage),
    /// 开播通知
    LiveNotification(LiveNotification),
    /// 弹幕监听状态变化
    DanmakuListenerStatus(DanmakuListenerStatus),
}

/// 弹幕监听状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DanmakuListenerStatus {
    pub platform: PlatformType,
    pub room_id: String,
    pub is_running: bool,
    pub error: Option<String>,
}
```

### 5. 实现步骤

1. **创建统一类型定义**
   - 实现统一的Platform trait
   - 定义统一的数据模型和错误类型
   - 实现事件系统

2. **重构watch.rs到platforms库**
   - 创建services/watch_service.rs
   - 迁移FollowWatchState和相关逻辑
   - 移除Tauri依赖，使用mpsc通道进行事件通信

3. **实现平台抽象层**
   - 为每个平台实现Platform trait
   - 实现PlatformFactory，用于创建平台实例
   - 统一平台API调用方式

4. **实现监控服务功能**
   - 实现周期性检查主播状态
   - 实现状态变化检测
   - 实现开播自动启动弹幕监听
   - 实现事件广播

5. **实现弹幕服务**
   - 统一弹幕监听启动/停止接口
   - 实现弹幕消息转发

6. **更新Tauri主应用**
   - 使用新的platforms库接口
   - 实现事件监听和处理
   - 实现通知发送
   - 更新命令处理逻辑

### 6. 与Tauri主应用的集成

#### Tauri应用中的事件处理
```rust
// 在Tauri主应用中
let (event_tx, mut event_rx) = mpsc::channel(100);

// 设置事件发送器到监控服务
watch_service.set_event_sender(event_tx);

// 启动事件处理任务
tokio::spawn(async move {
    while let Some(event) = event_rx.recv().await {
        match event {
            PlatformEvent::FollowStatusUpdate(update) => {
                // 发送状态更新事件到前端
                let _ = app.emit("follow_status_update", update);
            }
            PlatformEvent::LiveNotification(notification) => {
                // 发送通知
                let _ = app.emit("notify", notification);
            }
            PlatformEvent::DanmakuMessage(message) => {
                // 发送弹幕消息到前端
                let _ = app.emit("danmaku_message", message);
            }
            PlatformEvent::DanmakuListenerStatus(status) => {
                // 发送弹幕监听状态到前端
                let _ = app.emit("danmaku_listener_status", status);
            }
        }
    }
});
```

## 预期收益
1. **统一接口**：上层应用使用统一API，无需关心具体平台
2. **易于扩展**：新增平台只需实现统一trait
3. **低耦合**：platforms库与Tauri解耦，可独立使用
4. **自动化**：开播自动启动弹幕监听，无需手动操作
5. **实时通知**：及时通知用户关注的主播开播
6. **统一事件系统**：所有平台事件通过统一通道广播

## 实现优先级
1. 创建统一的类型系统和Platform trait
2. 重构watch.rs到platforms库
3. 实现平台抽象层和工厂
4. 实现监控服务功能
5. 实现事件系统
6. 实现弹幕服务
7. 更新Tauri主应用
8. 测试和验证