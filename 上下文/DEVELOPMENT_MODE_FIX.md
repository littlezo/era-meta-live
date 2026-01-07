# 开发模式错误修复文档

## 问题描述
运行 `pnpm tauri dev` 命令启动开发模式时，出现了编译错误和运行时错误，导致应用无法正常启动。

## 错误类型

### 1. 编译错误
- **语法错误**：缺失分号
- **类型不匹配**：HttpClient 类型使用错误
- **函数调用错误**：传递的参数类型与函数定义不匹配

### 2. 运行时错误
- **日志系统初始化失败**：`attempted to set a logger after the logging system was already initialized`

## 修复方案

### 1. 修复语法错误
在 `/src-tauri/platforms/huya/src/stream_url.rs` 文件中添加了缺失的分号。

### 2. 修复日志初始化问题
修改了 `/src-tauri/platforms/shared/src/logger.rs` 中的 `init_logger()` 函数：
- 移除了不存在的 `get_default()` 函数调用
- 改为直接使用 `try_init()` 并优雅地处理其返回结果
- 当日志系统已被初始化时，忽略初始化错误

### 3. 统一 HttpClient 使用方式
确保所有平台文件中都通过 `client.inner` 访问内部的 `reqwest::Client`，修复了以下文件：
- `/src-tauri/platforms/bilibili/src/live_list.rs`
- `/src-tauri/platforms/huya/src/stream_url.rs`
- `/src-tauri/platforms/bilibili/src/lib.rs`
- `/src-tauri/platforms/src/watch.rs`

### 4. 修复类型不匹配问题
确保函数调用时传递的参数类型与函数定义匹配，特别是在搜索功能和流 URL 获取功能中。

## 修复的文件和具体内容

### 1. `/src-tauri/platforms/shared/src/logger.rs`
**修复内容**：改进了日志初始化逻辑
```rust
// 修复前：使用了不存在的 get_default() 函数
if tracing::subscriber::get_default().is_ok() {
    // ...
}

// 修复后：直接使用 try_init() 并处理返回结果
match registry.try_init() {
    Ok(_) => {
        info!("日志系统初始化成功，日志文件: {}", log_file_path);
    },
    Err(e) => {
        eprintln!("日志系统初始化失败或已存在: {}", e);
        // 日志系统可能已被Tauri初始化，忽略此错误
    }
}
```

### 2. `/src-tauri/platforms/bilibili/src/live_list.rs`
**修复内容**：修复了 HttpClient 使用方式
```rust
// 修复前：直接使用 client
let resp = client
    .get(url)
    .header("Referer", "https://www.bilibili.com/")
    .header("Cookie", "buvid3=i;")
    .query(&params)
    .send()
    .await
    .map_err(|e| format!("Request failed: {}", e))?;

// 修复后：使用 client.inner
let resp = client.inner
    .get(url)
    .header("Referer", "https://www.bilibili.com/")
    .header("Cookie", "buvid3=i;")
    .query(&params)
    .send()
    .await
    .map_err(|e| format!("Request failed: {}", e))?;
```

### 3. `/src-tauri/platforms/huya/src/stream_url.rs`
**修复内容**：添加了缺失的分号和修复了类型不匹配
```rust
// 修复前：缺少分号
let web_stream = fetch_web_stream_data(&client.inner, room_id)
    .await
    .map_err(|e| PlatformError::Network(e.to_string()))?

// 修复后：添加了分号
let web_stream = fetch_web_stream_data(&client.inner, room_id)
    .await
    .map_err(|e| PlatformError::Network(e.to_string()))?;
```

### 4. `/src-tauri/platforms/bilibili/src/lib.rs`
**修复内容**：修复了搜索函数的类型不匹配
```rust
// 修复前：直接传递 client
let (result, raw_data) = search::search_bilibili_rooms(
    &self.http_client,
    keyword,
    page,
    self.cookie.as_deref(),
).await

// 修复后：传递 client.inner
let (result, raw_data) = search::search_bilibili_rooms(
    &self.http_client.inner,
    keyword,
    page,
    self.cookie.as_deref(),
).await
```

### 5. `/src-tauri/platforms/src/watch.rs`
**修复内容**：修复了 HttpClient 使用方式和 client 变量定义
```rust
// 修复前：缺少 client 变量定义
if let Ok(resp) = http_client.inner.get(base).headers(headers.clone()).query(&params).send().await {
    // ...
}

// 修复后：添加了 client 变量定义
let client = &http_client.inner;
if let Ok(resp) = client.get(base).headers(headers.clone()).query(&params).send().await {
    // ...
}
```

## 结果验证

运行 `pnpm tauri dev` 命令后，应用成功启动：
1. Vite 开发服务器成功启动，监听在 http://127.0.0.1:2896/
2. Rust 应用成功编译，只有一些不影响运行的警告
3. 应用正常运行，没有出现运行时错误

## 总结

通过修复语法错误、改进日志初始化逻辑、统一 HttpClient 使用方式和修复类型不匹配问题，成功解决了 `pnpm tauri dev` 运行开发模式时的错误。现在，开发模式可以正常启动，方便开发者进行应用开发和调试。

虽然编译过程中仍有一些警告（如未使用的宏定义、未使用的导入、非 snake_case 命名等），但这些警告不会影响应用的正常运行，属于代码质量优化的范畴，可以在后续的开发过程中逐步优化。