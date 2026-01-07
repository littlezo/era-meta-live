#![allow(unused_imports)]
pub mod event_broadcaster;
pub mod http_client;
pub mod interface;
pub mod logger;
pub mod macros;
pub mod types;
pub mod types_rust;

// Re-export necessary types to make them available directly under shared::TypeName
pub use event_broadcaster::EventBroadcaster;
pub use event_broadcaster::PlatformEvent;
pub use http_client::FollowHttpClient;
pub use types::BilibiliMessageState;
pub use types::MessageFrontendPayload;
pub use types::DouyinMessageState;
pub use types::DouyuMessageState;
pub use types::GetStreamUrlPayload;
pub use types::HuyaMessageState;
pub use types::LiveStreamInfo;

// Re-export interface types for convenience
pub use interface::*;

// Re-export macros for convenience
pub use macros::*;
