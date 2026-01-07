# 重构Platforms包实现统一直播平台接入层

## 1. 架构设计

### 1.1 核心概念
- **PlatformTrait**: 统一的直播平台抽象接口，定义所有平台必须实现的方法
- **PlatformRegistry**: 平台注册中心，负责动态注册和发现平台实现
- **WatchService**: 直播状态监控服务，独立于Tauri运行
- **EventBus**: 事件总线，实现跨平台的事件广播机制
- **RPCLayer**: JSON-RPC服务层，为外部提供统一调用接口

### 1.2 模块结构
```
platforms/
├── src/
│   ├── lib.rs               # 主入口，导出核心类型和服务
│   ├── platform/            # 平台抽象层
│   │   ├── mod.rs
│   │   ├── trait.rs         # PlatformTrait定义
│   │   └── registry.rs      # 平台注册机制
│   ├── watch/               # 直播状态监控
│   │   ├── mod.rs
│   │   ├── service.rs       # WatchService实现
│   │   └── config.rs        # 监控配置
│   ├── event/               # 事件系统
│   │   ├── mod.rs
│   │   └── bus.rs           # EventBus实现
│   ├── rpc/                 # RPC服务层
│   │   ├── mod.rs
│   │   └── server.rs        # JSON-RPC服务器
│   └── common/              # 通用工具和类型
│       ├── mod.rs
│       ├── types.rs         # 统一数据模型
│       └── error.rs         # 统一错误处理
├── bilibili/                # 平台实现
├── douyin/                  # 平台实现  
├── douyu/                   # 平台实现
├── huya/                    # 平台实现
└── shared/                  # 共享依赖
```

## 2. 实现步骤

### 2.1 定义统一平台接口 (PlatformTrait)
- 直播状态查询：`get_live_status()`
- 弹幕监听控制：`start_danmu_listener()`/`stop_danmu_listener()`
- 直播流获取：`get_stream_url()`
- 主播信息获取：`get_streamer_info()`

### 2.2 实现平台注册机制
- 动态注册：允许平台运行时注册自身实现
- 静态注册：提供宏方便平台编译时注册
- 平台发现：根据平台类型获取对应的实现

### 2.3 重构watch.rs到platforms库
- 移除Tauri依赖
- 使用新的PlatformTrait接口
- 实现独立的WatchService
- 支持配置管理

### 2.4 实现事件广播机制
- 定义统一事件类型
- 支持事件订阅和发布
- 提供跨语言事件传输

### 2.5 实现JSON-RPC服务层
- 定义RPC接口规范
- 实现JSON-RPC服务器
- 支持前端和Tauri调用

### 2.6 迁移现有平台实现
- 为每个平台实现PlatformTrait
- 移除平台代码中的Tauri依赖
- 统一数据模型和错误处理

### 2.7 集成测试
- 单元测试：验证核心功能正确性
- 集成测试：验证各平台接入层工作正常
- 端到端测试：验证完整流程

## 3. 关键设计要点

### 3.1 依赖解耦
- 使用依赖注入模式
- 抽象外部依赖（如HTTP客户端）
- 避免直接依赖Tauri API

### 3.2 数据模型标准化
- 统一直播状态定义
- 统一弹幕消息格式
- 统一错误类型

### 3.3 可扩展性
- 模块化设计，方便添加新平台
- 插件化架构，支持功能扩展
- 配置驱动，支持动态调整

### 3.4 性能优化
- 异步设计，提高并发处理能力
- 连接池管理，优化资源使用
- 批量处理，减少网络请求

## 4. 技术选型

- **异步运行时**：Tokio
- **HTTP客户端**：Reqwest
- **RPC框架**：jsonrpsee
- **序列化**：Serde
- **事件处理**：tokio::sync::broadcast

## 5. 预期成果

- 移除platforms库对Tauri的依赖
- 实现统一的直播平台接入层
- 提供JSON-RPC接口供前端和Tauri调用
- 支持直播状态监控和开播自动弹幕监听
- 实现事件广播机制
- 方便快速扩展新的直播平台

## 6. 迁移策略

1. 先设计和实现核心抽象层
2. 逐步迁移现有平台代码
3. 最后替换Tauri中的调用逻辑
4. 保持向后兼容性，确保平滑过渡