## 实现完整原始消息结构到raw字段

### 问题分析
目前的实现中，`raw`字段是手动构建的JSON，只包含部分字段。用户要求`raw`字段需要保留完整的原始数据结构，即完整的`ChatMessage`或其他消息类型。

### 解决方案
1. **添加prost-serde依赖**：为Prost生成的结构体添加`serde::Serialize`支持
2. **修改build.rs配置**：确保生成的结构体包含`Serialize` trait
3. **简化message_parsers.rs代码**：直接序列化完整消息结构到raw字段
4. **确保所有消息类型都支持序列化**

### 实施步骤

#### 1. 添加prost-serde依赖
- 在`platforms/douyin/Cargo.toml`中添加`prost-serde = "0.12"`依赖
- 确保版本与`prost = "0.12"`兼容

#### 2. 修改build.rs配置
- 修改`platforms/douyin/build.rs`，确保生成的结构体包含`Serialize` trait

#### 3. 简化message_parsers.rs代码
- 删除手动构建JSON的代码
- 直接使用`serde_json::to_string(&chat_msg)?`序列化完整消息结构
- 对所有消息类型（ChatMessage、MemberMessage、LikeMessage等）进行同样处理

#### 4. 验证修改
- 运行`cargo check`确保编译通过
- 确保所有消息类型都能正确序列化

### 预期结果
- `raw`字段包含完整的原始消息结构
- 代码更加简洁，易于维护
- 所有消息类型都能正确处理

### 代码修改点
- `platforms/douyin/Cargo.toml`：添加prost-serde依赖
- `platforms/douyin/build.rs`：修改生成配置
- `platforms/douyin/src/message/message_parsers.rs`：简化raw字段生成逻辑
- 其他消息类型的处理函数也进行类似修改

### 风险评估
- 依赖版本兼容性：确保prost-serde与现有prost版本兼容
- 序列化性能：完整消息结构可能包含大量数据，需要评估序列化性能影响
- 前端处理：前端需要能够处理完整的消息结构

### 测试计划
- 运行`cargo check`验证编译
- 测试各消息类型的序列化结果
- 确保前端能够正确处理完整的raw字段