// Main platform module

// Public interfaces and factories
pub mod commands;
pub mod interface;
pub mod factory;
pub mod watch;
pub mod message_manager;

// Re-export shared utilities
pub use shared;



// Re-export platform implementations
pub use bilibili::BilibiliPlatform;
pub use douyin::DouyinPlatform;
pub use douyu::DouyuPlatform;
pub use huya::HuyaPlatform;

// Re-export key types and traits for easy access
pub use shared::interface::{
    LivePlatform,
    PlatformType,
    StreamQuality,
    LiveListParams,
    LiveList,
    RoomInfo,
    StreamerInfo,
    StreamUrl,
    PlatformError,
    Message,
    MessageType,
    MessageCallback,
    Category,
    PlatformConfig,
    PlatformCreator,
};

pub use factory::{
    PlatformFactory,
    init_global_platform_factory,
    get_global_platform_factory,
};

pub use watch::{
    FollowWatchState,
    WatchConfig,
    FollowStreamerInput,
    FollowStatusUpdate,
    NotificationEvent,
    EventEmitter,
    start_follow_watch_service,
    stop_follow_watch_service,
};

// register_all_platforms is now a method on PlatformFactory
// It will be called automatically when getting the global platform factory

