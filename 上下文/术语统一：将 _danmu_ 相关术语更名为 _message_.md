# 术语统一计划：将 "danmu" 相关术语更名为 "message"

## 1. 前端部分（Vue/TypeScript）

### 1.1 类型定义和接口
- 将 `DanmakuMessage` 接口重命名为 `Message`
- 调整相关接口引用，如 `DanmuRenderOptions` 中的参数类型

### 1.2 组件名称和文件
- 将 `DanmuList` 组件重命名为 `MessageList`
- 更新组件引用和注册

### 1.3 核心功能模块
- 重命名 `danmakuManager.ts` 为 `messageManager.ts`
- 重命名 `danmuPlugins.ts` 为 `messagePlugins.ts`
- 重命名 `danmuOverlay.ts` 为 `messageOverlay.ts`
- 更新相关导入和引用

### 1.4 事件和通信
- 将事件名称 `message` 更改为 `message`
- 更新事件监听和触发代码

## 2. 后端部分（Rust）

### 2.1 目录结构
- 重命名 `danmu/` 目录为 `message/`（如抖音平台的 `danmu/` 子目录）
- 更新模块引用

### 2.2 函数和变量
- 重命名与弹幕相关的函数参数和变量
- 调整日志前缀，如 `[Douyin Danmaku]` 改为 `[Douyin Message]`

### 2.3 通信和事件
- 确保 Rust 端发送的事件名称与前端保持一致

## 3. 其他文件

### 3.1 配置文件
- 更新 `package.json`、`build.rs` 等文件中的相关引用

### 3.2 文档和注释
- 更新 `todo.md` 等文档中的术语
- 调整代码注释中的术语

## 4. 验证和测试

- 运行项目确保功能正常
- 检查是否有遗漏的术语引用
- 确保前端和后端通信正常

## 实施步骤

1. 首先修改类型定义和接口，确保基础类型一致
2. 然后修改核心功能模块和组件
3. 接着修改后端目录结构和代码
4. 最后更新配置文件和文档
5. 运行测试验证功能完整性

这个计划将确保所有 "danmu" 相关术语被统一更名为 "message"，同时保持代码的功能完整性和可维护性。