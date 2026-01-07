禁止使用 npm 改用 pnpm
tauri 版本 2.x 文档https://v2.tauri.app/zh-cn/start/
开发调试命令 使用 pnpm tauri dev 注意改命令非后台运行 得异步读取日志判定
当前正常重构改项目 那你可用需要通过 上下文 来获取重构历史过程
对于功能异常和缺失 你可能需要 通过 git diff 或者 #Folder: `/Volumes/Data/project/dtv` 原始项目 来从历史了解重构前原实现来修复
对于 rust 尽可能 使用 cargo check 来检查编译错误 而不是 cargo build