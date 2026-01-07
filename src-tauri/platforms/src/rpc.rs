use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tonic::{async_trait, Request, Response, Status};

use shared::interface::{Message, PlatformType, RoomInfo, StreamQuality, StreamerInfo};
use shared::logger::Logger;
use shared::platform::LivePlatform;
use shared::rpc::{LivePlatformRpcService, LivePlatformRpcServiceServer};

use crate::platform_factory::{PlatformFactory, get_global_platform_factory};

// 消息监听器管理器
#[derive(Default)]
pub struct ListenerManager {
    listeners: RwLock<HashMap<String, tokio::sync::broadcast::Sender<Message>>>,
}

impl ListenerManager {
    pub fn new() -> Self {
        Default::default()
    }

    pub async fn add_listener(&self, listener_id: String, tx: tokio::sync::broadcast::Sender<Message>) {
        let mut listeners = self.listeners.write().await;
        listeners.insert(listener_id, tx);
    }

    pub async fn remove_listener(&self, listener_id: &str) {
        let mut listeners = self.listeners.write().await;
        listeners.remove(listener_id);
    }

    pub async fn get_listener(&self, listener_id: &str) -> Option<tokio::sync::broadcast::Sender<Message>> {
        let listeners = self.listeners.read().await;
        listeners.get(listener_id).cloned()
    }
}

// RPC服务实现
pub struct LivePlatformRpcServiceImpl {
    factory: Arc<PlatformFactory>,
    listener_manager: Arc<ListenerManager>,
    logger: Logger,
}

impl LivePlatformRpcServiceImpl {
    pub fn new(factory: Arc<PlatformFactory>) -> Self {
        Self {
            factory,
            listener_manager: Arc::new(ListenerManager::new()),
            logger: Logger::new(None, "grpc_service"),
        }
    }
}

#[async_trait]
impl LivePlatformRpcService for LivePlatformRpcServiceImpl {
    async fn get_room_info(
        &self, 
        request: Request<shared::rpc::GetRoomInfoRequest>
    ) -> Result<Response<shared::rpc::GetRoomInfoResponse>, Status> {
        let req = request.into_inner();
        let platform = PlatformType::from_i32(req.platform).ok_or(Status::invalid_argument("Invalid platform"))?;
        let room_id = req.room_id;

        self.logger.info(format!("get_room_info: platform={}, room_id={}", platform, room_id));

        let platform_config = crate::platform_factory::PlatformConfig::default();
        match self.factory.get_platform(platform, platform_config).await {
            Ok(platform_instance) => {
                match platform_instance.fetch_room_info(&room_id).await {
                    Ok(room_info) => {
                        let room_info_proto = shared::rpc::RoomInfo {
                            room_id: room_info.room_id,
                            title: room_info.title,
                            streamer_id: room_info.streamer_id,
                            streamer_name: room_info.streamer_name,
                            live_status: room_info.live_status,
                            cover_url: room_info.cover_url,
                            viewer_count: room_info.viewer_count as u32,
                            viewer_count_str: room_info.viewer_count_str,
                            live_status_detail: room_info.live_status_detail,
                            avatar_url: room_info.avatar_url.unwrap_or_default(),
                            category_name: room_info.category_name.unwrap_or_default(),
                        };

                        Ok(Response::new(shared::rpc::GetRoomInfoResponse {
                            success: true,
                            room_info: Some(room_info_proto),
                            error: None,
                        }))
                    }
                    Err(e) => {
                        self.logger.error(format!("Failed to fetch room info: {:?}", e));
                        Ok(Response::new(shared::rpc::GetRoomInfoResponse {
                            success: false,
                            room_info: None,
                            error: Some(e.to_string()),
                        }))
                    }
                }
            }
            Err(e) => {
                self.logger.error(format!("Failed to get platform instance: {:?}", e));
                Ok(Response::new(shared::rpc::GetRoomInfoResponse {
                    success: false,
                    room_info: None,
                    error: Some(e.to_string()),
                }))
            }
        }
    }

