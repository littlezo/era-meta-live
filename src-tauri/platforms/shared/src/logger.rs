use log::{debug, error, info, trace, warn, Level};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Once;
use chrono::Local;
use tracing;
use tracing::Level as TracingLevel;
use tracing_log::LogTracer;
use tracing_subscriber;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::prelude::*;
use tracing_subscriber::filter::LevelFilter;
use std::env;
use std::time::Duration;

use super::interface::PlatformType;

/// 静态变量，确保日志初始化只执行一次
static LOG_INIT: Once = Once::new();

/// 日志级别枚举，便于外部调用
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LogLevel> for Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => Level::Trace,
            LogLevel::Debug => Level::Debug,
            LogLevel::Info => Level::Info,
            LogLevel::Warn => Level::Warn,
            LogLevel::Error => Level::Error,
        }
    }
}

/// 日志配置结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// 日志级别
    pub level: LogLevel,
    /// 日志文件目录
    pub log_dir: String,
    /// 单个日志文件最大大小（MB）
    pub max_file_size: u64,
    /// 最大备份文件数
    pub max_backups: usize,
    /// 是否输出到控制台
    pub console_output: bool,
    /// 是否输出到文件
    pub file_output: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            log_dir: "logs".to_string(),
            max_file_size: 10,
            max_backups: 5,
            console_output: true,
            file_output: true,
        }
    }
}

/// 初始化日志系统
pub fn init_logger() -> anyhow::Result<()> {
    init_logger_with_config(LogConfig::default())
}

/// 带配置的日志初始化
pub fn init_logger_with_config(config: LogConfig) -> anyhow::Result<()> {
    LOG_INIT.call_once(|| {
        // 确保日志目录存在
        std::fs::create_dir_all(&config.log_dir).expect("Failed to create logs directory");
        
        // 获取按天命名的日志文件路径
        let log_file_path = format!("{}/platforms-{}.log", config.log_dir, Local::now().format("%Y-%m-%d"));
        // 将相对路径转换为绝对路径
        let abs_log_file_path = std::fs::canonicalize(&log_file_path).unwrap_or_else(|_| {
            // 如果文件不存在（第一次运行），尝试获取目录的绝对路径
            let log_dir_abs = std::fs::canonicalize(&config.log_dir).unwrap_or_else(|_| {
                // 如果目录也不存在，使用当前工作目录拼接
                let current_dir = std::env::current_dir().expect("Failed to get current directory");
                current_dir.join(&config.log_dir)
            });
            log_dir_abs.join(format!("platforms-{}.log", Local::now().format("%Y-%m-%d")))
        });
        println!("日志文件路径: {}", abs_log_file_path.display());
        // 尝试将log记录转发到tracing，如果失败则忽略
        if let Err(e) = LogTracer::init() {
            eprintln!("日志追踪器初始化失败: {}", e);
            // 继续执行，不中断初始化
        }
        
        // 从配置获取日志级别字符串
        let level_str = match config.level {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        };
        
        // 初始化日志系统
        // 这里我们使用更简单的方式，避免复杂的配置
        let env_filter = tracing_subscriber::filter::EnvFilter::from_default_env()
            .add_directive(format!("shared={}", level_str).parse().unwrap())
            .add_directive(format!("tokio={}", level_str).parse().unwrap())
            .add_directive(format!("reqwest={}", level_str).parse().unwrap());
        
        // 控制台输出配置
        let console_layer = fmt::layer()
            .with_timer(fmt::time::LocalTime::rfc_3339())
            .with_ansi(config.console_output);
        
        // 构建registry
        let registry = tracing_subscriber::registry()
            .with(env_filter)
            .with(console_layer);
        
        // 初始化日志系统
        match registry.try_init() {
            Ok(_) => {
                info!("日志系统初始化成功，日志文件: {}", abs_log_file_path.display());
                info!("日志级别: {:?}", config.level);
            },
            Err(e) => {
                eprintln!("日志系统初始化失败或已存在: {}", e);
                // 日志系统可能已被Tauri初始化，忽略此错误
            }
        }
    });
    // 打印完整的日志文件路径
    Ok(())
}

