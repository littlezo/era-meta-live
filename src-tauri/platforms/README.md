# 统一直播平台接入层

统一直播平台接入层是一个统一的直播平台接入解决方案，提供了对多个直播平台的统一访问接口，简化了直播平台的接入和使用。

## 项目概述

本项目旨在提供一个统一的直播平台接入层，支持多种主流直播平台，包括 Bilibili、Douyin、Douyu 和 Huya。通过统一的 API 接口，用户可以方便地获取直播列表、直播间信息、直播流 URL、主播信息以及监听直播间消息。

## 架构设计

### 核心架构

```
┌─────────────────────────────────────────────────────────────────┐
│                     统一直播平台接入层                            │
├─────────────────────────────────────────────────────────────────┤
│                        前端 API 服务                              │
├─────────────────────────────────────────────────────────────────┤
│                     平台抽象层 (shared)                           │
├─────────┬───────────────┬───────────────┬───────────────────────┤
│ Bilibili │     Douyin    │     Douyu     │         Huya          │
└─────────┴───────────────┴───────────────┴───────────────────────┘
```

### 模块说明

1. **shared 包**：提供核心接口定义、数据结构、日志系统、HTTP 客户端等共享功能
2. **平台实现包**：每个平台有独立的实现包，包括 Bilibili、Douyin、Douyu 和 Huya
3. **前端 API 服务**：提供前端调用的统一 API 接口

## 核心功能

### 1. 直播列表获取

支持按分类、页码、排序等条件获取直播列表

### 2. 直播间信息获取

获取直播间的详细信息，包括标题、主播信息、直播状态、观众人数等

### 3. 直播流 URL 获取

获取直播流的播放 URL，支持多种画质选择

### 4. 主播信息获取

获取主播的详细信息，包括名称、头像、粉丝数、直播状态等

### 5. 消息监听

支持监听直播间的实时消息，包括弹幕、礼物、超级弹幕等

### 6. 认证功能

支持平台认证，包括获取认证 URL、登录、刷新令牌等

### 7. 搜索功能

支持搜索直播间和主播

## 平台支持

| 平台名称 | 支持状态 | 主要功能 |
|---------|---------|---------|
| Bilibili | ✅ 已支持 | 直播列表、直播间信息、直播流 URL、消息监听等 |
| Douyin | ✅ 已支持 | 直播列表、直播间信息、直播流 URL、消息监听等 |
| Douyu | ✅ 已支持 | 直播列表、直播间信息、直播流 URL、消息监听等 |
| Huya | ✅ 已支持 | 直播列表、直播间信息、直播流 URL、消息监听等 |

## 快速开始

### 1. 添加依赖

在 `Cargo.toml` 文件中添加所需的依赖：

```toml
dependencies = {
    "shared" = { path = "./shared" },
    "bilibili" = { path = "./bilibili" },
    "douyin" = { path = "./douyin" },
    "douyu" = { path = "./douyu" },
    "huya" = { path = "./huya" },
}
```

### 2. 创建平台实例

```rust
use shared::interface::PlatformConfig;
use bilibili::BilibiliPlatform;

let config = PlatformConfig {
    cookie: Some("bilibili_cookie_string".to_string()),
    ..Default::default()
};

let platform = BilibiliPlatform::new(config).unwrap();
```

### 3. 获取直播列表

```rust
use shared::interface::LiveListParams;

let params = LiveListParams {
    category_id: Some("1".to_string()), // 游戏分区
    page: Some(1),
    page_size: Some(10),
    sort: None,
    other: None,
};

let live_list = platform.fetch_live_list(params).await.unwrap();
println!("Found {} live rooms", live_list.items.len());
```

### 4. 获取直播间信息

```rust
let room_id = "10086";
let room_info = platform.fetch_room_info(room_id).await.unwrap();
println!("Room info: {:?}", room_info);
```

### 5. 获取直播流 URL

```rust
use shared::interface::StreamQuality;

let room_id = "10086";
let quality = Some(StreamQuality::UltraHD);
let stream_url = platform.get_stream_url(room_id, quality).await.unwrap();
println!("Stream URL: {:?}", stream_url);
```

### 6. 启动消息监听

```rust
use shared::interface::MessageCallback;

let room_id = "10086";
let callback: MessageCallback = Box::new(|message| {
    println!("Received message: {:?}", message);
});

platform.start_message_listener(room_id, callback).await.unwrap();
```

## 详细使用指南

### 平台配置

每个平台都支持一些特定的配置项，通过 `PlatformConfig` 结构体进行配置：

```rust
use shared::interface::PlatformConfig;

let config = PlatformConfig {
    user_agent: Some("Mozilla/5.0...".to_string()),
    cookie: Some("platform_cookie_string".to_string()),
    proxy: Some("http://proxy.example.com:8080".to_string()),
    auth_token: Some("auth_token_string".to_string()),
    with_auth: false,
    other: Some(serde_json::json!({
        "custom_config": "value"
    })),
};
```

### 日志配置

本项目使用 `tracing` 库进行日志记录，支持配置日志级别、输出目标等。日志配置可以通过 `log_config.toml` 文件进行配置，也可以通过代码动态配置：

```rust
use shared::logger::{self, LogConfig, LogLevel};

let log_config = LogConfig {
    level: LogLevel::Info,
    file: Some("platforms.log".to_string()),
    console: true,
    ..Default::default()
};

logger::init(log_config).unwrap();
```

### 错误处理

所有 API 调用都返回 `Result` 类型，使用 `PlatformError` 枚举表示错误类型：

```rust
use shared::interface::PlatformError;

match platform.fetch_room_info("invalid_room_id").await {
    Ok(room_info) => println!("Room info: {:?}", room_info),
    Err(e) => {
        match e {
            PlatformError::Network(msg) => println!("Network error: {}", msg),
            PlatformError::Parse(msg) => println!("Parse error: {}", msg),
            PlatformError::Api(msg) => println!("API error: {}", msg),
            PlatformError::Auth(msg) => println!("Auth error: {}", msg),
            PlatformError::NotFound(msg) => println!("Not found: {}", msg),
            _ => println!("Error: {}", e),
        }
    }
}
```

## 测试和验证

### 运行后端测试

```bash
# 运行所有平台测试
cargo test -p shared -p bilibili -p douyin -p douyu -p huya

# 运行单个平台测试
cargo test -p bilibili
```

### 构建项目

```bash
# 构建后端
cd src-tauri
cargo build

# 构建前端
pnpm build
```

### 运行开发服务器

```bash
pnpm tauri dev
```

## 贡献指南

### 代码规范

1. 遵循 Rust 官方代码规范
2. 使用 `rustfmt` 进行代码格式化
3. 使用 `clippy` 进行代码检查
4. 为新功能添加单元测试
5. 为新功能更新文档

### 提交 PR

1. Fork 仓库
2. 创建功能分支
3. 提交代码
4. 运行测试确保通过
5. 提交 PR

## 版本信息

### 当前版本

0.1.0

### 版本历史

- 0.1.0 (2026-01-06)：初始版本，支持 Bilibili、Douyin、Douyu 和 Huya 平台

## 许可证

MIT License

## 联系方式

如有问题或建议，欢迎提交 Issue 或 Pull Request。

## 致谢

感谢所有为项目做出贡献的开发者！