# 前端统一直播平台 SDK

前端统一直播平台 SDK 是一个用于访问统一直播平台接入层的前端 JavaScript SDK，提供了对多个直播平台的统一访问接口，简化了直播平台的接入和使用。

## 功能特性

- **统一 API 接口**：提供了对多个直播平台的统一访问接口
- **支持多个平台**：支持 Bilibili、Douyin、Douyu 和 Huya 平台
- **直播列表获取**：支持按分类、页码获取直播列表
- **直播间信息获取**：获取直播间详情、主播信息等
- **直播流 URL 获取**：获取不同画质的直播流 URL
- **消息监听**：支持监听直播间的弹幕、礼物等消息
- **搜索功能**：支持搜索直播间和主播
- **分类管理**：获取平台分类列表
- **认证功能**：支持平台认证、登录、刷新令牌等

## 安装

### 依赖

本 SDK 依赖 Tauri 框架，用于与后端进行通信。

## 快速开始

### 初始化 SDK

SDK 采用单例模式设计，通过 `platformApi` 全局实例访问所有功能：

```javascript
import { platformApi } from './platformApiService';

// 设置当前平台
platformApi.setCurrentPlatform('douyu');

// 获取当前平台
const currentPlatform = platformApi.getCurrentPlatform();
console.log('当前平台:', currentPlatform);
```

### 基本使用

#### 获取直播列表

```javascript
import { platformApi } from './platformApiService';

// 设置当前平台
platformApi.setCurrentPlatform('bilibili');

// 获取直播列表
try {
  const liveList = await platformApi.fetchLiveList(
    '1', // 分类 ID
    1,   // 页码
    10,  // 每页数量
    'hot', // 排序方式
    'bilibili' // 可选：覆盖当前平台
  );
  console.log('直播列表:', liveList);
} catch (error) {
  console.error('获取直播列表失败:', error);
}
```

#### 获取直播间信息

```javascript
// 获取直播间信息
try {
  const roomInfo = await platformApi.fetchRoomInfo(
    '123456', // 房间 ID
    'douyu'   // 可选：平台
  );
  console.log('直播间信息:', roomInfo);
} catch (error) {
  console.error('获取直播间信息失败:', error);
}
```

#### 获取直播流 URL

```javascript
// 获取直播流 URL
try {
  const streamUrl = await platformApi.getStreamUrl(
    '123456',   // 房间 ID
    'ultrahd',  // 画质：ultrahd, hd, sd, auto 或自定义
    'huya'      // 可选：平台
  );
  console.log('直播流 URL:', streamUrl);
} catch (error) {
  console.error('获取直播流 URL 失败:', error);
}
```

#### 消息监听

```javascript
// 启动消息监听
try {
  await platformApi.startMessageListener(
    '123456', // 房间 ID
    'douyin'  // 可选：平台
  );
  console.log('消息监听已启动');
} catch (error) {
  console.error('启动消息监听失败:', error);
}

// 订阅消息
const unsubscribe = await platformApi.subscribeToMessages(
  (message) => {
    console.log('收到消息:', message);
  },
  '123456' // 可选：指定房间 ID，不指定则接收所有房间消息
);

// 停止消息监听
try {
  await platformApi.stopMessageListener(
    '123456', // 房间 ID
    'douyin'  // 可选：平台
  );
  console.log('消息监听已停止');
  
  // 取消订阅
  unsubscribe();
} catch (error) {
  console.error('停止消息监听失败:', error);
}
```

## API 参考

### 核心方法

#### `setCurrentPlatform(platform: SupportedPlatform): void`

设置当前使用的直播平台。

- **参数**：
  - `platform`：平台名称，支持 `bilibili`、`douyin`、`douyu`、`huya`

#### `getCurrentPlatform(): SupportedPlatform`

获取当前使用的直播平台。

- **返回值**：当前平台名称

#### `fetchLiveList(categoryId?: string, page?: number, pageSize?: number, sort?: string, platform?: SupportedPlatform): Promise<UnifiedLiveList>`

获取直播列表。

- **参数**：
  - `categoryId`：分类 ID（可选）
  - `page`：页码（可选）
  - `pageSize`：每页数量（可选）
  - `sort`：排序方式（可选）
  - `platform`：平台名称（可选，覆盖当前平台）

- **返回值**：统一的直播列表