    async fn get_stream_url(
        &self, 
        request: Request<shared::rpc::GetStreamUrlRequest>
    ) -> Result<Response<shared::rpc::GetStreamUrlResponse>, Status> {
        let req = request.into_inner();
        let platform = PlatformType::from_i32(req.platform).ok_or(Status::invalid_argument("Invalid platform"))?;
        let room_id = req.room_id;
        let quality = StreamQuality::from_i32(req.quality).ok_or(Status::invalid_argument("Invalid quality"))?;

        self.logger.info(format!("get_stream_url: platform={}, room_id={}, quality={:?}", platform, room_id, quality));

        let platform_config = crate::platform_factory::PlatformConfig::default();
        match self.factory.get_platform(platform, platform_config).await {
            Ok(platform_instance) => {
                match platform_instance.fetch_stream_url(&room_id, quality).await {
                    Ok(stream_url) => {
                        Ok(Response::new(shared::rpc::GetStreamUrlResponse {
                            success: true,
                            stream_url: Some(stream_url),
                            error: None,
                        }))
                    }
                    Err(e) => {
                        self.logger.error(format!("Failed to fetch stream url: {:?}", e));
                        Ok(Response::new(shared::rpc::GetStreamUrlResponse {
                            success: false,
                            stream_url: None,
                            error: Some(e.to_string()),
                        }))
                    }
                }
            }
            Err(e) => {
                self.logger.error(format!("Failed to get platform instance: {:?}", e));
                Ok(Response::new(shared::rpc::GetStreamUrlResponse {
                    success: false,
                    stream_url: None,
                    error: Some(e.to_string()),
                }))
            }
        }
    }

    async fn get_live_list(
        &self, 
        request: Request<shared::rpc::GetLiveListRequest>
    ) -> Result<Response<shared::rpc::GetLiveListResponse>, Status> {
        let req = request.into_inner();
        let platform = PlatformType::from_i32(req.platform).ok_or(Status::invalid_argument("Invalid platform"))?;
        let category_id = req.category_id;
        let page = req.page;
        let page_size = req.page_size;

        self.logger.info(format!("get_live_list: platform={}, category_id={:?}, page={}, page_size={}", platform, category_id, page, page_size));

        let platform_config = crate::platform_factory::PlatformConfig::default();
        match self.factory.get_platform(platform, platform_config).await {
            Ok(platform_instance) => {
                match platform_instance.fetch_live_list(category_id.as_deref(), page, page_size).await {
                    Ok((rooms, has_more)) => {
                        let rooms_proto = rooms.into_iter().map(|room_info| shared::rpc::RoomInfo {
                            room_id: room_info.room_id,
                            title: room_info.title,
                            streamer_id: room_info.streamer_id,
                            streamer_name: room_info.streamer_name,
                            live_status: room_info.live_status,
                            cover_url: room_info.cover_url,
                            viewer_count: room_info.viewer_count as u32,
                            viewer_count_str: room_info.viewer_count_str,
                            live_status_detail: room_info.live_status_detail,
                            avatar_url: room_info.avatar_url.unwrap_or_default(),
                            category_name: room_info.category_name.unwrap_or_default(),
                        }).collect();

                        Ok(Response::new(shared::rpc::GetLiveListResponse {
                            success: true,
                            rooms: rooms_proto,
                            has_more,
                            error: None,
                        }))
                    }
                    Err(e) => {
                        self.logger.error(format!("Failed to fetch live list: {:?}", e));
                        Ok(Response::new(shared::rpc::GetLiveListResponse {
                            success: false,
                            rooms: vec![],
                            has_more: false,
                            error: Some(e.to_string()),
                        }))
                    }
                }
            }
            Err(e) => {
                self.logger.error(format!("Failed to get platform instance: {:?}", e));
                Ok(Response::new(shared::rpc::GetLiveListResponse {
                    success: false,
                    rooms: vec![],
                    has_more: false,
                    error: Some(e.to_string()),
                }))
            }
        }
    }

