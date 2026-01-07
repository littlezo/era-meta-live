# 为 src-tauri/platforms 下的每个包制作文档计划

## 概述
我将为 `/Volumes/Data/project/era-meta-live/src-tauri/platforms` 目录下的每个包（bilibili、douyin、douyu、huya、shared）制作完整的接口文档和技术文档。

## 文档结构

### 1. 根目录文档
- 创建 `platforms/README.md`，概述整个平台架构和设计理念

### 2. 每个平台包文档
为每个包创建 `README.md`，包含以下内容：

#### 2.1 shared 包
- 概述：共享类型和接口的核心库
- 核心模块：
  - `core.rs`：统一的平台接口和数据结构
  - `http_client.rs`：HTTP 客户端工具
  - `types.rs`：各平台特定的类型定义
- 详细的接口文档：
  - `Platform` trait：统一平台接口
  - `PlatformFactory` trait：平台工厂接口
  - 数据结构：`PlatformType`、`LiveStatus`、`StreamQuality`、`CoreLiveStreamInfo` 等

#### 2.2 bilibili 包
- 概述：B站直播平台支持
- 核心模块：
  - `platform.rs`：B站平台实现
  - 其他辅助模块：`cookie`、`message`、`live_list` 等
- 接口文档：B站特定的实现细节

#### 2.3 douyin 包
- 概述：抖音直播平台支持
- 核心模块：
  - `platform.rs`：抖音平台实现
  - 其他辅助模块：`message`、`web_api` 等
- 接口文档：抖音特定的实现细节

#### 2.4 douyu 包
- 概述：斗鱼直播平台支持
- 核心模块：
  - `platform.rs`：斗鱼平台实现
  - 其他辅助模块：`live_list`、`stream_url` 等
- 接口文档：斗鱼特定的实现细节

#### 2.5 huya 包
- 概述：虎牙直播平台支持
- 核心模块：
  - `platform.rs`：虎牙平台实现
  - 其他辅助模块：`live_list`、`message` 等
- 接口文档：虎牙特定的实现细节

## 文档格式
- 使用 Markdown 格式
- 包含代码示例
- 清晰的结构和目录
- 详细的接口说明
- 类型定义和参数说明

## 实现步骤

1. 创建 `platforms/README.md` 概述文档
2. 为 shared 包创建详细文档，重点是核心接口和数据结构
3. 为 bilibili 包创建文档
4. 为 douyin 包创建文档
5. 为 douyu 包创建文档
6. 为 huya 包创建文档
7. 检查文档完整性和一致性

## 预期输出
- 每个包都有完整的 README.md 文档
- 文档包含所有公共接口和数据结构的详细说明
- 文档清晰易懂，便于开发者使用和扩展
- 文档覆盖了每个包的核心功能和实现细节