#### `fetchRoomInfo(roomId: string, platform?: SupportedPlatform): Promise<UnifiedRoomInfo>`

获取直播间信息。

- **参数**：
  - `roomId`：房间 ID
  - `platform`：平台名称（可选，覆盖当前平台）

- **返回值**：统一的直播间信息

#### `getStreamUrl(roomId: string, quality?: UnifiedStreamQuality, platform?: SupportedPlatform): Promise<UnifiedStreamUrl>`

获取直播流 URL。

- **参数**：
  - `roomId`：房间 ID
  - `quality`：画质，支持 `ultrahd`、`hd`、`sd`、`auto` 或自定义字符串
  - `platform`：平台名称（可选，覆盖当前平台）

- **返回值**：统一的直播流 URL 信息

#### `fetchStreamerInfo(streamerId: string, platform?: SupportedPlatform): Promise<UnifiedStreamerInfo>`

获取主播信息。

- **参数**：
  - `streamerId`：主播 ID
  - `platform`：平台名称（可选，覆盖当前平台）

- **返回值**：统一的主播信息

#### `searchRooms(keyword: string, page?: number, pageSize?: number, platform?: SupportedPlatform): Promise<UnifiedLiveList>`

搜索直播间。

- **参数**：
  - `keyword`：搜索关键词
  - `page`：页码（可选）
  - `pageSize`：每页数量（可选）
  - `platform`：平台名称（可选，覆盖当前平台）

- **返回值**：搜索结果列表

#### `startMessageListener(roomId: string, platform?: SupportedPlatform): Promise<void>`

启动消息监听。

- **参数**：
  - `roomId`：房间 ID
  - `platform`：平台名称（可选，覆盖当前平台）

#### `stopMessageListener(roomId: string, platform?: SupportedPlatform): Promise<void>`

停止消息监听。

- **参数**：
  - `roomId`：房间 ID
  - `platform`：平台名称（可选，覆盖当前平台）

#### `subscribeToMessages(callback: (message: UnifiedMessage) => void, roomId?: string): Promise<() => void>`

订阅消息事件。

- **参数**：
  - `callback`：消息回调函数
  - `roomId`：可选，指定房间 ID，不指定则接收所有房间消息

- **返回值**：取消订阅函数

#### `checkRoomStatus(roomId: string, platform?: SupportedPlatform): Promise<boolean>`

检查直播间状态。

- **参数**：
  - `roomId`：房间 ID
  - `platform`：平台名称（可选，覆盖当前平台）

- **返回值**：直播间是否在线

#### `fetchCategories(parentId?: string, platform?: SupportedPlatform): Promise<Category[]>`

获取分类列表。

- **参数**：
  - `parentId`：父分类 ID（可选）
  - `platform`：平台名称（可选，覆盖当前平台）

- **返回值**：分类列表

#### `getAuthUrl(platform?: SupportedPlatform): Promise<string>`

获取认证 URL。

- **参数**：
  - `platform`：平台名称（可选，覆盖当前平台）

- **返回值**：认证 URL

#### `loginWithCode(code: string, platform?: SupportedPlatform): Promise<UserInfo>`

使用认证码登录。

- **参数**：
  - `code`：认证码
  - `platform`：平台名称（可选，覆盖当前平台）

- **返回值**：用户信息

#### `refreshToken(platform?: SupportedPlatform): Promise<UserInfo>`

刷新认证令牌。

- **参数**：
  - `platform`：平台名称（可选，覆盖当前平台）

- **返回值**：用户信息

#### `getCurrentUser(platform?: SupportedPlatform): Promise<UserInfo>`

获取当前用户信息。

- **参数**：
  - `platform`：平台名称（可选，覆盖当前平台）

- **返回值**：用户信息

## 类型定义

### SupportedPlatform

支持的直播平台类型：

```typescript
type SupportedPlatform = 'bilibili' | 'douyin' | 'douyu' | 'huya';
```

### UnifiedStreamQuality

直播流质量类型：

```typescript
type UnifiedStreamQuality = 'ultrahd' | 'hd' | 'sd' | 'auto' | string;
```

### Category

分类信息：

```typescript
interface Category {
  id: string;
  name: string;
  href: string;
  subcategories?: Category[];
}
```

### UnifiedRoomInfo

统一的直播间信息：

