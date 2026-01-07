## 计划：将MessageFrontendPayload.raw字段改为serde_json::Value类型

### 问题分析
- 当前`MessageFrontendPayload.raw`字段定义为`String`类型
- 但在`message_parsers.rs`中，我们已经将消息序列化为`serde_json::Value`，然后转换为字符串
- 而`interface.rs`中的`Message.raw`字段已经是`Option<serde_json::Value>`类型
- 这种不一致导致了不必要的字符串转换开销

### 实现步骤

1. **修改MessageFrontendPayload定义**
   - 将`platforms/shared/src/types.rs`中`MessageFrontendPayload.raw`字段类型从`String`改为`serde_json::Value`

2. **更新message_parsers.rs中的解析函数**
   - 修改所有`parse_*`函数，不再使用`serde_json::to_string()`转换，直接将序列化后的`serde_json::Value`赋值给`raw`字段
   - 例如，将`let raw_string = serde_json::to_string(&raw_json)?;`改为直接使用`raw: raw_json`

3. **更新message_handler.rs中的使用**
   - 修改`message_handler.rs`中创建`PlatformMessage`的代码，不再需要`serde_json::from_str()`转换，直接使用`raw_value: Some(chat_payload.raw)`

4. **验证编译**
   - 运行`cargo check`确保所有修改都能正常编译

### 预期效果
- 减少不必要的JSON序列化和反序列化开销
- 保持类型一致性，`MessageFrontendPayload.raw`和`Message.raw`都使用`serde_json::Value`类型
- 简化代码，不再需要额外的字符串转换

### 影响范围
- `platforms/shared/src/types.rs`：修改`MessageFrontendPayload`结构体定义
- `platforms/douyin/src/message/message_parsers.rs`：修改所有`parse_*`函数
- `platforms/douyin/src/message/message_handler.rs`：修改创建`PlatformMessage`的代码

这个变更将使代码更加高效和一致，符合用户的要求。