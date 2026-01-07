# 使用统一API替代特定平台调用，抹平平台差异

## 目标
- 确保主应用不再直接调用特定平台的API
- 所有平台相关操作通过统一API进行调用
- 抹平不同平台之间的差异
- 将watch功能迁移到platforms crate，使用统一调用方式

## 计划步骤

### 1. 完善platforms/src/watch.rs的统一API
- 确保`LiveWatchService`提供完整的统一API接口
- 验证`WatchConfig`、`FollowStreamerInput`和`FollowStatusUpdate`结构体设计合理
- 确保服务内部使用统一的平台调用方式

### 2. 更新platforms/src/commands.rs
- 导出统一的watch服务相关类型和函数
- 确保主应用可以通过统一API访问watch功能
- 移除任何可能暴露的平台特定API

### 3. 重构src/platform_commands.rs
- 添加统一的watch命令包装函数：
  - `start_follow_watch_service`
  - `stop_follow_watch_service`
  - `send_test_notification`
- 这些命令将调用platforms crate提供的统一API
- 移除任何特定平台的命令实现

### 4. 更新src/main.rs
- 移除对旧watch.rs的依赖
- 更新状态管理，使用统一的状态类型
- 更新invoke_handler，只保留统一API命令
- 确保所有平台调用都通过统一API进行

### 5. 移除特定平台API调用
- 删除src-tauri/src/watch.rs，使用platforms crate中的实现
- 确保主应用中没有直接调用特定平台API的代码
- 所有平台操作都通过统一API进行

### 6. 验证修复
- 运行`cargo check`确保编译通过
- 确保统一API能够正确处理所有平台的操作
- 验证watch功能正常工作

## 预期结果
- 主应用不再直接调用特定平台的API
- 所有平台操作通过统一API进行，抹平了平台差异
- watch功能完整迁移到platforms crate，使用统一实现
- 代码结构更加模块化，易于维护和扩展
- 支持快速添加新平台，无需修改主应用代码