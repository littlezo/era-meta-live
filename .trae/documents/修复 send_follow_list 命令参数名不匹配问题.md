## 修复 send_follow_list 命令参数名不匹配问题

### 问题分析
根据错误信息和代码检查，发现 Tauri 2.x 会自动将 Rust 函数参数名从 snake_case 转换为 camelCase，导致前端传递的 `follow_list`（snake_case）与后端期望的 `followList`（camelCase）不匹配。

### 修复方案
修改 `platformApiService.ts` 文件中的 `sendFollowList` 方法，将参数名从 `follow_list` 改为 `followList`：

```typescript
// 修改前
await invoke<void>('send_follow_list', {
  follow_list: followList.map(streamer => ({
    // ...
  }))
});

// 修改后
await invoke<void>('send_follow_list', {
  followList: followList.map(streamer => ({
    // ...
  }))
});
```

### 验证方法
1. 编译项目，确保没有类型错误
2. 运行应用，检查是否还有 "invalid args `followList` for command `send_follow_list`" 错误
3. 确认关注列表能够成功发送到后端