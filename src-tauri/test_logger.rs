use std::path::Path;
use platforms::shared::logger::{init_logger, Logger, LogLevel};
use platforms::shared::interface::PlatformType;

fn main() {
    println!("开始测试日志系统...");
    
    // 初始化日志系统
    if let Err(e) = init_logger() {
        eprintln!("日志系统初始化失败: {}", e);
        return;
    }
    
    println!("日志系统初始化成功");
    
    // 创建日志记录器
    let logger = Logger::new(Some(PlatformType::Douyin), "test");
    
    // 记录不同级别的日志
    logger.trace("这是一条Trace级别的日志");
    logger.debug("这是一条Debug级别的日志");
    logger.info("这是一条Info级别的日志");
    logger.warn("这是一条Warn级别的日志");
    logger.error("这是一条Error级别的日志");
    
    // 测试执行时间日志
    let result = logger.log_with_duration("test_operation", || {
        std::thread::sleep(std::time::Duration::from_millis(50));
        Ok("success")
    });
    
    println!("测试操作结果: {:?}", result);
    
    // 检查日志文件是否生成
    let log_file_path = format!("logs/platforms-{}.log", chrono::Local::now().format("%Y-%m-%d"));
    if Path::new(&log_file_path).exists() {
        println!("日志文件已生成: {}", log_file_path);
        
        // 显示日志文件内容
        match std::fs::read_to_string(&log_file_path) {
            Ok(content) => {
                println!("\n日志文件内容:");
                println!("{}", content);
            },
            Err(e) => {
                eprintln!("读取日志文件失败: {}", e);
            }
        }
    } else {
        println!("日志文件未生成: {}", log_file_path);
    }
    
    println!("\n日志系统测试完成");
}