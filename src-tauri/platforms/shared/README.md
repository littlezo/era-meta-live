# Shared Package

Shared 包是统一直播平台接入层的核心包，提供了统一的接口定义、日志系统和HTTP客户端等核心功能。

## 功能模块

### 1. 统一接口定义

提供了所有直播平台必须实现的统一接口 `LivePlatform`，包括：

- 直播列表获取
- 直播间信息获取
- 直播流URL获取
- 主播信息获取
- 消息监听
- 认证相关功能

### 2. 日志系统

提供了统一的日志记录功能，支持：

- 多级日志（Trace、Debug、Info、Warn、Error）
- 结构化日志
- 带执行时间的日志
- 平台和模块标识

### 3. HTTP客户端

封装了HTTP请求功能，支持：

- GET/POST请求
- JSON处理
- Cookie管理
- 代理支持
- 重试机制

## 快速开始

### 1. 日志系统使用

```rust
use shared::logger::{init_logger, Logger, LogLevel};

// 初始化日志系统
init_logger().unwrap();

// 创建日志记录器
let logger = Logger::new(Some(PlatformType::Douyu), "test_module");

// 记录不同级别的日志
logger.trace("Trace log");
logger.debug("Debug log");
logger.info("Info log");
logger.warn("Warn log");
logger.error("Error log");

// 记录结构化日志
logger.log_structured(
    LogLevel::Info,
    "Structured log",
    &serde_json::json!({"key": "value", "number": 42})
);

// 记录带执行时间的日志
let result = logger.log_with_duration("test_operation", || {
    // 执行操作
    Ok("success")
});
```

### 2. 实现直播平台接口

```rust
use shared::interface::{LivePlatform, PlatformType, LiveList, LiveListParams, RoomInfo, StreamUrl, StreamQuality, Message, MessageCallback};

struct MyPlatform;

#[async_trait::async_trait]
impl LivePlatform for MyPlatform {
    fn platform_type(&self) -> PlatformType {
        PlatformType::Bilibili
    }
    
    async fn fetch_live_list(&self, params: LiveListParams) -> Result<LiveList, PlatformError> {
        // 实现直播列表获取
    }
    
    async fn fetch_room_info(&self, room_id: &str) -> Result<RoomInfo, PlatformError> {
        // 实现直播间信息获取
    }
    
    // 实现其他必要的方法...
}
```

## 核心数据结构

### PlatformType

平台类型枚举，支持：
- Bilibili
- Douyin
- Douyu
- Huya

### MessageType

消息类型枚举，支持：
- Danmaku（弹幕）
- Gift（礼物）
- SuperChat（超级弹幕）
- EnterRoom（进入房间）
- Follow/Unfollow（关注/取消关注）
- Like（点赞）
- Share（分享）
- System（系统消息）
- GiftCombo（礼物连击）
- GuardBuy（购买守护）
- RoomChange（房间信息变更）

### Message

统一的消息结构，包含：
- 消息类型
- 发送者信息
- 消息内容
- 礼物相关字段
- 超级弹幕相关字段
- 连击相关字段

## 配置说明

### 日志配置

```rust
use shared::logger::LogConfig;

let config = LogConfig {
    level: LogLevel::Info,
    log_dir: "logs".to_string(),
    max_file_size: 10, // MB
    max_backups: 5,
    console_output: true,
    file_output: true,
};

init_logger_with_config(config).unwrap();
```

### 平台配置

```rust
use shared::interface::PlatformConfig;

let config = PlatformConfig {
    user_agent: Some("Mozilla/5.0...".to_string()),
    cookie: Some("cookie_string".to_string()),
    proxy: Some("http://proxy.example.com:8080".to_string()),
    auth_token: Some("auth_token".to_string()),
    with_auth: false,
    other: None,
};
```

## 错误处理

包中定义了统一的 `PlatformError` 类型，包含：

- Network（网络错误）
- Parse（解析错误）
- Api（API错误）
- Auth（认证错误）
- NotFound（资源未找到）
- Platform（平台特定错误）
- Internal（内部错误）
- Unsupported（不支持的操作）

## 开发指南

### 添加新的平台支持

1. 创建新的平台包
2. 实现 `LivePlatform` trait
3. 在工厂函数中注册平台

### 扩展消息类型

1. 在 `MessageType` 枚举中添加新的消息类型
2. 在 `Message` 结构体中添加相应的字段
3. 更新各平台的消息解析逻辑

## 测试

运行测试：

```bash
cargo test -p shared
```

## 版本

当前版本：0.1.0

## 贡献

欢迎提交Issue和Pull Request！
