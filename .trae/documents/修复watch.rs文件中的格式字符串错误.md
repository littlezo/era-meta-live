## 问题分析
在 `src-tauri/src/watch.rs` 文件的第83行，存在一个格式字符串错误：
```rust
logger.info(format!("Checking {} streamers 「{?}」", cfg.streamers.len(), cfg.interval_ms));
```

错误原因：在 Rust 的 `format!` 宏中，使用 `{:?}` 来表示 `Debug` trait 的格式化，而不是 `{?}`。

## 修复方案
将第83行的格式字符串从 `{?}` 改为 `{:?}`，即：
```rust
logger.info(format!("Checking {} streamers 「{:?}」", cfg.streamers.len(), cfg.interval_ms));
```

## 修复步骤
1. 打开 `src-tauri/src/watch.rs` 文件
2. 定位到第83行
3. 将 `{?}` 替换为 `{:?}`
4. 保存文件

## 预期结果
修复后，项目应该能够成功编译，不再出现格式字符串错误。