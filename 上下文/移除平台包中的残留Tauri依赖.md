# 移除平台包中的残留Tauri依赖并优化实现

## 1. 问题分析

在完成统一直播平台接入层的实现后，发现以下问题：

1. **平台包中仍残留Tauri依赖**：主要集中在各平台的message.rs、cookie.rs和stream_url.rs等文件中
2. **lib.rs中的实现未优先使用已有函数**：例如Bilibili平台的lib.rs重新实现了start_message_listener和stop_message_listener方法，而不是使用message.rs中已有的函数

## 2. 解决方案

### 2.1 核心原则

**优先使用已有实现**：如果已有对应实现，应该在lib.rs中优先使用，而不是重新实现

### 2.2 移除Tauri依赖

1. **修改已有函数，移除Tauri依赖**：
   - 移除Tauri相关导入
   - 移除#[tauri::command]宏
   - 替换Tauri特定类型（如AppHandle、Emitter）为通用类型
   - 替换tauri::async_runtime::spawn_blocking为tokio::task::spawn_blocking

2. **修改lib.rs实现**：
   - 使用修改后的已有函数，而不是重新实现
   - 确保所有LivePlatform trait方法都使用已有的平台特定函数

## 3. 实现步骤

### 3.1 处理Bilibili平台

#### 3.1.1 修改`message.rs`

1. **移除Tauri相关导入**：
   - 移除`use tauri::Emitter;
   - 移除`use tauri::command;

2. **修改`start_bilibili_message_listener`函数**：
   - 替换参数类型：
     - 移除`app_handle: tauri::AppHandle`
     - 移除`state: tauri::State<'_, shared::BilibiliMessageState>`
     - 添加`callback: MessageCallback`
     - 添加`stop_signal_tx: tokio::sync::mpsc::Sender<()>`
   - 替换`app_handle.emit`为直接调用`callback`
   - 移除Tauri特定的状态管理

3. **修改`stop_bilibili_message_listener`函数**：
   - 移除`state: tauri::State<'_, shared::BilibiliMessageState>`参数
   - 使用通用的状态管理

#### 3.1.2 修改`lib.rs`

1. **更新`start_message_listener`方法**：
   - 调用`message::start_bilibili_message_listener`函数，而不是重新实现

2. **更新`stop_message_listener`方法**：
   - 调用`message::stop_bilibili_message_listener`函数，而不是重新实现

#### 3.1.3 修改其他文件

1. **修改`stream_url.rs`**：
   - 移除Tauri相关导入
   - 移除`#[tauri::command]`宏

2. **修改`cookie.rs`**：
   - 移除Tauri相关导入
   - 移除`#[tauri::command]`宏
   - 替换`tauri::async_runtime::spawn_blocking`为`tokio::task::spawn_blocking`

### 3.2 处理Douyin平台

1. **修改`message/signature.rs`**：
   - 移除`#[tauri::command]`宏

2. **修改`message/web_fetcher.rs`**：
   - 移除`#[tauri::command]`宏

### 3.3 处理Douyu平台

1. **修改`three_cate.rs`**：
   - 移除`#[tauri::command]`宏

### 3.4 验证修改

1. 运行`npm run build`确保主应用能成功构建
2. 运行`cargo check -p platforms`确保平台包能成功编译

## 4. 预期效果

- 平台包不再依赖Tauri，提高了模块独立性
- 遵循了优先使用已有实现的原则
- 代码结构更加清晰，减少了重复实现
- 统一接口更加稳定，易于维护和扩展

## 5. 注意事项

- 确保修改后的函数签名与原有函数兼容
- 保持代码风格和现有代码一致
- 确保所有依赖关系正确
- 优先考虑修改已有代码，而不是替换它