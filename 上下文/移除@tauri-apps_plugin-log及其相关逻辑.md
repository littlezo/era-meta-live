# 移除@tauri-apps/plugin-log及其相关逻辑

## 任务概述
移除src目录中对@tauri-apps/plugin-log的依赖及其相关逻辑，同时保持日志功能正常工作。

## 详细步骤
1. **修改logger.ts文件**：
   - 移除对@tauri-apps/plugin-log的导入
   - 修改Logger类的方法，移除对tauri log方法的调用，只保留console输出
   - 确保日志格式和功能保持不变

2. **检查依赖关系**：
   - 确保没有其他文件依赖@tauri-apps/plugin-log
   - 如果有其他文件使用了logger.ts中的Logger类或日志方法，确保它们仍然能正常工作

3. **验证修改**：
   - 运行应用，确保日志功能正常
   - 检查是否有编译错误

## 预期结果
- 成功移除@tauri-apps/plugin-log依赖
- 日志功能仍然正常工作，只是不再使用tauri log系统
- 没有编译错误
- 应用运行正常