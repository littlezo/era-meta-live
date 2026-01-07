# 重构计划：将 src-tauri/src/platforms 重构为单独模块包

## 步骤1：创建新的 platforms 目录结构
- 将 `src-tauri/src/platforms` 目录移动到 `src-tauri/platforms`
- 创建 `platforms/Cargo.toml` 文件
- 将 `platforms/mod.rs` 重命名为 `platforms/src/lib.rs`

## 步骤2：配置 platforms/Cargo.toml
- 设置基本的包信息（name, version, edition）
- 从原 Cargo.toml 中提取 platforms 模块使用的依赖项
- 添加必要的依赖项到 platforms/Cargo.toml

## 步骤3：更新 src-tauri/Cargo.toml
- 添加 `platforms = { path = "platforms" }` 依赖
- 声明 platforms 为 workspace 成员
- 移除 src 目录未使用的依赖项

## 步骤4：验证重构结果
- 对 platforms 执行 `cargo check` 确保没有错误
- 对 src-tauri 执行 `cargo check` 确保没有错误

## 依赖项分析
根据 Cargo.toml 和代码结构，platforms 模块可能使用的依赖项包括：
- serde, serde_json
- tokio, futures-util
- reqwest, url
- regex
- md-5, base64, hex
- thiserror, anyhow
- once_cell, lazy_static
- prost, prost-types
- tars-stream
- tungstenite, tokio-tungstenite
- log

src 目录下其他文件（main.rs, lib.rs, proxy.rs, watch.rs）可能使用的依赖项包括：
- tauri 及其插件
- window-vibrancy
- deno_core
- actix-web, actix-cors
- awc
- bytes
- chrono
- specta
- cookie
- html-escape
- brotlic

## 执行顺序
1. 移动目录和创建文件结构
2. 配置 platforms/Cargo.toml
3. 更新 src-tauri/Cargo.toml
4. 执行 cargo check 验证

这个计划将确保 platforms 模块被正确重构为独立的模块包，同时保持项目的整体功能正常。