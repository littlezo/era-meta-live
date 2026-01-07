# Douyin Platform Package

Douyin 平台包是统一直播平台接入层的 Douyin 平台实现，提供了 Douyin 直播平台的各项功能接入。

## 功能实现

### 1. 直播列表获取

实现了 Douyin 平台的直播列表获取功能，支持按分类和页码获取直播列表。

### 2. 直播间信息获取

实现了 Douyin 平台的直播间信息获取功能，包括直播间标题、主播信息、直播状态等。

### 3. 直播流 URL 获取

实现了 Douyin 平台的直播流 URL 获取功能，支持多种画质选择。

### 4. 主播信息获取

实现了 Douyin 平台的主播信息获取功能，包括主播名称、头像、直播状态等。

### 5. 消息监听

实现了 Douyin 平台的消息监听功能，支持监听直播间的弹幕、礼物、超级弹幕等消息。

### 6. 认证相关功能

实现了 Douyin 平台的认证相关功能，包括获取认证 URL、登录、刷新令牌等。

## 配置说明

### 平台配置

```rust
use shared::interface::PlatformConfig;

let config = PlatformConfig {
    // Douyin 平台配置项...
    ..Default::default()
};
```

### 日志配置

Douyin 平台使用 shared 包中的日志系统，支持配置日志级别、输出目标等。

## 使用示例

### 创建平台实例

```rust
use douyin::DouyinPlatform;
use shared::interface::PlatformConfig;

let config = PlatformConfig {
    ..Default::default()
};

let platform = DouyinPlatform::new(config).unwrap();
```

### 获取直播列表

```rust
use shared::interface::LiveListParams;

let params = LiveListParams {
    category_id: Some("game".to_string()), // 游戏分区
    page: Some(1),
    page_size: Some(10),
    sort: None,
    other: None,
};

let live_list = platform.fetch_live_list(params).await.unwrap();
println!("Found {} live rooms", live_list.items.len());
```

### 获取直播间信息

```rust
let room_id = "123456789";
let room_info = platform.fetch_room_info(room_id).await.unwrap();
println!("Room info: {:?}", room_info);
```

### 获取直播流 URL

```rust
use shared::interface::StreamQuality;

let room_id = "123456789";
let quality = Some(StreamQuality::UltraHD);
let stream_url = platform.get_stream_url(room_id, quality).await.unwrap();
println!("Stream URL: {:?}", stream_url);
```

### 启动消息监听

```rust
use shared::interface::MessageCallback;

let room_id = "123456789";
let callback: MessageCallback = Box::new(|message| {
    println!("Received message: {:?}", message);
});

platform.start_message_listener(room_id, callback).await.unwrap();
```

## 测试

运行测试：

```bash
cargo test -p douyin
```

## 版本

当前版本：0.1.0

## 贡献

欢迎提交Issue和Pull Request！