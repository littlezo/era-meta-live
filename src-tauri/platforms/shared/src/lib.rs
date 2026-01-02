#![allow(unused_imports)]
pub mod http_client;
pub mod types;
pub mod types_rust;

// Re-export necessary types to make them available directly under shared::TypeName
pub use http_client::FollowHttpClient;
pub use types::BilibiliMessageState;
pub use types::MessageFrontendPayload;
pub use types::DouyinMessageState;
pub use types::DouyuMessageState;
pub use types::GetStreamUrlPayload;
pub use types::HuyaMessageState;
pub use types::LiveStreamInfo;