    async fn get_categories(
        &self, 
        request: Request<shared::rpc::GetCategoriesRequest>
    ) -> Result<Response<shared::rpc::GetCategoriesResponse>, Status> {
        let req = request.into_inner();
        let platform = PlatformType::from_i32(req.platform).ok_or(Status::invalid_argument("Invalid platform"))?;

        self.logger.info(format!("get_categories: platform={}", platform));

        let platform_config = crate::platform_factory::PlatformConfig::default();
        match self.factory.get_platform(platform, platform_config).await {
            Ok(platform_instance) => {
                match platform_instance.fetch_categories().await {
                    Ok(categories) => {
                        let categories_bytes = categories.into_iter()
                            .filter_map(|cat| serde_json::to_vec(&cat).ok())
                            .collect();

                        Ok(Response::new(shared::rpc::GetCategoriesResponse {
                            success: true,
                            categories: categories_bytes,
                            error: None,
                        }))
                    }
                    Err(e) => {
                        self.logger.error(format!("Failed to fetch categories: {:?}", e));
                        Ok(Response::new(shared::rpc::GetCategoriesResponse {
                            success: false,
                            categories: vec![],
                            error: Some(e.to_string()),
                        }))
                    }
                }
            }
            Err(e) => {
                self.logger.error(format!("Failed to get platform instance: {:?}", e));
                Ok(Response::new(shared::rpc::GetCategoriesResponse {
                    success: false,
                    categories: vec![],
                    error: Some(e.to_string()),
                }))
            }
        }
    }

    async fn get_streamer_info(
        &self, 
        request: Request<shared::rpc::GetStreamerInfoRequest>
    ) -> Result<Response<shared::rpc::GetStreamerInfoResponse>, Status> {
        let req = request.into_inner();
        let platform = PlatformType::from_i32(req.platform).ok_or(Status::invalid_argument("Invalid platform"))?;
        let streamer_id = req.streamer_id;

        self.logger.info(format!("get_streamer_info: platform={}, streamer_id={}", platform, streamer_id));

        let platform_config = crate::platform_factory::PlatformConfig::default();
        match self.factory.get_platform(platform, platform_config).await {
            Ok(platform_instance) => {
                match platform_instance.fetch_streamer_info(&streamer_id).await {
                    Ok(streamer_info) => {
                        let streamer_info_proto = shared::rpc::StreamerInfo {
                            id: streamer_info.id,
                            name: streamer_info.name,
                            avatar_url: streamer_info.avatar_url,
                            live_status: streamer_info.live_status,
                            room_id: streamer_info.room_id,
                            room_title: streamer_info.room_title,
                            live_status_detail: streamer_info.live_status_detail,
                        };

                        Ok(Response::new(shared::rpc::GetStreamerInfoResponse {
                            success: true,
                            streamer_info: Some(streamer_info_proto),
                            error: None,
                        }))
                    }
                    Err(e) => {
                        self.logger.error(format!("Failed to fetch streamer info: {:?}", e));
                        Ok(Response::new(shared::rpc::GetStreamerInfoResponse {
                            success: false,
                            streamer_info: None,
                            error: Some(e.to_string()),
                        }))
                    }
                }
            }
            Err(e) => {
                self.logger.error(format!("Failed to get platform instance: {:?}", e));
                Ok(Response::new(shared::rpc::GetStreamerInfoResponse {
                    success: false,
                    streamer_info: None,
                    error: Some(e.to_string()),
                }))
            }
        }
    }

