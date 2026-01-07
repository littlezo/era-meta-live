### 为所有平台添加详细日志记录

为了全面监控和调试所有平台的API调用，我将在以下几个层次添加详细的日志记录：

#### 1. HTTP客户端层 (`http_client.rs`)
- 修改`get_json_with_headers`方法，在JSON解析失败时记录完整的响应内容
- 添加响应状态码、响应长度和响应内容的日志记录
- 改进错误处理，提供更详细的错误信息
- 为所有HTTP请求和响应添加统一的日志格式

#### 2. 平台命令层 (`platform_commands.rs`)
- 为所有命令函数添加一致的日志记录，包括：
  - `fetch_categories`
  - `fetch_live_list`
  - `get_stream_url`
  - `search_rooms`
  - `start_message_listener`
  - `fetch_streamer_info`
  - `stop_message_listener`
  - 认证相关命令
- 每个命令添加：
  - 开始和结束日志
  - 参数日志
  - 错误日志
  - 成功日志
- 采用与`fetch_room_info`相同的日志格式

#### 3. 平台工厂层 (`factory.rs`)
- 在平台注册和创建过程中添加日志记录
- 记录平台实例的缓存和复用情况
- 为`register_all_platforms`函数添加详细日志

#### 4. 平台API层
- 确保所有平台的API调用都有适当的日志记录
- 重点关注抖音平台的API调用，添加更详细的调试信息
- 记录API请求的完整URL、参数和响应

#### 5. 日志级别配置
- 确保所有调试日志能够被正确输出
- 使用适当的日志级别（debug、info、error）
- 确保日志格式一致，便于分析和调试

通过这些修改，我们将能够全面监控和调试所有平台的API调用，包括请求发送、服务器响应和结果处理的全过程，从而提高系统的可维护性和调试效率。