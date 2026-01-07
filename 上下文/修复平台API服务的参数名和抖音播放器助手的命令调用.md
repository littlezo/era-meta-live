## 全面修复计划

### 1. 主要问题修复：platformApiService.ts参数名不匹配

**问题**：Rust后端期望`roomId`（驼峰式），但platformApiService.ts使用`room_id`（下划线）

**修复方法**：将以下方法中的`room_id`改为`roomId`
- `fetchRoomInfo` (行145)
- `getStreamUrl` (行166) 
- `startMessageListener` (行236)
- `stopMessageListener` (行253)

### 2. 替换剩余的平台特定命令调用

#### 2.1 抖音平台
- **文件**：`douyin/playerHelper.ts`
  - **问题**：使用`get_douyin_live_stream_url_with_quality`命令
  - **修复**：替换为`platformApi.getStreamUrl`

- **文件**：`useDouyinLiveRooms.ts`
  - **问题**：使用`generate_douyin_ms_token`和`fetch_douyin_partition_rooms`命令
  - **修复**：保留现有实现，这些是抖音特定功能

#### 2.2 斗鱼平台
- **文件**：`douyu/playerHelper.ts`
  - **问题**：使用`get_stream_url_with_quality_cmd`和`start_proxy`命令
  - **修复**：替换为`platformApi.getStreamUrl`，保留`start_proxy`（代理功能）

#### 2.3 Bilibili平台
- **文件**：`bilibili/cookieHelper.ts`
  - **问题**：使用`get_bilibili_cookie`和`bootstrap_bilibili_cookie`命令
  - **修复**：保留现有实现，这些是B站特定Cookie处理

- **文件**：`stores/bilibili.ts`
  - **问题**：使用`generate_bilibili_w_webid`命令
  - **修复**：保留现有实现，这是B站特定功能

### 3. 其他修复

#### 3.1 统一静态代理服务器调用
- **文件**：`useBilibiliLiveRooms.ts`和`useProxy.ts`
  - **问题**：直接调用`start_static_proxy_server`
  - **修复**：保留现有实现，代理功能暂时不统一

#### 3.2 检查其他参数名
- **文件**：`platformApiService.ts`
  - **问题**：检查其他方法是否也存在参数名不匹配问题
  - **修复**：确保所有方法的参数名与Rust后端一致

### 4. 验证和测试

- 运行`npm run build`验证TypeScript类型检查
- 运行应用程序测试所有修复后的功能
- 检查浏览器控制台是否还有错误
- 验证各个平台的API调用是否正常工作

## 预期结果

1. **修复后不再出现**：`"invalid args 'roomId' for command 'fetch_room_info': command fetch_room_info missing required key roomId"`
2. **修复后不再出现**：`"Command get_douyin_live_stream_url_with_quality not found"`
3. 所有平台的API调用将使用统一的参数命名规范
4. 更多平台特定命令将迁移到统一API服务
5. 代码结构更加统一和可维护

## 修复文件列表

1. `/Volumes/Data/project/era-meta-live/src/platforms/common/platformApiService.ts` - 修复参数名不匹配
2. `/Volumes/Data/project/era-meta-live/src/platforms/douyin/playerHelper.ts` - 替换旧命令调用
3. `/Volumes/Data/project/era-meta-live/src/platforms/douyu/playerHelper.ts` - 替换旧命令调用

## 修复优先级

1. 高优先级：platformApiService.ts参数名修复（影响所有平台）
2. 中优先级：抖音和斗鱼播放器助手的命令替换
3. 低优先级：其他平台特定命令的评估和替换

## 风险评估

- **低风险**：参数名修复是简单的字符串替换
- **中风险**：命令替换需要确保新方法返回相同的数据结构
- **低风险**：保留的平台特定命令不会影响统一API服务的使用

## 实施步骤

1. 首先修复platformApiService.ts中的参数名
2. 然后替换抖音和斗鱼播放器助手的命令调用
3. 最后验证所有修复是否有效
4. 运行构建命令确保没有TypeScript错误