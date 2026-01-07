/// 错误转换宏，简化将各种错误转换为PlatformError的过程
/// 并自动记录错误日志
#[allow(unused_macros)]
macro_rules! map_err_to_platform {
    ($result:expr, $error_type:ident, $message:expr) => {
        $result.map_err(|e| {
            let platform_error = crate::interface::PlatformError::$error_type(format!("{}: {:?}", $message, e));
            
            // 记录错误日志
            error!("{}", platform_error.full_error_message());
            
            platform_error
        })
    };
    
    ($result:expr, $error_type:ident, $message:expr, $logger:expr) => {
        $result.map_err(|e| {
            let platform_error = crate::interface::PlatformError::$error_type(format!("{}: {:?}", $message, e));
            
            // 使用提供的logger记录错误日志
            $logger.error(platform_error.full_error_message());
            
            platform_error
        })
    };
}

/// 异步错误转换宏，用于async函数
#[allow(unused_macros)]
macro_rules! async_map_err_to_platform {
    ($result:expr, $error_type:ident, $message:expr) => {
        $result.await.map_err(|e| {
            let platform_error = crate::interface::PlatformError::$error_type(format!("{}: {:?}", $message, e));
            
            // 记录错误日志
            error!("{}", platform_error.full_error_message());
            
            platform_error
        })
    };
    
    ($result:expr, $error_type:ident, $message:expr, $logger:expr) => {
        $result.await.map_err(|e| {
            let platform_error = crate::interface::PlatformError::$error_type(format!("{}: {:?}", $message, e));
            
            // 使用提供的logger记录错误日志
            $logger.error(platform_error.full_error_message());
            
            platform_error
        })
    };
}

/// 包装错误宏，将错误包装为PlatformError::Wrapped并记录日志
#[allow(unused_macros)]
macro_rules! wrap_err {
    ($result:expr, $message:expr) => {
        $result.map_err(|e| {
            let platform_error = crate::interface::PlatformError::wrap(e, $message);
            
            // 记录错误日志
            error!("{}", platform_error.full_error_message());
            
            platform_error
        })
    };
    
    ($result:expr, $message:expr, $logger:expr) => {
        $result.map_err(|e| {
            let platform_error = crate::interface::PlatformError::wrap(e, $message);
            
            // 使用提供的logger记录错误日志
            $logger.error(platform_error.full_error_message());
            
            platform_error
        })
    };
}

/// 异步包装错误宏，用于async函数
#[allow(unused_macros)]
macro_rules! async_wrap_err {
    ($result:expr, $message:expr) => {
        $result.await.map_err(|e| {
            let platform_error = crate::interface::PlatformError::wrap(e, $message);
            
            // 记录错误日志
            error!("{}", platform_error.full_error_message());
            
            platform_error
        })
    };
    
    ($result:expr, $message:expr, $logger:expr) => {
        $result.await.map_err(|e| {
            let platform_error = crate::interface::PlatformError::wrap(e, $message);
            
            // 使用提供的logger记录错误日志
            $logger.error(platform_error.full_error_message());
            
            platform_error
        })
    };
}

// 导出宏
pub(crate) use map_err_to_platform;
pub(crate) use async_map_err_to_platform;
pub(crate) use wrap_err;
pub(crate) use async_wrap_err;