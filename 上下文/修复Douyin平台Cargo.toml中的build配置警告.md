### 问题分析
从终端输出中看到警告：`warning: /Volumes/Data/project/era-meta-live/src-tauri/platforms/douyin/Cargo.toml: unused manifest key: lib.build`

### 根本原因
根据Rust Cargo规范，`build`配置项应该直接放在`[package]`部分，用于指定构建脚本的路径，而不是在`[lib]`部分。

### 解决方案
修改`/Volumes/Data/project/era-meta-live/src-tauri/platforms/douyin/Cargo.toml`文件，将`build = "build.rs"`从`[lib]`部分移动到`[package]`部分。

### 具体修改步骤
1. 将第10行的`build = "build.rs"`从`[lib]`部分剪切
2. 将其粘贴到`[package]`部分（第6行之后）
3. 保持其他配置不变

### 预期效果
修复后，运行`pnpm tauri dev`命令时，该警告将消失，构建过程更加规范。