```typescript
interface UnifiedRoomInfo {
  room_id: string;
  title: string;
  streamer_name: string;
  streamer_id: string;
  avatar_url?: string;
  cover_url?: string;
  live_status: boolean;
  viewer_count?: number;
  category_name?: string;
  category_id?: string;
  tags?: string[];
  live_status_detail: 'LIVE' | 'REPLAY' | 'OFFLINE' | 'UNKNOWN';
}
```

### UnifiedLiveList

统一的直播列表：

```typescript
interface UnifiedLiveList {
  items: UnifiedRoomInfo[];
  total?: number;
  page?: number;
  page_size?: number;
  has_more: boolean;
}
```

### UnifiedStreamerInfo

统一的主播信息：

```typescript
interface UnifiedStreamerInfo {
  streamer_id: string;
  name: string;
  avatar_url?: string;
  bio?: string;
  follower_count?: number;
  live_status: boolean;
  room_id?: string;
  room_title?: string;
  live_status_detail: 'LIVE' | 'REPLAY' | 'OFFLINE' | 'UNKNOWN';
}
```

### UnifiedStreamUrl

统一的直播流 URL 信息：

```typescript
interface UnifiedStreamUrl {
  primary_url: string;
  upstream_url?: string;
  available_streams: {
    url: string;
    format?: string | null;
    desc?: string | null;
    qn?: number | null;
    protocol?: string | null;
  }[];
  room_info?: UnifiedRoomInfo;
  streamer_info?: UnifiedStreamerInfo;
}
```

### UnifiedMessage

统一的消息类型：

```typescript
interface UnifiedMessage {
  id: string;
  room_id: string;
  user: string;
  content: string;
  user_level?: number;
  fans_club_level?: number;
  type?: string;
  color?: string;
}
```

### UserInfo

用户信息：

```typescript
interface UserInfo {
  id: string;
  name: string;
  avatar?: string;
  level?: number;
  [key: string]: unknown;
}
```

## 最佳实践

### 错误处理

SDK 所有异步方法都可能抛出错误，建议使用 try-catch 进行错误处理：

```javascript
try {
  const liveList = await platformApi.fetchLiveList('1', 1, 10, 'hot');
  console.log('直播列表:', liveList);
} catch (error) {
  console.error('获取直播列表失败:', error);
  // 显示错误提示给用户
}
```

### 平台切换

可以根据用户选择动态切换平台：

```javascript
// 用户选择了 bilibili 平台
platformApi.setCurrentPlatform('bilibili');

// 后续的 API 调用都会使用 bilibili 平台
const liveList = await platformApi.fetchLiveList('1', 1, 10);
```

### 消息监听管理

对于频繁切换直播间的场景，建议在切换前停止之前的消息监听：

```javascript
// 停止之前的监听
if (previousRoomId) {
  await platformApi.stopMessageListener(previousRoomId);
}

// 启动新的监听
await platformApi.startMessageListener(newRoomId);
```

## 常见问题

### 1. 如何处理跨平台差异？

SDK 已经处理了大部分跨平台差异，提供了统一的 API 接口。如果需要处理特定平台的特殊情况，可以通过 `platform` 参数指定平台，或者使用 `setCurrentPlatform` 切换平台。

### 2. 消息监听如何处理断线重连？

SDK 内部已经实现了断线重连机制，无需手动处理。如果重连失败，会抛出错误并停止监听。

### 3. 如何获取更高画质的直播流？

在调用 `getStreamUrl` 方法时，可以指定 `quality` 参数为 `ultrahd`（超清）或自定义画质名称。

### 4. 如何处理不同平台的认证？

每个平台的认证流程可能不同，建议根据平台文档进行处理。SDK 提供了统一的认证 API，但具体实现可能需要根据平台要求进行调整。

## 开发与测试

### 运行单元测试

```bash
pnpm vitest run
```

### 开发模式

```bash
pnpm dev
```

## 版本历史

### v1.0.0

- 初始版本，支持基本功能
- 支持 Bilibili、Douyin、Douyu 和 Huya 平台
- 提供统一的 API 接口
- 支持直播列表、直播间信息、直播流 URL、主播信息获取
- 支持消息监听、搜索、分类管理等功能

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！

## 联系方式

如有问题或建议，欢迎提交 Issue 或联系项目维护者。
