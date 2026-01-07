# 平台日志系统重构总结

## 项目背景

本项目涉及重构一个多平台直播应用的日志系统。目标是创建一个统一的日志系统，覆盖所有平台（Bilibili、Douyin、Douyu、Huya），并具有改进的功能，如每日日志轮转、结构化日志和更好的错误处理。

## 核心目标

1. 增强平台统一日志系统，实现每日日志轮转、持久化和可配置性
2. 将抖音相关构建代码迁移到 platforms/douyin/build.rs
3. 清理主 build.rs，移除抖音特定代码
4. 改进错误处理，支持错误源追踪和嵌套错误
5. 替换 log4rs 为 tracing 生态系统
6. 增强所有平台关键模块的日志记录
7. 统一各平台的日志风格和初始化方式

## 技术实现

### 日志系统架构

- **核心日志实现**：`platforms/shared/src/logger.rs`
  - 使用 tracing 生态系统实现结构化日志
  - 基于日期命名的每日日志轮转
  - 带有平台和模块上下文的 Logger 结构体
  - 使用 `Once` 实现线程安全的静态初始化

### 实现的关键特性

1. **统一日志抽象**：
   - 带有平台和模块上下文的 `Logger` 结构体
   - 支持不同日志级别（trace、debug、info、warn、error）
   - 结构化日志支持
   - 操作执行时间跟踪

2. **改进的错误处理**：
   - 带有完整错误链的错误源追踪
   - 嵌套错误支持
   - 自定义错误宏

3. **平台集成**：
   - 每个平台使用静态 LOGGER 实例
   - WebSocket、HTTP、流 URL 获取和认证日志
   - 所有平台统一的日志风格

## 修改的文件

### 核心日志文件
- **platforms/shared/src/logger.rs**：实现了基于 tracing 集成的核心日志功能
- **platforms/shared/src/macros.rs**：添加了错误处理宏

### 平台特定文件
- **platforms/bilibili/src/lib.rs**：移除了未使用的导入
- **platforms/douyin/src/lib.rs**：修复了未使用的 code 变量
- **platforms/douyu/src/lib.rs**：修复了未使用的 code 变量
- **platforms/huya/src/message.rs**：修复了未使用的 room_id_hb 变量

### 构建和配置文件
- **src-tauri/Cargo.toml**：移除了冲突的 tauri-plugin-log 依赖
- **src-tauri/src/main.rs**：修复了日志初始化

## 解决的问题

1. **日志初始化冲突**：
   - 问题：`tauri-plugin-log` 与自定义日志系统冲突
   - 解决方案：从 Cargo.toml 中移除 tauri-plugin-log 依赖
   - 错误信息："attempted to set a logger after the logging system was already initialized"

2. **未使用的变量和导入**：
   - 修复了所有平台多个文件中的未使用变量
   - 移除了共享模块中未使用的宏
   - 修复了平台方法中未使用的参数

3. **日志不一致**：
   - 标准化了所有平台的日志风格
   - 确保了正确的初始化顺序
   - 为所有模块添加了静态 LOGGER 实例

## 平台特定日志

### Bilibili
- WebSocket 连接和消息日志
- HTTP 请求/响应日志
- 流 URL 获取日志
- 认证流程日志

### Douyin
- 消息监听器日志
- 主播详情获取日志
- 流 URL 质量选择日志
- Web API 请求日志

### Douyu
- 直播列表获取日志
- 直播间信息获取日志
- 搜索功能日志
- WebSocket 消息处理日志

### Huya
- TARS 协议消息解码日志
- WebSocket 心跳日志
- 流 URL 构建日志
- 房间 ID 解析日志

## 构建系统变更

- **抖音构建代码**：从主 build.rs 迁移到 platforms/douyin/build.rs
- **清理主 build.rs**：移除了抖音特定代码
- **依赖管理**：更新日志依赖为使用 tracing 生态系统

## 日志配置

### 日志级别
- 支持 Trace、Debug、Info、Warn、Error 级别
- 可通过环境变量配置
- 控制台和文件输出支持不同日志级别

### 日志格式
- RFC3339 格式的时间戳
- 平台和模块上下文
- 文件日志使用结构化 JSON 格式
- 控制台日志使用人类可读格式

### 日志轮转
- 每日日志轮转
- 文件命名格式：`logs/platforms-YYYY-MM-DD.log`
- 自动创建日志目录（如果不存在）

## 错误处理改进

### 错误源追踪
- 带有完整错误链的错误源信息
- 嵌套错误支持
- 带有上下文的清晰错误信息

### 自定义错误宏
- `map_err_to_platform`：将错误转换为 PlatformError
- `async_map_err_to_platform`：上述的异步版本
- `wrap_err`：带有上下文的错误包装
- `async_wrap_err`：上述的异步版本

## 性能考虑

- 使用 `OnceLock` 实现线程安全的静态 LOGGER 实例
- 带有惰性格式化的高效日志
- 异步日志支持
- 禁用日志级别的最小开销

## 测试和验证

- `cargo check` 成功通过，无编译错误
- 所有平台成功初始化日志
- 日志正确写入文件
- 不再有日志初始化冲突
- 所有平台日志风格一致

## 未来增强

1. 通过应用设置添加日志级别配置
2. 实现按平台和模块的日志过滤
3. 为旧日志添加压缩功能
4. 实现集中式日志管理
5. 添加日志分析工具

## 结论

日志系统重构成功创建了一个统一、健壮的跨平台日志基础设施。新系统提供了更好的错误处理、结构化日志和改进的应用可观察性。重构还解决了日志初始化冲突，并标准化了所有模块的日志实践。