/// 统一日志记录工具
#[derive(Debug, Clone)]
pub struct Logger {
    platform: Option<PlatformType>,
    module: &'static str,
}

impl Logger {
    /// 创建新的日志记录器
    /// 注意：调用此方法前必须确保日志系统已通过init_logger()初始化
    pub fn new(platform: Option<PlatformType>, module: &'static str) -> Self {
        Self {
            platform,
            module,
        }
    }

    /// 获取带有平台和模块信息的日志前缀
    fn get_prefix(&self) -> String {
        match &self.platform {
            Some(platform) => format!("[{:?}] [{}]", platform, self.module),
            None => format!("[SHARED] [{}]", self.module),
        }
    }

    /// 记录Trace级别日志
    pub fn trace<T: Display>(&self, message: T) {
        trace!("{} - {}", self.get_prefix(), message);
    }

    /// 记录Debug级别日志
    pub fn debug<T: Display>(&self, message: T) {
        debug!("{} - {}", self.get_prefix(), message);
    }

    /// 记录Info级别日志
    pub fn info<T: Display>(&self, message: T) {
        info!("{} - {}", self.get_prefix(), message);
    }

    /// 记录Warn级别日志
    pub fn warn<T: Display>(&self, message: T) {
        warn!("{} - {}", self.get_prefix(), message);
    }

    /// 记录Error级别日志
    pub fn error<T: Display>(&self, message: T) {
        error!("{} - {}", self.get_prefix(), message);
    }

    /// 记录带有原始数据的响应日志
    pub fn log_response_with_raw<T: Serialize>(&self, method: &str, response: &T, raw: &Option<serde_json::Value>) {
        self.debug(format!("{} - Success", method));
        
        // 记录处理后的数据
        match serde_json::to_string_pretty(response) {
            Ok(data_str) => self.debug(format!("{} - Processed Data: {}", method, data_str)),
            Err(e) => self.error(format!("{} - Failed to serialize processed data: {}", method, e)),
        }
        
        // 记录原始数据
        if let Some(raw_data) = raw {
            match serde_json::to_string_pretty(raw_data) {
                Ok(raw_str) => self.debug(format!("{} - Raw Data: {}", method, raw_str)),
                Err(e) => self.error(format!("{} - Failed to serialize raw data: {}", method, e)),
            }
        }
    }

    /// 记录API请求日志
    pub fn log_request(&self, method: &str, url: &str, params: &Option<serde_json::Value>) {
        self.debug(format!("{} - Request to: {}", method, url));
        
        if let Some(request_params) = params {
            match serde_json::to_string_pretty(request_params) {
                Ok(params_str) => self.debug(format!("{} - Request Params: {}", method, params_str)),
                Err(e) => self.error(format!("{} - Failed to serialize request params: {}", method, e)),
            }
        }
    }

    /// 记录API错误日志
    pub fn log_error(&self, method: &str, error: &anyhow::Error) {
        self.error(format!("{} - Error: {}", method, error));
        
        // 记录完整的错误链
        let mut cause = error.source();
        let mut depth = 1;
        while let Some(c) = cause {
            self.error(format!("{} - Cause {}: {}", method, depth, c));
            cause = c.source();
            depth += 1;
        }
    }

    /// 记录带有执行时间的日志
    pub fn log_with_duration<T, F>(&self, method: &str, f: F) -> anyhow::Result<T>
    where
        F: FnOnce() -> anyhow::Result<T>,
    {
        let start = std::time::Instant::now();
        self.debug(format!("{} - Start", method));
        
        match f() {
            Ok(result) => {
                let duration = start.elapsed();
                self.debug(format!("{} - Completed in {:?}", method, duration));
                Ok(result)
            },
            Err(e) => {
                let duration = start.elapsed();
                self.error(format!("{} - Failed in {:?}: {}", method, duration, e));
                
                // 记录完整的错误链
                let mut cause = e.source();
                let mut depth = 1;
                while let Some(c) = cause {
                    self.error(format!("{} - Cause {}: {}", method, depth, c));
                    cause = c.source();
                    depth += 1;
                }
                
                Err(e)
            }
        }
    }

