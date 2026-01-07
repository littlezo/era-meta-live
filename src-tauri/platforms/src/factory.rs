use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use once_cell::sync::OnceCell;
use log::{debug, error, info, warn};

use shared::interface::{LivePlatform, PlatformConfig, PlatformCreator, PlatformError, PlatformType};

// 平台工厂结构体
#[derive(Default)]
pub struct PlatformFactory {
    // 注册的平台创建器
    platform_creators: RwLock<HashMap<PlatformType, PlatformCreator>>,
    // 已创建的平台实例缓存
    platform_instances: RwLock<HashMap<(PlatformType, String), Arc<dyn LivePlatform>>>,
    // 平台是否已注册的标志
    platforms_registered: RwLock<bool>,
}

impl PlatformFactory {
    // 创建新的平台工厂实例
    pub fn new() -> Self {
        Self::default()
    }
    
    // 获取全局单例实例
    pub fn global() -> Arc<Self> {
        static INSTANCE: once_cell::sync::OnceCell<Arc<PlatformFactory>> = once_cell::sync::OnceCell::new();
        INSTANCE.get_or_init(|| Arc::new(Self::new())).clone()
    }
    
    // 注册平台创建器
    pub async fn register_platform(
        &self,
        platform_type: PlatformType,
        creator: PlatformCreator
    ) -> Result<(), PlatformError> {
        let mut creators = self.platform_creators.write().await;
        if creators.contains_key(&platform_type) {
            warn!("Platform {:?} is already registered, overwriting...", platform_type);
        }
        creators.insert(platform_type.clone(), creator);
        info!("Successfully registered platform: {:?}", platform_type);
        Ok(())
    }
    
    // 注销平台
    pub async fn unregister_platform(
        &self,
        platform_type: PlatformType
    ) -> Result<(), PlatformError> {
        let mut creators = self.platform_creators.write().await;
        if creators.remove(&platform_type).is_some() {
            info!("Successfully unregistered platform: {:?}", platform_type);
        } else {
            warn!("Attempted to unregister non-existent platform: {:?}", platform_type);
        }
        
        // 同时移除相关的实例缓存
        let mut instances = self.platform_instances.write().await;
        let removed_count = instances.len();
        instances.retain(|(pt, _), _| pt != &platform_type);
        let remaining_count = instances.len();
        if removed_count > remaining_count {
            debug!("Removed {} cached instances for platform {:?}", removed_count - remaining_count, platform_type);
        }
        
        Ok(())
    }
    
    // 创建平台实例
    pub async fn create_platform(
        &self,
        platform_type: PlatformType,
        config: PlatformConfig
    ) -> Result<Arc<dyn LivePlatform>, PlatformError> {
        // 检查平台是否已注册，如果没有，尝试注册所有平台
        let creators = self.platform_creators.read().await;
        if !creators.contains_key(&platform_type) {
            debug!("Platform {:?} not registered, attempting to register all platforms...", platform_type);
            // 释放读锁，获取写锁注册平台
            drop(creators);
            self.register_all_platforms().await?;
            // 重新获取读锁
            let creators = self.platform_creators.read().await;
            if !creators.contains_key(&platform_type) {
                error!("Platform {:?} still not registered after attempting to register all platforms", platform_type);
                return Err(PlatformError::Unsupported(format!("Platform {:?} not registered", platform_type)));
            }
        }
        
        // 现在平台应该已经注册，可以创建实例
        let creators = self.platform_creators.read().await;
        if let Some(creator) = creators.get(&platform_type) {
            debug!("Creating platform instance for {:?} with config: {:?}", platform_type, config);
            let instance = creator(config)?;
            info!("Successfully created platform instance for {:?}", platform_type);
            Ok(instance.into())
        } else {
            error!("Platform {:?} not registered, cannot create instance", platform_type);
            Err(PlatformError::Unsupported(format!("Platform {:?} not registered", platform_type)))
        }
    }
    
    // 获取平台实例（带缓存）
    pub async fn get_platform(
        &self,
        platform_type: PlatformType,
        config: PlatformConfig
    ) -> Result<Arc<dyn LivePlatform>, PlatformError> {
        // 创建缓存键（基于平台类型和配置的哈希值）
        let config_hash = serde_json::to_string(&config)
            .map_err(|e| PlatformError::Internal(format!("Failed to serialize config: {:?}", e)))?;
        let cache_key = (platform_type.clone(), config_hash);
        debug!("Getting platform instance for {:?} with cache key: {:?}", platform_type, cache_key.0);
        
        // 检查缓存中是否已有实例
        {
            let instances = self.platform_instances.read().await;
            if let Some(instance) = instances.get(&cache_key) {
                debug!("Cache hit for platform {:?}", platform_type);
                return Ok(instance.clone());
            }
        }
        debug!("Cache miss for platform {:?}, creating new instance", platform_type);
        
        // 如果缓存中没有，创建新实例
        let instance = self.create_platform(platform_type, config).await?;
        
        // 将实例存入缓存
        let mut instances = self.platform_instances.write().await;
        instances.insert(cache_key, instance.clone());
        debug!("Cached platform instance for {:?}", instance.platform_type());
        
        Ok(instance)
    }
    
