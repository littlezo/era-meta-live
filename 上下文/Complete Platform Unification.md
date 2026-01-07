## 修复Terminal#1-225中的编译错误

### 分析当前错误
从终端输出可以看到，应用程序编译失败，存在以下具体错误：

1. **Bilibili live_list.rs 错误**
   - `E0599`: 方法 `header` 未找到，因为 `get` 方法返回的是 `Future` 而不是 `RequestBuilder`
   - `E0282`: 需要类型标注，无法推断 `resp` 和 `text` 的类型

2. **Huya stream_url.rs 错误**
   - `E0308`: 类型不匹配 - 期望 `&Client`，但得到 `&Arc<HttpClient>`
   - 发生在 `fetch_web_stream_data` 调用（第492和563行）

3. **Bilibili lib.rs 错误**
   - `E0308`: 类型不匹配 - 期望 `&Client`，但得到 `&Arc<HttpClient>`
   - 发生在 `search_bilibili_rooms` 调用（第294行）

### 修复计划

#### 1. 修复 Bilibili live_list.rs
   - 问题：使用共享 `HttpClient` 时错误地调用了 `reqwest::Client` 的方法链
   - 解决方案：
     - 改用 `HttpClient` 的专用方法（如 `get_text_with_headers`）
     - 或使用 `HttpClient.inner` 访问内部 `reqwest::Client`
     - 添加适当的类型标注

#### 2. 修复 Huya stream_url.rs
   - 问题：`fetch_web_stream_data` 函数期望 `&reqwest::Client`，但收到 `&Arc<HttpClient>`
   - 解决方案：
     - 修改 `fetch_web_stream_data` 函数签名以接受 `&HttpClient`
     - 或在调用时传递 `&client.inner`
     - 更新函数内部以使用 `HttpClient` 的方法

#### 3. 修复 Bilibili lib.rs
   - 问题：`search_bilibili_rooms` 函数期望 `&reqwest::Client`，但收到 `&Arc<HttpClient>`
   - 解决方案：
     - 修改 `search_bilibili_rooms` 函数签名以接受 `&HttpClient`
     - 或在调用时传递 `&self.http_client.inner`
     - 更新函数内部以使用 `HttpClient` 的方法

### 实施步骤

1. **修复 Bilibili live_list.rs**
   - 分析 `live_list.rs` 中 `fetch_bilibili_live_list` 函数
   - 更新 HTTP 请求代码以正确使用 `HttpClient`
   - 添加必要的类型标注

2. **修复 Huya stream_url.rs**
   - 分析 `fetch_web_stream_data` 函数
   - 修改函数签名和内部实现以使用 `HttpClient`
   - 更新调用点

3. **修复 Bilibili lib.rs**
   - 分析 `search_bilibili_rooms` 函数
   - 修改函数签名和内部实现以使用 `HttpClient`
   - 更新调用点

4. **测试编译**
   - 运行 `cargo build` 验证所有错误已修复
   - 运行 `pnpm tauri dev` 确保应用正常启动

### 预期结果
- 所有编译错误消失
- 应用程序成功启动
- 所有平台正常工作
- 保持代码的统一和一致性

这个计划将解决当前编译失败的问题，同时推进平台统一的目标，确保所有平台使用一致的 HTTP 客户端 API。