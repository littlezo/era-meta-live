# 术语统一计划：将 "danmu" 和 "danmuke" 替换为 "message"

## 1. 搜索与识别
- 使用 grep 搜索整个代码库，识别所有包含 "danmu" 和 "danmuke" 的文件
- 分类整理需要修改的内容：文件路径、代码引用、注释、字符串等

## 2. 后端代码修改（Rust）
### 2.1 文件系统更改
- 将 `platforms/douyin/src/danmu/` 目录重命名为 `platforms/douyin/src/message/`
- 更新 `build.rs` 中的文件路径引用

### 2.2 模块与函数修改
- 更新 `platforms/douyin/src/mod.rs` 中的模块引用，将 `douyin_danmu_listener` 替换为 `douyin_message_listener`
- 更新 `platforms/douyu/src/mod.rs` 中的模块引用，将 `danmu_start` 替换为 `message_start`
- 更新所有相关函数名称，如 `start_douyin_danmu_listener` 替换为 `start_douyin_message_listener`

### 2.3 类型与结构修改
- 更新所有相关类型定义，如将 `DanmuServer` 替换为 `MessageServer`（如需要）
- 更新日志消息和字符串中的术语

## 3. 前端代码修改（TypeScript/Vue）
### 3.1 组件与工具修改
- 检查并更新 `messageOverlay.ts` 中的相关引用
- 更新 `types.ts` 中的类型定义
- 更新组件模板和样式中的相关术语

### 3.2 API 与事件修改
- 更新 API 调用和事件名称中的术语
- 更新状态管理中的相关变量名称

## 4. 构建与测试
- 运行 `cargo build` 测试后端编译
- 运行 `pnpm build` 测试前端构建
- 确保所有测试通过，没有编译错误

## 5. 验证与清理
- 再次搜索代码库，确保所有 "danmu" 和 "danmuke" 实例都已替换为 "message"
- 清理任何临时文件或残留的引用

## 注意事项
- 对于外部库引用（如 `DanmuJs`），保持原样，不进行修改
- 对于 URL 中的 "danmuproxy" 等特定术语，根据上下文决定是否修改
- 确保修改过程中保持代码的语法正确性和功能完整性