# 不用辅助函数直接将 chat_msg 转换为 JSON 的实现方案

## 问题分析
当前项目中，`ChatMessage` 等 protobuf 结构体没有实现 `serde::Serialize` 特性，因此需要手动编写 `chat_message_to_json` 等辅助函数来将这些结构体转换为 JSON。用户希望能够直接将 `chat_msg` 转换为 JSON，避免使用这些辅助函数。

## 解决方案
通过修改 `build.rs` 文件，在 prost-build 配置中启用 serde 支持，这样生成的 protobuf 结构体就会自动实现 `serde::Serialize` 特性，从而可以直接使用 `serde_json::to_value()` 函数进行转换。

## 实现步骤

1. **修改 `build.rs` 文件**
   - 在 prost-build 配置中添加 `prost-serde` 支持
   - 具体修改：在 `prost_build::Config::new()` 后面添加 `.type_attribute(".", "#[derive(serde::Serialize)]")` 和 `.compile_well_known_types(true)`

2. **修改 `message_parsers.rs` 文件**
   - 将所有手动编写的 JSON 转换辅助函数替换为直接使用 `serde_json::to_value()`
   - 具体修改：
     - 删除 `chat_message_to_json`、`member_message_to_json`、`like_message_to_json` 等辅助函数
     - 在 `parse_chat_message` 函数中，将 `let raw_json = chat_message_to_json(&chat_msg);` 替换为 `let raw_json = serde_json::to_value(&chat_msg)?;`
     - 对其他消息解析函数进行类似修改

3. **修改其他相关文件**
   - 确保所有使用到这些辅助函数的地方都被正确修改

## 预期效果
- 不再需要手动编写繁琐的 JSON 转换辅助函数
- 可以直接使用 `serde_json::to_value(&chat_msg)` 将 `ChatMessage` 转换为 `serde_json::Value`
- 代码更加简洁，维护成本更低
- 生成的 JSON 包含完整的消息结构，而不是手动选择的字段

## 注意事项
- 需要确保 `prost-serde` 依赖已经正确添加（当前项目中已经添加）
- 修改后需要重新编译项目，以生成带有 `serde::Serialize` 特性的新 protobuf 结构体
- 需要处理可能出现的序列化错误，使用 `?` 操作符或 `match` 表达式进行错误处理