    async fn search_streamers(
        &self, 
        request: Request<shared::rpc::SearchStreamersRequest>
    ) -> Result<Response<shared::rpc::SearchStreamersResponse>, Status> {
        let req = request.into_inner();
        let platform = PlatformType::from_i32(req.platform).ok_or(Status::invalid_argument("Invalid platform"))?;
        let keyword = req.keyword;
        let page = req.page;
        let page_size = req.page_size;

        self.logger.info(format!("search_streamers: platform={}, keyword={}, page={}, page_size={}", platform, keyword, page, page_size));

        let platform_config = crate::platform_factory::PlatformConfig::default();
        match self.factory.get_platform(platform, platform_config).await {
            Ok(platform_instance) => {
                match platform_instance.search_streamers(&keyword, page, page_size).await {
                    Ok((streamers, has_more)) => {
                        let streamers_proto = streamers.into_iter().map(|streamer_info| shared::rpc::StreamerInfo {
                            id: streamer_info.id,
                            name: streamer_info.name,
                            avatar_url: streamer_info.avatar_url,
                            live_status: streamer_info.live_status,
                            room_id: streamer_info.room_id,
                            room_title: streamer_info.room_title,
                            live_status_detail: streamer_info.live_status_detail,
                        }).collect();

                        Ok(Response::new(shared::rpc::SearchStreamersResponse {
                            success: true,
                            streamers: streamers_proto,
                            has_more,
                            error: None,
                        }))
                    }
                    Err(e) => {
                        self.logger.error(format!("Failed to search streamers: {:?}", e));
                        Ok(Response::new(shared::rpc::SearchStreamersResponse {
                            success: false,
                            streamers: vec![],
                            has_more: false,
                            error: Some(e.to_string()),
                        }))
                    }
                }
            }
            Err(e) => {
                self.logger.error(format!("Failed to get platform instance: {:?}", e));
                Ok(Response::new(shared::rpc::SearchStreamersResponse {
                    success: false,
                    streamers: vec![],
                    has_more: false,
                    error: Some(e.to_string()),
                }))
            }
        }
    }

    async fn start_message_listener(
        &self, 
        request: Request<shared::rpc::StartMessageListenerRequest>
    ) -> Result<Response<shared::rpc::StartMessageListenerResponse>, Status> {
        let req = request.into_inner();
        let platform = PlatformType::from_i32(req.platform).ok_or(Status::invalid_argument("Invalid platform"))?;
        let room_id = req.room_id;

        self.logger.info(format!("start_message_listener: platform={}, room_id={}", platform, room_id));

        // 生成唯一的listener_id
        let listener_id = format!("{}-{}-{}", platform.to_string(), room_id, uuid::Uuid::new_v4());

        let platform_config = crate::platform_factory::PlatformConfig::default();
        match self.factory.get_platform(platform, platform_config).await {
            Ok(platform_instance) => {
                let (tx, _rx) = tokio::sync::broadcast::channel(100);
                self.listener_manager.add_listener(listener_id.clone(), tx.clone()).await;

                // 启动消息监听器
                let result = platform_instance.start_message_listener(&room_id, move |msg| {
                    // 发送消息到广播通道
                    let _ = tx.send(msg);
                }).await;

                match result {
                    Ok(_) => {
                        Ok(Response::new(shared::rpc::StartMessageListenerResponse {
                            success: true,
                            listener_id: Some(listener_id),
                            error: None,
                        }))
                    }
                    Err(e) => {
                        self.logger.error(format!("Failed to start message listener: {:?}", e));
                        self.listener_manager.remove_listener(&listener_id).await;
                        Ok(Response::new(shared::rpc::StartMessageListenerResponse {
                            success: false,
                            listener_id: None,
                            error: Some(e.to_string()),
                        }))
                    }
                }
            }
            Err(e) => {
                self.logger.error(format!("Failed to get platform instance: {:?}", e));
                Ok(Response::new(shared::rpc::StartMessageListenerResponse {
                    success: false,
                    listener_id: None,
                    error: Some(e.to_string()),
                }))
            }
        }
    }

    async fn stop_message_listener(
        &self, 
        request: Request<shared::rpc::StopMessageListenerRequest>
    ) -> Result<Response<shared::rpc::StopMessageListenerResponse>, Status> {
        let req = request.into_inner();
        let listener_id = req.listener_id;

        self.logger.info(format!("stop_message_listener: listener_id={}", listener_id));

        // 移除监听器
        self.listener_manager.remove_listener(&listener_id).await;

        Ok(Response::new(shared::rpc::StopMessageListenerResponse {
            success: true,
            error: None,
        }))
    }

