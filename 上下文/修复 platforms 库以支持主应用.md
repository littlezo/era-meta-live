# 实现统一平台调用方式

## 问题分析
主应用 `era-meta-live` 正在使用**平台特定的调用方式**，如 `platforms::douyin::fetch_douyin_streamer_info`，这导致了编译错误。需要实现**统一的调用方式**，通过 `PlatformManager` 作为中央调度机制，无需直接访问特定平台模块。

## 实现步骤

### 1. 扩展统一 API 接口
- 在 `platforms/src/core.rs` 中添加统一的平台调用函数
- 实现 `fetch_streamer_info`、`get_stream_url`、`start_message_listener` 等统一函数
- 这些函数将使用 `PlatformManager` 动态获取平台实例并调用相应方法

### 2. 统一平台命令导出
- 更新 `platforms/src/commands.rs` 导出统一 API
- 确保主应用可以通过 `platforms::commands::*` 调用所有平台功能
- 移除所有平台特定的导出，只保留统一接口

### 3. 更新主应用调用方式
- 修改 `src/platform_commands.rs` 使用统一 API
- 更新 `src/main.rs` 移除平台特定调用
- 修改 `src/watch.rs` 使用统一 API
- 所有平台调用通过统一接口进行，无需直接访问平台模块

### 4. 测试构建
- 运行 `cargo check` 验证修复效果
- 确保主应用和平台库都能成功构建

## 预期结果
- 移除所有平台特定的调用方式
- 使用统一 API 访问所有平台功能
- `cargo check` 无错误
- 开发模式可以正常启动

## 关键文件修改
1. **`platforms/src/core.rs`** - 添加统一 API 函数
2. **`platforms/src/commands.rs`** - 导出统一 API
3. **`src/platform_commands.rs`** - 使用统一 API
4. **`src/main.rs`** - 移除平台特定调用
5. **`src/watch.rs`** - 使用统一 API

这个修复将实现用户要求的统一调用方式，移除平台特定的调用代码，使系统更加模块化和易于维护。