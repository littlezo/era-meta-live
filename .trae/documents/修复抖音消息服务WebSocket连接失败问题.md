# 修复抖音消息服务WebSocket连接失败问题

## 问题分析
从终端日志可以看到错误：`WebSocket protocol error: Missing, duplicated or incorrect header sec-websocket-key`

通过比较当前实现和原始实现，发现问题根本原因：
1. 当前实现中，在重试循环里手动使用`Request::builder()`构建请求，没有自动添加WebSocket握手所需的`sec-websocket-key`头
2. 而`into_client_request()`方法会自动处理WebSocket协议所需的所有头信息
3. 手动构建的请求缺少必要的WebSocket协议头，导致连接失败

## 修复方案
修改`connect_and_manage_websocket`函数，在重试循环中使用`into_client_request()`方法重新创建请求，确保每次重试都包含正确的WebSocket协议头：

### 具体修改点
1. **文件**: `/Volumes/Data/project/era-meta-live/src-tauri/platforms/douyin/src/message/websocket_connection.rs`
2. **函数**: `connect_and_manage_websocket`
3. **修改内容**:
   - 删除手动构建Request的代码
   - 在重试循环中，每次都从完整的URL字符串重新创建请求
   - 使用`into_client_request()`方法自动处理WebSocket协议头
   - 保留所有必要的HTTP头插入逻辑

### 修复后流程
1. 生成完整的WebSocket URL（包含签名）
2. 在重试循环中：
   - 使用`final_wss_url_str.into_client_request()`创建请求
   - 添加所有必要的HTTP头
   - 调用`connect_async`建立连接
3. 确保每次重试都包含正确的WebSocket协议头，包括`sec-websocket-key`

## 预期效果
- WebSocket连接能够成功建立
- 消息服务能够正常工作
- 弹幕消息能够正确接收和处理

## 验证方法
1. 运行`cargo check`验证编译通过
2. 运行`pnpm tauri dev`启动应用
3. 打开抖音直播间，观察弹幕消息是否能够正常显示
4. 查看日志，确认WebSocket连接成功建立

## 风险评估
- 修复仅涉及WebSocket连接建立逻辑，不影响其他功能
- 保留了原有的重试机制，提高了连接成功率
- 使用了成熟的库方法，减少了手动处理协议头的错误风险