    type SubscribeMessagesStream = tokio::sync::mpsc::Receiver<Result<shared::rpc::MessageNotification, Status>>;

    async fn subscribe_messages(
        &self, 
        request: Request<shared::rpc::StartMessageListenerRequest>
    ) -> Result<Response<Self::SubscribeMessagesStream>, Status> {
        // 实现消息订阅逻辑
        let req = request.into_inner();
        let platform = PlatformType::from_i32(req.platform).ok_or(Status::invalid_argument("Invalid platform"))?;
        let room_id = req.room_id;

        self.logger.info(format!("subscribe_messages: platform={}, room_id={}", platform, room_id));

        // 生成唯一的listener_id
        let listener_id = format!("{}-{}-{}", platform.to_string(), room_id, uuid::Uuid::new_v4());

        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        let (broadcast_tx, mut broadcast_rx) = tokio::sync::broadcast::channel(100);

        self.listener_manager.add_listener(listener_id.clone(), broadcast_tx.clone()).await;

        let platform_config = crate::platform_factory::PlatformConfig::default();
        match self.factory.get_platform(platform, platform_config).await {
            Ok(platform_instance) => {
                // 启动消息监听器
                let result = platform_instance.start_message_listener(&room_id, move |msg| {
                    // 发送消息到广播通道
                    let _ = broadcast_tx.send(msg);
                }).await;

                match result {
                    Ok(_) => {
                        // 启动消息转发任务
                        tokio::spawn(async move {
                            while let Ok(msg) = broadcast_rx.recv().await {
                                // 转换为proto消息
                                let msg_proto = shared::rpc::Message {
                                    id: msg.id,
                                    message_type: msg.message_type.into(),
                                    user: msg.user,
                                    content: msg.content,
                                    user_level: msg.user_level,
                                    fans_level: msg.fans_level,
                                    fans_club_level: msg.fans_club_level,
                                    badge_name: msg.badge_name,
                                    badge_level: msg.badge_level,
                                    uid: msg.uid,
                                    color: msg.color,
                                    timestamp: msg.timestamp,
                                    room_id: msg.room_id,
                                    platform: msg.platform.into(),
                                    gift_name: msg.gift_name,
                                    gift_count: msg.gift_count,
                                    gift_price: msg.gift_price,
                                    gift_total: msg.gift_total,
                                    super_chat_price: msg.super_chat_price,
                                    super_chat_duration: msg.super_chat_duration,
                                    combo_count: msg.combo_count,
                                    combo_user: msg.combo_user,
                                    raw: msg.raw,
                                };

                                let notification = shared::rpc::MessageNotification {
                                    listener_id: listener_id.clone(),
                                    message: msg_proto,
                                };

                                if tx.send(Ok(notification)).await.is_err() {
                                    // 客户端断开连接
                                    break;
                                }
                            }
                        });

                        Ok(Response::new(rx))
                    }
                    Err(e) => {
                        self.logger.error(format!("Failed to start message listener: {:?}", e));
                        self.listener_manager.remove_listener(&listener_id).await;
                        Err(Status::internal(e.to_string()))
                    }
                }
            }
            Err(e) => {
                self.logger.error(format!("Failed to get platform instance: {:?}", e));
                Err(Status::internal(e.to_string()))
            }
        }
    }
}

// 启动gRPC服务
pub async fn start_grpc_server(addr: &str) -> Result<(), anyhow::Error> {
    let logger = Logger::new(None, "grpc_server");
    logger.info(format!("Starting gRPC server on {}", addr));

    let factory = get_global_platform_factory();
    let service = LivePlatformRpcServiceImpl::new(factory);
    let server = LivePlatformRpcServiceServer::new(service);

    let addr = addr.parse()?;
    tonic::transport::Server::builder()
        .add_service(server)
        .serve(addr)
        .await?;

    Ok(())
}