    /// 记录结构化日志
    pub fn log_structured<T: serde::Serialize>(&self, level: LogLevel, message: &str, data: &T) {
        let data_str = match serde_json::to_string(data) {
            Ok(s) => s,
            Err(e) => format!("Failed to serialize data: {}", e),
        };
        
        let log_message = format!("{} - {} - {}", self.get_prefix(), message, data_str);
        
        match level {
            LogLevel::Trace => trace!("{}", log_message),
            LogLevel::Debug => debug!("{}", log_message),
            LogLevel::Info => info!("{}", log_message),
            LogLevel::Warn => warn!("{}", log_message),
            LogLevel::Error => error!("{}", log_message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface::PlatformType;
    use std::sync::Arc;
    use std::sync::Mutex;
    
    #[test]
    fn test_log_level_conversion() {
        // 测试 LogLevel 到 log::Level 的转换
        assert_eq!(Level::from(LogLevel::Trace), Level::Trace);
        assert_eq!(Level::from(LogLevel::Debug), Level::Debug);
        assert_eq!(Level::from(LogLevel::Info), Level::Info);
        assert_eq!(Level::from(LogLevel::Warn), Level::Warn);
        assert_eq!(Level::from(LogLevel::Error), Level::Error);
    }
    
    #[test]
    fn test_log_config_default() {
        // 测试 LogConfig 默认值
        let config = LogConfig::default();
        assert_eq!(config.level, LogLevel::Info);
        assert_eq!(config.log_dir, "logs");
        assert_eq!(config.max_file_size, 10);
        assert_eq!(config.max_backups, 5);
        assert!(config.console_output);
        assert!(config.file_output);
    }
    
    #[tokio::test]
    async fn test_logger_initialization() {
        // 测试日志初始化
        let result = init_logger();
        assert!(result.is_ok());
        
        // 测试带配置的日志初始化
        let custom_config = LogConfig {
            level: LogLevel::Debug,
            log_dir: "test_logs".to_string(),
            max_file_size: 5,
            max_backups: 3,
            console_output: true,
            file_output: false,
        };
        
        let result = init_logger_with_config(custom_config);
        assert!(result.is_ok());
        
        // 清理测试日志目录
        let _ = std::fs::remove_dir_all("test_logs");
    }
    
    #[tokio::test]
    async fn test_logger_functions() {
        // 初始化日志系统
        let _ = init_logger();
        
        // 创建日志记录器
        let logger = Logger::new(Some(PlatformType::Douyu), "test_module");
        
        // 测试不同级别的日志记录
        logger.trace("Test trace log");
        logger.debug("Test debug log");
        logger.info("Test info log");
        logger.warn("Test warn log");
        logger.error("Test error log");
        
        // 测试结构化日志
        logger.log_structured(
            LogLevel::Info,
            "Test structured log",
            &serde_json::json!({"key": "value", "number": 42})
        );
        
        // 测试带执行时间的日志
        let result = logger.log_with_duration("test_operation", || {
            Ok("success")
        });
        assert_eq!(result.unwrap(), "success");
        
        // 测试请求日志
        logger.log_request("test_request", "https://example.com", &Some(serde_json::json!({"param": "value"})));
        
        // 测试响应日志
        logger.log_response_with_raw("test_response", &serde_json::json!({"status": "ok"}), &Some(serde_json::json!({"raw": "data"})));
    }
    
    #[test]
    fn test_logger_prefix() {
        // 测试带有平台信息的日志前缀
        let logger_with_platform = Logger::new(Some(PlatformType::Bilibili), "test_module");
        let prefix_with_platform = logger_with_platform.get_prefix();
        assert!(prefix_with_platform.contains("Bilibili"));
        assert!(prefix_with_platform.contains("test_module"));
        
        // 测试不带平台信息的日志前缀
        let logger_without_platform = Logger::new(None, "test_module");
        let prefix_without_platform = logger_without_platform.get_prefix();
        assert!(prefix_without_platform.contains("SHARED"));
        assert!(prefix_without_platform.contains("test_module"));
    }
}