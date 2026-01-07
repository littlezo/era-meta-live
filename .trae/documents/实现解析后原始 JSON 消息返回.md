## 实现解析后原始 JSON 消息返回

### 需求分析
用户要求 `raw` 字段应该是解析后的原始 JSON 消息，而不是包含 base64 编码的原始字节。这意味着我们需要将解析后的 protobuf 消息直接序列化为 JSON 格式。

### 实施步骤

1. **添加 prost-serde 支持**
   - 修改 `platforms/douyin/Cargo.toml`，添加 `prost-serde` 依赖
   - 更新 `platforms/douyin/build.rs`，使用 `protoc-gen-prost-serde` 插件为生成的 protobuf 类型添加 serde 支持

2. **更新消息解析函数**
   - 修改所有消息解析函数，将解析后的 protobuf 消息直接序列化为 JSON 作为 `raw` 字段的值
   - 移除之前的 base64 编码逻辑
   - 确保 `raw` 字段包含完整的解析后消息的 JSON 表示

3. **更新消息处理逻辑**
   - 确保 `message_handler.rs` 中的代码正确处理新的 `raw` 字段格式
   - 验证 `PlatformMessage.raw` 字段正确接收和传递解析后的 JSON 消息

4. **测试和验证**
   - 运行 `cargo check` 确保编译通过
   - 运行 `cargo build` 验证构建成功
   - 确保所有消息类型都能正确返回解析后的原始 JSON 消息

### 预期结果
- 所有消息解析函数返回的 `MessageFrontendPayload.raw` 字段包含解析后的 protobuf 消息的 JSON 表示
- `PlatformMessage.raw` 字段正确接收和传递这些 JSON 消息
- 前端能够收到完整的解析后原始 JSON 消息

### 注意事项
- 确保为所有 protobuf 消息类型添加 serde 支持
- 处理好嵌套消息和枚举类型的 JSON 序列化
- 确保 JSON 格式符合预期，便于前端处理
- 保持代码的可读性和可维护性