    // 获取平台实例（使用默认配置）
    pub async fn get_platform_with_default_config(
        &self,
        platform_type: PlatformType
    ) -> Result<Arc<dyn LivePlatform>, PlatformError> {
        debug!("Getting platform instance with default config for {:?}", platform_type);
        let result = self.get_platform(platform_type.clone(), PlatformConfig::default()).await;
        match &result {
            Ok(_) => {
                info!("Successfully got platform instance with default config for {:?}", platform_type);
            },
            Err(e) => {
                error!("Failed to get platform instance with default config for {:?}: {}", platform_type, e);
            }
        }
        result
    }
    
    // 列出已注册的平台类型
    pub async fn list_registered_platforms(
        &self
    ) -> Vec<PlatformType> {
        let creators = self.platform_creators.read().await;
        let platforms: Vec<PlatformType> = creators.keys().cloned().collect();
        debug!("Listing registered platforms: {:?}", platforms);
        platforms
    }
    
    // 清除平台实例缓存
    pub async fn clear_platform_cache(
        &self
    ) -> Result<(), PlatformError> {
        let mut instances = self.platform_instances.write().await;
        let cache_size = instances.len();
        instances.clear();
        info!("Cleared all platform instance cache: {} instances removed", cache_size);
        Ok(())
    }
    
    // 清除特定平台的实例缓存
    pub async fn clear_platform_cache_for_type(
        &self,
        platform_type: PlatformType
    ) -> Result<(), PlatformError> {
        let mut instances = self.platform_instances.write().await;
        let initial_count = instances.len();
        instances.retain(|(pt, _), _| pt != &platform_type);
        let final_count = instances.len();
        let removed_count = initial_count - final_count;
        info!("Cleared cache for platform {:?}: {} instances removed", platform_type, removed_count);
        Ok(())
    }
    
    // 检查平台是否已注册
    pub async fn are_platforms_registered(&self) -> bool {
        *self.platforms_registered.read().await
    }
    
    // 注册所有平台
    pub async fn register_all_platforms(&self) -> Result<(), PlatformError> {
        let mut registered = self.platforms_registered.write().await;
        if *registered {
            debug!("Platforms already registered, skipping...");
            return Ok(());
        }
        
        info!("Registering all platforms...");
        
        // Register Bilibili platform
        self.register_platform(
            PlatformType::Bilibili,
            Box::new(|config| Ok(Box::new(crate::BilibiliPlatform::new(config)?)))
        ).await?;
        info!("Registered Bilibili platform");
        
        // Register Douyin platform
        self.register_platform(
            PlatformType::Douyin,
            Box::new(|config| Ok(Box::new(crate::DouyinPlatform::new(config)?)))
        ).await?;
        info!("Registered Douyin platform");
        
        // Register Douyu platform
        self.register_platform(
            PlatformType::Douyu,
            Box::new(|config| Ok(Box::new(crate::DouyuPlatform::new(config)?)))
        ).await?;
        info!("Registered Douyu platform");
        
        // Register Huya platform
        self.register_platform(
            PlatformType::Huya,
            Box::new(|config| Ok(Box::new(crate::HuyaPlatform::new(config)?)))
        ).await?;
        info!("Registered Huya platform");
        
        *registered = true;
        info!("All platforms registered successfully");
        Ok(())
    }
}

// 平台工厂的全局实例
pub static GLOBAL_PLATFORM_FACTORY: OnceCell<Arc<PlatformFactory>> = OnceCell::new();

// 初始化全局平台工厂
pub async fn init_global_platform_factory() -> Result<(), PlatformError> {
    debug!("Initializing global platform factory...");
    let factory = get_global_platform_factory();
    info!("Global platform factory obtained");
    factory.register_all_platforms().await
}

// 获取全局平台工厂实例
pub fn get_global_platform_factory() -> Arc<PlatformFactory> {
    debug!("Getting global platform factory instance...");
    let factory = GLOBAL_PLATFORM_FACTORY.get_or_init(|| {
        info!("Creating new global platform factory instance");
        Arc::new(PlatformFactory::new())
    }).clone();
    debug!("Returning global platform factory instance");
    factory
}
