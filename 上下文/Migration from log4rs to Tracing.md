# 从 log4rs 迁移到 Tracing 计划

## 1. 依赖更新
- **移除**：`log4rs`
- **添加**：
  - `tracing = { version = "0.1", features = ["log"] }`
  - `tracing-subscriber = { version = "0.3", features = ["env-filter", "local-time", "time"] }`
  - `console-subscriber = { version = "0.4.1", optional = true }`
- **保留**：`log`（通过 tracing 的 log 特性兼容）

## 2. 代码修改

### 2.1 核心日志初始化
- 更新 `init_logger()` 函数，使用 `tracing-subscriber` 替代 `log4rs`
- 实现按天日志轮转（使用 `tracing-subscriber::fmt::MakeWriter` 或文件滚动策略）
- 配置日志格式，保持与现有格式兼容：`{d(%Y-%m-%d %H:%M:%S%.3f)} [{l}] {m}{n}`

### 2.2 Logger 结构体兼容
- 保持 `Logger` 结构体的现有 API 不变，确保向后兼容
- 内部实现使用 `tracing` 宏或适配层
- 保留所有现有方法：`trace`, `debug`, `info`, `warn`, `error`, `log_request`, `log_response_with_raw`, `log_error`, `log_with_duration`, `log_structured`

### 2.3 日志级别转换
- 保持 `LogLevel` 枚举不变
- 实现从 `LogLevel` 到 `tracing::Level` 的转换

### 2.4 路径处理
- 确保日志文件路径格式不变：`logs/platforms-{date}.log`
- 保留日志目录自动创建功能

## 3. 关键实现细节

### 3.1 按天日志轮转
- 使用 `tracing-subscriber` 的文件滚动功能或自定义实现
- 确保每天生成新的日志文件
- 考虑使用 `tracing-appender` 库实现更高级的滚动策略

### 3.2 配置兼容
- 支持通过环境变量 `RUST_LOG` 配置日志级别
- 保持与现有 `log_config.toml` 配置文件的兼容性（可选）

### 3.3 性能考虑
- 确保 `tracing` 的性能表现与 `log4rs` 相当或更好
- 合理配置 `tracing-subscriber` 的功能，避免不必要的开销

## 4. 测试验证
- 运行现有测试用例，确保日志功能正常
- 验证日志文件按天生成
- 检查日志格式是否符合预期
- 确保各平台日志记录正常

## 5. 风险评估
- **兼容性风险**：通过保持 `Logger` 结构体 API 不变，最小化对现有代码的影响
- **性能风险**：`tracing` 通常比传统日志库性能更好，但需要合理配置
- **配置风险**：需要确保 `tracing-subscriber` 的配置与现有行为一致

## 6. 预期效果
- 保持所有现有日志功能
- 提高日志系统的灵活性和可扩展性
- 支持结构化日志（原生支持，无需额外实现）
- 更好的性能和更低的开销
- 与现代 Rust 生态更好地集成