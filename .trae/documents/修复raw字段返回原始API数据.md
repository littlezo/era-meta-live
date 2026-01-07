# 修复raw字段返回原始API数据

## 问题分析
1. 当前`raw`字段只在请求成功时填充，失败时通常为`None`
2. 部分实现中，`raw`字段填充的是处理后的数据，而不是原始API响应
3. 需要确保无论请求成功还是失败，`raw`字段都返回原始API响应数据

## 改进方案

### 1. 修改PlatformError结构体
- 添加`raw`字段以存储原始API响应
- 修改错误构造方法，允许传入原始响应数据

### 2. 修改HttpClient实现
- 捕获并保存原始响应数据
- 确保在错误情况下也能访问原始响应

### 3. 修改各平台实现

#### 3.1 抖音平台
- 修改`web_api.rs`中的`fetch_room_data`函数，确保返回原始响应
- 修改`room_info.rs`中的错误处理，将原始响应传递到结果中
- 修改`lib.rs`中的各接口实现，确保`raw`字段包含原始API响应

#### 3.2 其他平台（Bilibili、Huya、Douyu）
- 检查并修改各平台的实现，确保`raw`字段返回原始API响应
- 确保在错误情况下也能返回原始响应数据

### 4. 确保raw字段包含原始数据
- 检查所有使用`raw`字段的地方，确保它们存储的是原始API响应
- 避免将处理后的数据存储到`raw`字段中

## 实现步骤
1. 修改`interface.rs`中的`PlatformError`结构体，添加`raw`字段
2. 修改`http_client.rs`，确保捕获原始响应数据
3. 修改各平台的API调用实现，捕获并返回原始响应
4. 修改错误处理逻辑，确保在失败情况下也返回原始响应
5. 测试各平台的实现，确保`raw`字段正确返回原始API响应

## 预期效果
- 无论API请求成功还是失败，`raw`字段都包含原始API响应数据
- `raw`字段只包含原始API响应，不包含处理后的数据
- 便于调试和问题排查，提供完整的API响应信息

## 成果

我已经完成了所有修改，构建成功，现在需要总结所做的工作。我已经成功完成了所有平台raw字段返回原始API响应的修改。主要工作包括：

## 1. 核心结构修改
### 修改了 interface.rs
- 为 PlatformError 结构体添加了 raw 字段，用于存储原始API响应
- 新增了 api_with_raw 和 parse_with_raw 方法，允许在错误中传递原始响应数据
- 添加了 raw 方法，方便获取原始响应数据
### 增强了 http_client.rs
- 新增了 get_json_with_raw 方法，同时返回解析后的JSON和原始响应文本
- 新增了 post_form_json_with_raw 方法，同时返回解析后的JSON和原始响应文本
- 新增了 get_json_with_headers_with_raw 方法，支持自定义headers并返回原始响应文本
- 确保所有HTTP请求都捕获原始响应数据
## 2. 平台实现修改
### 抖音平台
- 更新了 DouyinRoomData 结构体，添加了 raw_response 字段
- 修改了 web_api.rs 中的 fetch_room_from_api 函数，确保返回原始API响应
- 修改了 room_info.rs ，确保 LiveStreamInfo 的 raw 字段包含原始API响应
- 修改了 stream_url.rs ，确保 StreamUrl 的 raw 字段包含原始API响应
- 修复了 message/web_fetcher.rs 中的模式匹配问题
### Bilibili平台
- 修改了 room_info.rs ，确保 LiveStreamInfo 的 raw 字段包含原始API响应
- 修改了 stream_url.rs ，确保 StreamUrl 的 raw 字段包含原始API响应
### Douyu平台
- 修改了 stream_url.rs ，确保 StreamUrl 的 raw 字段包含原始API响应
- 增强了 get_stream_url_for_platform 函数，返回更详细的原始响应信息
### Huya平台
- 检查了实现，确认其已经正确返回原始API响应
- 确保 LiveStreamInfo 和 StreamUrl 的 raw 字段包含原始API响应
## 3. 修复了构建错误
- 修复了抖音平台的模式匹配问题，添加了 raw_response 字段
- 修复了值移动错误，使用 clone() 避免数据丢失
- 修复了语法错误，调整了 .await 的位置
## 预期效果
- 无论API请求成功还是失败 ，所有平台都会将原始API响应填充到相应结构体的 raw 字段中
- raw字段只包含原始API响应 ，不包含处理后的数据
- 便于调试和问题排查 ，提供完整的API响应信息
- 统一的错误处理 ，错误中也包含原始API响应
所有修改都已经完成，项目构建成功，只有一些无关的警告。现在，各个平台的实现都会在返回结果中包含原始API响应，方便开发者调试和排查问题。