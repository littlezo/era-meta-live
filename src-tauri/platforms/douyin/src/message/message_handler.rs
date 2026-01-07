use flate2::read::GzDecoder;
use futures_util::{stream::SplitStream, StreamExt};
use prost::Message as ProstMessage; // For decode/encode
use std::io::Read;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;

use crate::message::gen::{PushFrame, Response};
use crate::message::message_parsers::{parse_chat_message, parse_like_message, parse_member_message, parse_room_stats_message, parse_gift_message, parse_control_message, parse_social_message, parse_update_fan_ticket_message, parse_emoji_chat_message, parse_fansclub_message, parse_room_rank_message, parse_product_change_message, parse_live_shopping_message, parse_room_stream_adaptation_message};
use crate::message::websocket_connection::WsStream;
use shared::interface::Message as PlatformMessage;
use shared::interface::MessageType;
use shared::interface::PlatformType;
use shared::logger::Logger;

// 创建静态日志记录器
static LOGGER: std::sync::OnceLock<Logger> = std::sync::OnceLock::new();

fn logger() -> &'static Logger {
    LOGGER.get_or_init(|| {
        Logger::new(Some(PlatformType::Douyin), "douyin::message::message_handler")
    })
}

// This function will handle the message receiving loop and parsing with callback
pub async fn handle_received_messages_with_callback(
    mut read_stream: SplitStream<WsStream>,
    ack_tx: Sender<WsMessage>,
    callback: impl Fn(PlatformMessage) + Send + Sync + 'static,
    room_id: String,              // Added room_id parameter
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    logger().info(format!(
        "Message handler started for room_id: {}",
        room_id
    ));
    logger().info("进入消息接收循环，等待WebSocket消息...");
    
    let mut message_count = 0;
    while let Some(message_result) = read_stream.next().await {
        message_count += 1;
        // 为每条消息生成唯一标识符
        let message_id = format!("msg-{}-{}", room_id, message_count);
        
        // 简化消息日志，只显示消息类型
        match &message_result {
            Ok(ws_msg) => {
                let msg_type = match ws_msg {
                    WsMessage::Binary(_) => "Binary",
                    WsMessage::Ping(_) => "Ping",
                    WsMessage::Pong(_) => "Pong",
                    WsMessage::Text(_) => "Text",
                    WsMessage::Close(_) => "Close",
                    WsMessage::Frame(_) => "Frame",
                };
                logger().info(format!("📥 收到消息 #{} (ID: {}): Ok({})
", message_count, message_id, msg_type));
            },
            Err(_) => {
                logger().info(format!("📥 收到消息 #{} (ID: {}): Err
", message_count, message_id));
            }
        }
        
        match message_result {
            Ok(ws_msg) => {
                let msg_type = match &ws_msg {
                    WsMessage::Binary(_) => "Binary",
                    WsMessage::Ping(_) => "Ping",
                    WsMessage::Pong(_) => "Pong",
                    WsMessage::Text(_) => "Text",
                    WsMessage::Close(_) => "Close",
                    WsMessage::Frame(_) => "Frame",
                };
                logger().info(format!("✅ 消息 #{} (ID: {}): 成功解析，类型: {}
", message_count, message_id, msg_type));
                match ws_msg {
                    WsMessage::Binary(bin_data) => {
                        logger().debug(format!("📦 二进制消息 #{} (ID: {}): 长度: {} 字节", message_count, message_id, bin_data.len()));
                        
                        // 记录消息处理开始时间
                        let start_time = std::time::Instant::now();
                        
                        match PushFrame::decode(bin_data.as_slice()) {
                            Ok(push_frame) => {
                                logger().debug(format!("🔍 PushFrame #{} (ID: {}): 类型: {}, 日志ID: {}, payload长度: {}", 
                                    message_count, message_id, push_frame.payload_type, push_frame.log_id, push_frame.payload.len()));
                                
                                if push_frame.payload_type == "msg" && !push_frame.payload.is_empty() {
                                    logger().debug(format!("📝 开始处理消息内容 #{} (ID: {})
                                        开始解压缩消息payload", message_count, message_id));
                                    let mut gz = GzDecoder::new(push_frame.payload.as_slice());
                                    let mut decompressed_payload = Vec::new();
                                    if let Err(e) = gz.read_to_end(&mut decompressed_payload) {
                                        logger().error(format!("❌ Gzip解压缩错误 #{} (ID: {}): {}", message_count, message_id, e));
                                        continue;
                                    }
                                    logger().debug(format!("🗜️ 解压缩后payload #{} (ID: {}): 长度: {} 字节", 
                                        message_count, message_id, decompressed_payload.len()));
                                    
                                    match Response::decode(decompressed_payload.as_slice()) {
                                        Ok(response) => {
                                            logger().debug(format!("📋 Response解码成功 #{} (ID: {}): 需要ACK: {}, 消息数量: {}", 
                                                message_count, message_id, response.need_ack, response.messages_list.len()));
                                            
                                            if response.need_ack {
                                                let ack_payload_bytes = response.internal_ext.encode_to_vec();
                                                let ack_push_frame = PushFrame {
                                                    log_id: push_frame.log_id,
                                                    payload_type: "ack".to_string(),
                                                    payload: ack_payload_bytes,
                                                    ..Default::default()
                                                };
                                                let mut ack_buf = Vec::new();
                                                if ack_push_frame.encode(&mut ack_buf).is_ok() {
                                                    if ack_tx.send(WsMessage::Binary(ack_buf)).await.is_err() {
                                                        logger().error(format!("❌ 发送ACK消息失败 #{} (ID: {})
                                                            日志ID: {}", message_count, message_id, push_frame.log_id));
                                                    } else {
                                                        logger().debug(format!("✅ 发送ACK成功 #{} (ID: {})
                                                            日志ID: {}", message_count, message_id, push_frame.log_id));
                                                    }
                                                } else {
                                                    logger().error(format!("❌ 编码ACK PushFrame失败 #{} (ID: {})
                                                        日志ID: {}", message_count, message_id, push_frame.log_id));
                                                }
                                            }
                                            
                                            // 遍历所有消息
                                            for (index, msg) in response.messages_list.iter().enumerate() {
                                                let sub_message_id = format!("{}-sub-{}", message_id, index + 1);
                                                logger().info(format!("📝 【消息 {}】#{} (ID: {})
                                                    类型: {}", 
                                                    index + 1, message_count, sub_message_id, msg.method));
                                                
                                                // 处理不同类型的消息
                                                match msg.method.as_str() {
                                                    "WebcastChatMessage" => {
                                                        logger().debug(format!("💬 处理聊天消息 #{} (ID: {})
                                                            开始解析聊天消息", message_count, sub_message_id));
                                                        match parse_chat_message(
                                                            &msg.payload,
                                                            &room_id,
                                                        ) {
                                                            Ok(Some(chat_payload)) => {
                                                                logger().info(format!("✅ 解析聊天消息成功 #{} (ID: {})
                                                                    {}: {}", message_count, sub_message_id, chat_payload.user, chat_payload.content));
                                                                // Convert to PlatformMessage and call callback
                                                                let raw_value = Some(chat_payload.raw.clone());
                                                                
                                                                // 尝试从原始JSON中提取更多信息
                                                                let mut uid = None;
                                                                let mut badge_name = None;
                                                                let mut badge_level = None;
                                                                let mut color = None;
                                                                
                                                                // 解析原始JSON，提取用户ID和其他信息
                                                                let raw_json = &chat_payload.raw;
                                                                    // 提取用户ID
                                                                    if let Some(user_obj) = raw_json.get("user").and_then(|u| u.as_object()) {
                                                                        if let Some(id) = user_obj.get("id").and_then(|id| id.as_u64()) {
                                                                            uid = Some(id.to_string());
                                                                        } else if let Some(short_id) = user_obj.get("short_id").and_then(|id| id.as_u64()) {
                                                                            uid = Some(short_id.to_string());
                                                                        }
                                                                        
                                                                        // 提取徽章信息
                                                                        if let Some(fans_club) = user_obj.get("fans_club").and_then(|fc| fc.as_object()) {
                                                                            if let Some(data) = fans_club.get("data").and_then(|d| d.as_object()) {
                                                                                if let Some(level) = data.get("level").and_then(|l| l.as_i64()) {
                                                                                    badge_level = Some(level as i32);
                                                                                }
                                                                                if let Some(name) = data.get("club_name").and_then(|n| n.as_str()) {
                                                                                    badge_name = Some(name.to_string());
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                    
                                                                    // 提取文本颜色
                                                                    if let Some(full_screen_color) = raw_json.get("full_screen_text_color").and_then(|c| c.as_str()) {
                                                                        if !full_screen_color.is_empty() {
                                                                            color = Some(full_screen_color.to_string());
                                                                        }
                                                                    }
                                                                
                                                                let platform_message = PlatformMessage {
                                                                    id: Some(sub_message_id.clone()),
                                                                    message_type: MessageType::Danmaku,
                                                                    user: chat_payload.user,
                                                                    content: chat_payload.content,
                                                                    user_level: Some(chat_payload.user_level as i64),
                                                                    fans_level: Some(chat_payload.fans_club_level as i32),
                                                                    fans_club_level: Some(chat_payload.fans_club_level as i32),
                                                                    badge_name,
                                                                    badge_level,
                                                                    uid,
                                                                    color,
                                                                    timestamp: tokio::time::Instant::now().elapsed().as_millis() as u64,
                                                                    room_id: chat_payload.room_id,
                                                                    platform: PlatformType::Douyin,
                                                                    gift_name: None,
                                                                    gift_count: None,
                                                                    gift_price: None,
                                                                    gift_total: None,
                                                                    super_chat_price: None,
                                                                    super_chat_duration: None,
                                                                    combo_count: None,
                                                                    combo_user: None,
                                                                    raw: raw_value,
                                                                    other: None,
                                                                };
                                                                logger().debug(format!("📤 调用回调函数广播消息 #{} (ID: {})
                                                                    消息类型: {:?}", message_count, sub_message_id, platform_message.message_type));
                                                                callback(platform_message);
                                                                logger().debug(format!("✅ 回调函数调用完成 #{} (ID: {})", message_count, sub_message_id));
                                                            }
                                                            Ok(None) => {
                                                                logger().debug(format!("⏭️ 聊天消息解析结果为None，跳过 #{} (ID: {})
                                                                    房间ID: {}", message_count, sub_message_id, room_id));
                                                            }
                                                            Err(e) => {
                                                                logger().error(format!("❌ 解析聊天消息错误 #{} (ID: {})
                                                                    错误: {}", message_count, sub_message_id, e));
                                                            }
                                                        }
                                                    },
                                                    "WebcastMemberMessage" => {
                                                        logger().debug(format!("👤 处理进场消息 #{} (ID: {})
                                                            开始解析进场消息", message_count, sub_message_id));
                                                        match parse_member_message(
                                                            &msg.payload,
                                                            &room_id,
                                                        ) {
                                                            Ok(Some(member_payload)) => {
                                                                logger().info(format!("✅ 解析进场消息成功 #{} (ID: {})
                                                                    {} 进入了直播间", message_count, sub_message_id));
                                                                let raw_value = Some(member_payload.raw.clone());
                                                                
                                                                // 从raw JSON中提取用户ID和徽章信息
                                                                let mut uid = None;
                                                                let mut badge_name = None;
                                                                let mut badge_level = None;
                                                                
                                                                let raw_json = &member_payload.raw;
                                                                    // 提取用户ID
                                                                    if let Some(user_obj) = raw_json.get("user").and_then(|u| u.as_object()) {
                                                                        if let Some(id) = user_obj.get("id").and_then(|id| id.as_u64()) {
                                                                            uid = Some(id.to_string());
                                                                        } else if let Some(short_id) = user_obj.get("short_id").and_then(|id| id.as_u64()) {
                                                                            uid = Some(short_id.to_string());
                                                                        }
                                                                        
                                                                        // 提取徽章信息
                                                                        if let Some(fans_club) = user_obj.get("fans_club").and_then(|fc| fc.as_object()) {
                                                                            if let Some(data) = fans_club.get("data").and_then(|d| d.as_object()) {
                                                                                if let Some(level) = data.get("level").and_then(|l| l.as_i64()) {
                                                                                    badge_level = Some(level as i32);
                                                                                }
                                                                                if let Some(name) = data.get("club_name").and_then(|n| n.as_str()) {
                                                                                    badge_name = Some(name.to_string());
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                
                                                                // Convert to PlatformMessage and call callback
                                                                let platform_message = PlatformMessage {
                                                                    id: Some(sub_message_id.clone()),
                                                                    message_type: MessageType::EnterRoom,
                                                                    user: member_payload.user,
                                                                    content: "进入了直播间".to_string(),
                                                                    user_level: Some(member_payload.user_level as i64),
                                                                    fans_level: Some(member_payload.fans_club_level as i32),
                                                                    fans_club_level: Some(member_payload.fans_club_level as i32),
                                                                    badge_name,
                                                                    badge_level,
                                                                    uid,
                                                                    color: None,
                                                                    timestamp: tokio::time::Instant::now().elapsed().as_millis() as u64,
                                                                    room_id: member_payload.room_id,
                                                                    platform: PlatformType::Douyin,
                                                                    gift_name: None,
                                                                    gift_count: None,
                                                                    gift_price: None,
                                                                    gift_total: None,
                                                                    super_chat_price: None,
                                                                    super_chat_duration: None,
                                                                    combo_count: None,
                                                                    combo_user: None,
                                                                    raw: raw_value,
                                                                    other: None,
                                                                };
                                                                logger().debug(format!("📤 调用回调函数广播消息 #{} (ID: {})
                                                                    消息类型: {:?}", message_count, sub_message_id, platform_message.message_type));
                                                                callback(platform_message);
                                                                logger().debug(format!("✅ 回调函数调用完成 #{} (ID: {})", message_count, sub_message_id));
                                                            }
                                                            Ok(None) => {
                                                                logger().debug(format!("⏭️ 进场消息解析结果为None，跳过 #{} (ID: {})
                                                                    房间ID: {}", message_count, sub_message_id, room_id));
                                                            }
                                                            Err(e) => {
                                                                logger().error(format!("❌ 解析进场消息错误 #{} (ID: {})
                                                                    错误: {}", message_count, sub_message_id, e));
                                                            }
                                                        }
                                                    },
                                                    "WebcastLikeMessage" => {
                                                        logger().debug(format!("👍 处理点赞消息 #{} (ID: {})
                                                            开始解析点赞消息", message_count, sub_message_id));
                                                        match parse_like_message(
                                                            &msg.payload,
                                                            &room_id,
                                                        ) {
                                                            Ok(Some(like_payload)) => {
                                                                logger().info(format!("✅ 解析点赞消息成功 #{} (ID: {})
                                                                    {} 点了赞", message_count, sub_message_id, like_payload.user));
                                                                let raw_value = Some(like_payload.raw.clone());
                                                                
                                                                // 从raw JSON中提取点赞数量和用户信息
                                                                let mut uid = None;
                                                                let mut badge_name = None;
                                                                let mut badge_level = None;
                                                                
                                                                let raw_json = &like_payload.raw;
                                                                    // 提取用户ID
                                                                    if let Some(user_obj) = raw_json.get("user").and_then(|u| u.as_object()) {
                                                                        if let Some(id) = user_obj.get("id").and_then(|id| id.as_u64()) {
                                                                            uid = Some(id.to_string());
                                                                        } else if let Some(short_id) = user_obj.get("short_id").and_then(|id| id.as_u64()) {
                                                                            uid = Some(short_id.to_string());
                                                                        }
                                                                        
                                                                        // 提取徽章信息
                                                                        if let Some(fans_club) = user_obj.get("fans_club").and_then(|fc| fc.as_object()) {
                                                                            if let Some(data) = fans_club.get("data").and_then(|d| d.as_object()) {
                                                                                if let Some(level) = data.get("level").and_then(|l| l.as_i64()) {
                                                                                    badge_level = Some(level as i32);
                                                                                }
                                                                                if let Some(name) = data.get("club_name").and_then(|n| n.as_str()) {
                                                                                    badge_name = Some(name.to_string());
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                
                                                                // Convert to PlatformMessage and call callback
                                                                let platform_message = PlatformMessage {
                                                                    id: Some(sub_message_id.clone()),
                                                                    message_type: MessageType::Like,
                                                                    user: like_payload.user,
                                                                    content: like_payload.content,
                                                                    user_level: Some(like_payload.user_level as i64),
                                                                    fans_level: Some(like_payload.fans_club_level as i32),
                                                                    fans_club_level: Some(like_payload.fans_club_level as i32),
                                                                    badge_name,
                                                                    badge_level,
                                                                    uid,
                                                                    color: None,
                                                                    timestamp: tokio::time::Instant::now().elapsed().as_millis() as u64,
                                                                    room_id: like_payload.room_id,
                                                                    platform: PlatformType::Douyin,
                                                                    gift_name: None,
                                                                    gift_count: None,
                                                                    gift_price: None,
                                                                    gift_total: None,
                                                                    super_chat_price: None,
                                                                    super_chat_duration: None,
                                                                    combo_count: None,
                                                                    combo_user: None,
                                                                    raw: raw_value,
                                                                    other: None,
                                                                };
                                                                logger().debug(format!("📤 调用回调函数广播消息 #{} (ID: {})
                                                                    消息类型: {:?}", message_count, sub_message_id, platform_message.message_type));
                                                                callback(platform_message);
                                                                logger().debug(format!("✅ 回调函数调用完成 #{} (ID: {})", message_count, sub_message_id));
                                                            }
                                                            Ok(None) => {
                                                                logger().debug(format!("⏭️ 点赞消息解析结果为None，跳过 #{} (ID: {})
                                                                    房间ID: {}", message_count, sub_message_id, room_id));
                                                            }
                                                            Err(e) => {
                                                                logger().error(format!("❌ 解析点赞消息错误 #{} (ID: {})
                                                                    错误: {}", message_count, sub_message_id, e));
                                                            }
                                                        }
                                                    },
                                                    "WebcastRoomStatsMessage" => {
                                                        logger().debug(format!("📊 处理直播间统计消息 #{} (ID: {})
                                                            开始解析直播间统计消息", message_count, sub_message_id));
                                                        match parse_room_stats_message(
                                                            &msg.payload,
                                                            &room_id,
                                                        ) {
                                                            Ok(Some(stats_payload)) => {
                                                                logger().info(format!("✅ 解析直播间统计消息成功 #{} (ID: {})
                                                                    {}", message_count, sub_message_id, stats_payload.content));
                                                                // Convert to PlatformMessage and call callback
                                                                let raw_value = Some(stats_payload.raw.clone());
                                                                let platform_message = PlatformMessage {
                                                                    id: Some(sub_message_id.clone()),
                                                                    message_type: MessageType::RoomChange,
                                                                    user: stats_payload.user,
                                                                    content: stats_payload.content,
                                                                    user_level: Some(stats_payload.user_level as i64),
                                                                    fans_level: Some(stats_payload.fans_club_level as i32),
                                                                    fans_club_level: None,
                                                                    badge_name: None,
                                                                    badge_level: None,
                                                                    uid: None,
                                                                    color: None,
                                                                    timestamp: tokio::time::Instant::now().elapsed().as_millis() as u64,
                                                                    room_id: stats_payload.room_id,
                                                                    platform: PlatformType::Douyin,
                                                                    gift_name: None,
                                                                    gift_count: None,
                                                                    gift_price: None,
                                                                    gift_total: None,
                                                                    super_chat_price: None,
                                                                    super_chat_duration: None,
                                                                    combo_count: None,
                                                                    combo_user: None,
                                                                    raw: raw_value,
                                                                    other: None,
                                                                };
                                                                logger().debug(format!("📤 调用回调函数广播消息 #{} (ID: {})
                                                                    消息类型: {:?}", message_count, sub_message_id, platform_message.message_type));
                                                                callback(platform_message);
                                                                logger().debug(format!("✅ 回调函数调用完成 #{} (ID: {})", message_count, sub_message_id));
                                                            }
                                                            Ok(None) => {
                                                                logger().debug(format!("⏭️ 直播间统计消息解析结果为None，跳过 #{} (ID: {})
                                                                    房间ID: {}", message_count, sub_message_id, room_id));
                                                            }
                                                            Err(e) => {
                                                                logger().error(format!("❌ 解析直播间统计消息错误 #{} (ID: {})
                                                                    错误: {}", message_count, sub_message_id, e));
                                                            }
                                                        }
                                                    },
                                                    "WebcastGiftMessage" => {
                                                        logger().debug(format!("🎁 处理礼物消息 #{} (ID: {})
                                                            开始解析礼物消息", message_count, sub_message_id));
                                                        match parse_gift_message(
                                                            &msg.payload,
                                                            &room_id,
                                                        ) {
                                                            Ok(Some(gift_payload)) => {
                                                                logger().info(format!("✅ 解析礼物消息成功 #{} (ID: {})
                                                                    {}", message_count, sub_message_id, gift_payload.content));
                                                                let raw_value = Some(gift_payload.raw.clone());
                                                                
                                                                // 从raw JSON中提取礼物详细信息
                                                                let mut gift_name = None;
                                                                let mut gift_count = None;
                                                                let mut gift_price = None;
                                                                let mut combo_count = None;
                                                                let mut uid = None;
                                                                let mut badge_name = None;
                                                                let mut badge_level = None;
                                                                
                                                                let raw_json = &gift_payload.raw;
                                                                    // 提取礼物信息
                                                                    if let Some(gift_obj) = raw_json.get("gift").and_then(|g| g.as_object()) {
                                                                        if let Some(name) = gift_obj.get("name").and_then(|n| n.as_str()) {
                                                                            gift_name = Some(name.to_string());
                                                                        }
                                                                        if let Some(diamond_count) = gift_obj.get("diamond_count").and_then(|dc| dc.as_u64()) {
                                                                            gift_price = Some(diamond_count as f64);
                                                                        }
                                                                    }
                                                                    
                                                                    // 提取礼物数量
                                                                    if let Some(count) = raw_json.get("total_count").and_then(|c| c.as_u64()) {
                                                                        gift_count = Some(count as u32);
                                                                    } else if let Some(repeat_count) = raw_json.get("repeat_count").and_then(|rc| rc.as_u64()) {
                                                                        gift_count = Some((repeat_count + 1) as u32);
                                                                    }
                                                                    
                                                                    // 提取连击数
                                                                    if let Some(combo) = raw_json.get("combo_count").and_then(|cc| cc.as_u64()) {
                                                                        combo_count = Some(combo as u32);
                                                                    }
                                                                    
                                                                    // 提取用户信息
                                                                    if let Some(user_obj) = raw_json.get("user").and_then(|u| u.as_object()) {
                                                                        if let Some(id) = user_obj.get("id").and_then(|id| id.as_u64()) {
                                                                            uid = Some(id.to_string());
                                                                        } else if let Some(short_id) = user_obj.get("short_id").and_then(|id| id.as_u64()) {
                                                                            uid = Some(short_id.to_string());
                                                                        }
                                                                        
                                                                        // 提取徽章信息
                                                                        if let Some(fans_club) = user_obj.get("fans_club").and_then(|fc| fc.as_object()) {
                                                                            if let Some(data) = fans_club.get("data").and_then(|d| d.as_object()) {
                                                                                if let Some(level) = data.get("level").and_then(|l| l.as_i64()) {
                                                                                    badge_level = Some(level as i32);
                                                                                }
                                                                                if let Some(name) = data.get("club_name").and_then(|n| n.as_str()) {
                                                                                    badge_name = Some(name.to_string());
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                
                                                                // 如果没有从raw中提取到礼物名称，使用旧方法
                                                                let final_gift_name = gift_name.or_else(|| {
                                                                    gift_payload.content.split_once(" ").map(|(name, _)| name.to_string())
                                                                });
                                                                
                                                                // 计算礼物总价
                                                                let gift_total = if let (Some(price), Some(count)) = (gift_price, gift_count) {
                                                                    Some(price * count as f64)
                                                                } else {
                                                                    None
                                                                };
                                                                
                                                                // Convert to PlatformMessage and call callback
                                                                let platform_message = PlatformMessage {
                                                                    id: Some(sub_message_id.clone()),
                                                                    message_type: MessageType::Gift,
                                                                    user: gift_payload.user,
                                                                    content: gift_payload.content,
                                                                    user_level: Some(gift_payload.user_level as i64),
                                                                    fans_level: Some(gift_payload.fans_club_level as i32),
                                                                    fans_club_level: Some(gift_payload.fans_club_level as i32),
                                                                    badge_name,
                                                                    badge_level,
                                                                    uid,
                                                                    color: None,
                                                                    timestamp: tokio::time::Instant::now().elapsed().as_millis() as u64,
                                                                    room_id: gift_payload.room_id,
                                                                    platform: PlatformType::Douyin,
                                                                    gift_name: final_gift_name,
                                                                    gift_count,
                                                                    gift_price,
                                                                    gift_total,
                                                                    super_chat_price: None,
                                                                    super_chat_duration: None,
                                                                    combo_count,
                                                                    combo_user: None,
                                                                    raw: raw_value,
                                                                    other: None,
                                                                };
                                                                logger().debug(format!("📤 调用回调函数广播消息 #{} (ID: {})
                                                                    消息类型: {:?}", message_count, sub_message_id, platform_message.message_type));
                                                                callback(platform_message);
                                                                logger().debug(format!("✅ 回调函数调用完成 #{} (ID: {})", message_count, sub_message_id));
                                                            }
                                                            Ok(None) => {
                                                                logger().debug(format!("⏭️ 礼物消息解析结果为None，跳过 #{} (ID: {})
                                                                    房间ID: {}", message_count, sub_message_id, room_id));
                                                            }
                                                            Err(e) => {
                                                                logger().error(format!("❌ 解析礼物消息错误 #{} (ID: {})
                                                                    错误: {}", message_count, sub_message_id, e));
                                                            }
                                                        }
                                                    },
                                                    "WebcastControlMessage" => {
                                                        logger().debug(format!("🎮 处理控制消息 #{} (ID: {})
                                                            开始解析控制消息", message_count, sub_message_id));
                                                        match parse_control_message(
                                                            &msg.payload,
                                                            &room_id,
                                                        ) {
                                                            Ok(Some(control_payload)) => {
                                                                logger().info(format!("✅ 解析控制消息成功 #{} (ID: {})
                                                                    {}", message_count, sub_message_id, control_payload.content));
                                                                // Convert to PlatformMessage and call callback
                                                                let raw_value = Some(control_payload.raw.clone());
                                                                let platform_message = PlatformMessage {
                                                                    id: Some(sub_message_id.clone()),
                                                                    message_type: MessageType::System,
                                                                    user: control_payload.user,
                                                                    content: control_payload.content,
                                                                    user_level: Some(control_payload.user_level as i64),
                                                                    fans_level: Some(control_payload.fans_club_level as i32),
                                                                    fans_club_level: None,
                                                                    badge_name: None,
                                                                    badge_level: None,
                                                                    uid: None,
                                                                    color: None,
                                                                    timestamp: tokio::time::Instant::now().elapsed().as_millis() as u64,
                                                                    room_id: control_payload.room_id,
                                                                    platform: PlatformType::Douyin,
                                                                    gift_name: None,
                                                                    gift_count: None,
                                                                    gift_price: None,
                                                                    gift_total: None,
                                                                    super_chat_price: None,
                                                                    super_chat_duration: None,
                                                                    combo_count: None,
                                                                    combo_user: None,
                                                                    raw: raw_value,
                                                                    other: None,
                                                                };
                                                                logger().debug(format!("📤 调用回调函数广播消息 #{} (ID: {})
                                                                    消息类型: {:?}", message_count, sub_message_id, platform_message.message_type));
                                                                callback(platform_message);
                                                                logger().debug(format!("✅ 回调函数调用完成 #{} (ID: {})", message_count, sub_message_id));
                                                            }
                                                            Ok(None) => {
                                                                logger().debug(format!("⏭️ 控制消息解析结果为None，跳过 #{} (ID: {})
                                                                    房间ID: {}", message_count, sub_message_id, room_id));
                                                            }
                                                            Err(e) => {
                                                                logger().error(format!("❌ 解析控制消息错误 #{} (ID: {})
                                                                    错误: {}", message_count, sub_message_id, e));
                                                            }
                                                        }
                                                    },
                                                    "WebcastSocialMessage" => {
                                                        logger().debug(format!("👥 处理社交消息 #{} (ID: {})
                                                            开始解析社交消息", message_count, sub_message_id));
                                                        match parse_social_message(
                                                            &msg.payload,
                                                            &room_id,
                                                        ) {
                                                            Ok(Some(social_payload)) => {
                                                                logger().info(format!("✅ 解析社交消息成功 #{} (ID: {})
                                                                    {}", message_count, sub_message_id, social_payload.content));
                                                                // Convert to PlatformMessage and call callback
                                                                let raw_value = Some(social_payload.raw.clone());
                                                                let platform_message = PlatformMessage {
                                                                    id: Some(sub_message_id.clone()),
                                                                    message_type: MessageType::Other("social".to_string()),
                                                                    user: social_payload.user,
                                                                    content: social_payload.content,
                                                                    user_level: Some(social_payload.user_level as i64),
                                                                    fans_level: Some(social_payload.fans_club_level as i32),
                                                                    fans_club_level: None,
                                                                    badge_name: None,
                                                                    badge_level: None,
                                                                    uid: None,
                                                                    color: None,
                                                                    timestamp: tokio::time::Instant::now().elapsed().as_millis() as u64,
                                                                    room_id: social_payload.room_id,
                                                                    platform: PlatformType::Douyin,
                                                                    gift_name: None,
                                                                    gift_count: None,
                                                                    gift_price: None,
                                                                    gift_total: None,
                                                                    super_chat_price: None,
                                                                    super_chat_duration: None,
                                                                    combo_count: None,
                                                                    combo_user: None,
                                                                    raw: raw_value,
                                                                    other: None,
                                                                };
                                                                logger().debug(format!("📤 调用回调函数广播消息 #{} (ID: {})
                                                                    消息类型: {:?}", message_count, sub_message_id, platform_message.message_type));
                                                                callback(platform_message);
                                                                logger().debug(format!("✅ 回调函数调用完成 #{} (ID: {})", message_count, sub_message_id));
                                                            }
                                                            Ok(None) => {
                                                                logger().debug(format!("⏭️ 社交消息解析结果为None，跳过 #{} (ID: {})
                                                                    房间ID: {}", message_count, sub_message_id, room_id));
                                                            }
                                                            Err(e) => {
                                                                logger().error(format!("❌ 解析社交消息错误 #{} (ID: {})
                                                                    错误: {}", message_count, sub_message_id, e));
                                                            }
                                                        }
                                                    },
                                                    "WebcastUpdateFanTicketMessage" => {
                                                        logger().debug(format!("🎫 处理粉丝票更新消息 #{} (ID: {})
                                                            开始解析粉丝票更新消息", message_count, sub_message_id));
                                                        match parse_update_fan_ticket_message(
                                                            &msg.payload,
                                                            &room_id,
                                                        ) {
                                                            Ok(Some(ticket_payload)) => {
                                                                logger().info(format!("✅ 解析粉丝票更新消息成功 #{} (ID: {})
                                                                    {}", message_count, sub_message_id, ticket_payload.content));
                                                                // Convert to PlatformMessage and call callback
                                                                let raw_value = Some(ticket_payload.raw.clone());
                                                                let platform_message = PlatformMessage {
                                                                    id: Some(sub_message_id.clone()),
                                                                    message_type: MessageType::Other("fan_ticket_update".to_string()),
                                                                    user: ticket_payload.user,
                                                                    content: ticket_payload.content,
                                                                    user_level: Some(ticket_payload.user_level as i64),
                                                                    fans_level: Some(ticket_payload.fans_club_level as i32),
                                                                    fans_club_level: None,
                                                                    badge_name: None,
                                                                    badge_level: None,
                                                                    uid: None,
                                                                    color: None,
                                                                    timestamp: tokio::time::Instant::now().elapsed().as_millis() as u64,
                                                                    room_id: ticket_payload.room_id,
                                                                    platform: PlatformType::Douyin,
                                                                    gift_name: None,
                                                                    gift_count: None,
                                                                    gift_price: None,
                                                                    gift_total: None,
                                                                    super_chat_price: None,
                                                                    super_chat_duration: None,
                                                                    combo_count: None,
                                                                    combo_user: None,
                                                                    raw: raw_value,
                                                                    other: None,
                                                                };
                                                                logger().debug(format!("📤 调用回调函数广播消息 #{} (ID: {})
                                                                    消息类型: {:?}", message_count, sub_message_id, platform_message.message_type));
                                                                callback(platform_message);
                                                                logger().debug(format!("✅ 回调函数调用完成 #{} (ID: {})", message_count, sub_message_id));
                                                            }
                                                            Ok(None) => {
                                                                logger().debug(format!("⏭️ 粉丝票更新消息解析结果为None，跳过 #{} (ID: {})
                                                                    房间ID: {}", message_count, sub_message_id, room_id));
                                                            }
                                                            Err(e) => {
                                                                logger().error(format!("❌ 解析粉丝票更新消息错误 #{} (ID: {})
                                                                    错误: {}", message_count, sub_message_id, e));
                                                            }
                                                        }
                                                    },
                                                    "WebcastEmojiChatMessage" => {
                                                        logger().debug(format!("😊 处理表情聊天消息 #{} (ID: {})
                                                            开始解析表情聊天消息", message_count, sub_message_id));
                                                        match parse_emoji_chat_message(
                                                            &msg.payload,
                                                            &room_id,
                                                        ) {
                                                            Ok(Some(emoji_payload)) => {
                                                                logger().info(format!("✅ 解析表情聊天消息成功 #{} (ID: {})
                                                                    {}: {}", message_count, sub_message_id, emoji_payload.user, emoji_payload.content));
                                                                let raw_value = Some(emoji_payload.raw.clone());
                                                                
                                                                // 从raw JSON中提取用户信息和徽章信息
                                                                let mut uid = None;
                                                                let mut badge_name = None;
                                                                let mut badge_level = None;
                                                                
                                                                let raw_json = &emoji_payload.raw;
                                                                    // 提取用户ID
                                                                    if let Some(user_obj) = raw_json.get("user").and_then(|u| u.as_object()) {
                                                                        if let Some(id) = user_obj.get("id").and_then(|id| id.as_u64()) {
                                                                            uid = Some(id.to_string());
                                                                        } else if let Some(short_id) = user_obj.get("short_id").and_then(|id| id.as_u64()) {
                                                                            uid = Some(short_id.to_string());
                                                                        }
                                                                        
                                                                        // 提取徽章信息
                                                                        if let Some(fans_club) = user_obj.get("fans_club").and_then(|fc| fc.as_object()) {
                                                                            if let Some(data) = fans_club.get("data").and_then(|d| d.as_object()) {
                                                                                if let Some(level) = data.get("level").and_then(|l| l.as_i64()) {
                                                                                    badge_level = Some(level as i32);
                                                                                }
                                                                                if let Some(name) = data.get("club_name").and_then(|n| n.as_str()) {
                                                                                    badge_name = Some(name.to_string());
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                
                                                                // Convert to PlatformMessage and call callback
                                                                let platform_message = PlatformMessage {
                                                                    id: Some(sub_message_id.clone()),
                                                                    message_type: MessageType::Danmaku,
                                                                    user: emoji_payload.user,
                                                                    content: emoji_payload.content,
                                                                    user_level: Some(emoji_payload.user_level as i64),
                                                                    fans_level: Some(emoji_payload.fans_club_level as i32),
                                                                    fans_club_level: Some(emoji_payload.fans_club_level as i32),
                                                                    badge_name,
                                                                    badge_level,
                                                                    uid,
                                                                    color: None,
                                                                    timestamp: tokio::time::Instant::now().elapsed().as_millis() as u64,
                                                                    room_id: emoji_payload.room_id,
                                                                    platform: PlatformType::Douyin,
                                                                    gift_name: None,
                                                                    gift_count: None,
                                                                    gift_price: None,
                                                                    gift_total: None,
                                                                    super_chat_price: None,
                                                                    super_chat_duration: None,
                                                                    combo_count: None,
                                                                    combo_user: None,
                                                                    raw: raw_value,
                                                                    other: None,
                                                                };
                                                                logger().debug(format!("📤 调用回调函数广播消息 #{} (ID: {})
                                                                    消息类型: {:?}", message_count, sub_message_id, platform_message.message_type));
                                                                callback(platform_message);
                                                                logger().debug(format!("✅ 回调函数调用完成 #{} (ID: {})", message_count, sub_message_id));
                                                            }
                                                            Ok(None) => {
                                                                logger().debug(format!("⏭️ 表情聊天消息解析结果为None，跳过 #{} (ID: {})
                                                                    房间ID: {}", message_count, sub_message_id, room_id));
                                                            }
                                                            Err(e) => {
                                                                logger().error(format!("❌ 解析表情聊天消息错误 #{} (ID: {})
                                                                    错误: {}", message_count, sub_message_id, e));
                                                            }
                                                        }
                                                    },
                                                    "WebcastFansclubMessage" => {
                                                        logger().debug(format!("🏠 处理粉丝团消息 #{} (ID: {})
                                                            开始解析粉丝团消息", message_count, sub_message_id));
                                                        match parse_fansclub_message(
                                                            &msg.payload,
                                                            &room_id,
                                                        ) {
                                                            Ok(Some(fansclub_payload)) => {
                                                                logger().info(format!("✅ 解析粉丝团消息成功 #{} (ID: {})
                                                                    {}", message_count, sub_message_id, fansclub_payload.content));
                                                                // Convert to PlatformMessage and call callback
                                                                let raw_value = Some(fansclub_payload.raw.clone());
                                                                let platform_message = PlatformMessage {
                                                                    id: Some(sub_message_id.clone()),
                                                                    message_type: MessageType::Other("fansclub".to_string()),
                                                                    user: fansclub_payload.user,
                                                                    content: fansclub_payload.content,
                                                                    user_level: Some(fansclub_payload.user_level as i64),
                                                                    fans_level: Some(fansclub_payload.fans_club_level as i32),
                                                                    fans_club_level: None,
                                                                    badge_name: None,
                                                                    badge_level: None,
                                                                    uid: None,
                                                                    color: None,
                                                                    timestamp: tokio::time::Instant::now().elapsed().as_millis() as u64,
                                                                    room_id: fansclub_payload.room_id,
                                                                    platform: PlatformType::Douyin,
                                                                    gift_name: None,
                                                                    gift_count: None,
                                                                    gift_price: None,
                                                                    gift_total: None,
                                                                    super_chat_price: None,
                                                                    super_chat_duration: None,
                                                                    combo_count: None,
                                                                    combo_user: None,
                                                                    raw: raw_value,
                                                                    other: None,
                                                                };
                                                                logger().debug(format!("📤 调用回调函数广播消息 #{} (ID: {})
                                                                    消息类型: {:?}", message_count, sub_message_id, platform_message.message_type));
                                                                callback(platform_message);
                                                                logger().debug(format!("✅ 回调函数调用完成 #{} (ID: {})", message_count, sub_message_id));
                                                            }
                                                            Ok(None) => {
                                                                logger().debug(format!("⏭️ 粉丝团消息解析结果为None，跳过 #{} (ID: {})
                                                                    房间ID: {}", message_count, sub_message_id, room_id));
                                                            }
                                                            Err(e) => {
                                                                logger().error(format!("❌ 解析粉丝团消息错误 #{} (ID: {})
                                                                    错误: {}", message_count, sub_message_id, e));
                                                            }
                                                        }
                                                    },
                                                    "WebcastRoomRankMessage" => {
                                                        logger().debug(format!("🏆 处理直播间排行榜消息 #{} (ID: {})
                                                            开始解析直播间排行榜消息", message_count, sub_message_id));
                                                        match parse_room_rank_message(
                                                            &msg.payload,
                                                            &room_id,
                                                        ) {
                                                            Ok(Some(rank_payload)) => {
                                                                logger().info(format!("✅ 解析直播间排行榜消息成功 #{} (ID: {})
                                                                    {}", message_count, sub_message_id, rank_payload.content));
                                                                // Convert to PlatformMessage and call callback
                                                                let raw_value = Some(rank_payload.raw.clone());
                                                                let platform_message = PlatformMessage {
                                                                    id: Some(sub_message_id.clone()),
                                                                    message_type: MessageType::Other("room_rank".to_string()),
                                                                    user: rank_payload.user,
                                                                    content: rank_payload.content,
                                                                    user_level: Some(rank_payload.user_level as i64),
                                                                    fans_level: Some(rank_payload.fans_club_level as i32),
                                                                    fans_club_level: None,
                                                                    badge_name: None,
                                                                    badge_level: None,
                                                                    uid: None,
                                                                    color: None,
                                                                    timestamp: tokio::time::Instant::now().elapsed().as_millis() as u64,
                                                                    room_id: rank_payload.room_id,
                                                                    platform: PlatformType::Douyin,
                                                                    gift_name: None,
                                                                    gift_count: None,
                                                                    gift_price: None,
                                                                    gift_total: None,
                                                                    super_chat_price: None,
                                                                    super_chat_duration: None,
                                                                    combo_count: None,
                                                                    combo_user: None,
                                                                    raw: raw_value,
                                                                    other: None,
                                                                };
                                                                logger().debug(format!("📤 调用回调函数广播消息 #{} (ID: {})
                                                                    消息类型: {:?}", message_count, sub_message_id, platform_message.message_type));
                                                                callback(platform_message);
                                                                logger().debug(format!("✅ 回调函数调用完成 #{} (ID: {})", message_count, sub_message_id));
                                                            }
                                                            Ok(None) => {
                                                                logger().debug(format!("⏭️ 直播间排行榜消息解析结果为None，跳过 #{} (ID: {})
                                                                    房间ID: {}", message_count, sub_message_id, room_id));
                                                            }
                                                            Err(e) => {
                                                                logger().error(format!("❌ 解析直播间排行榜消息错误 #{} (ID: {})
                                                                    错误: {}", message_count, sub_message_id, e));
                                                            }
                                                        }
                                                    },
                                                    "WebcastProductChangeMessage" => {
                                                        logger().debug(format!("🛒 处理商品变更消息 #{} (ID: {})
                                                            开始解析商品变更消息", message_count, sub_message_id));
                                                        match parse_product_change_message(
                                                            &msg.payload,
                                                            &room_id,
                                                        ) {
                                                            Ok(Some(product_payload)) => {
                                                                logger().info(format!("✅ 解析商品变更消息成功 #{} (ID: {})
                                                                    {}", message_count, sub_message_id, product_payload.content));
                                                                // Convert to PlatformMessage and call callback
                                                                let raw_value = Some(product_payload.raw.clone());
                                                                let platform_message = PlatformMessage {
                                                                    id: Some(sub_message_id.clone()),
                                                                    message_type: MessageType::Other("product_change".to_string()),
                                                                    user: product_payload.user,
                                                                    content: product_payload.content,
                                                                    user_level: Some(product_payload.user_level as i64),
                                                                    fans_level: Some(product_payload.fans_club_level as i32),
                                                                    fans_club_level: None,
                                                                    badge_name: None,
                                                                    badge_level: None,
                                                                    uid: None,
                                                                    color: None,
                                                                    timestamp: tokio::time::Instant::now().elapsed().as_millis() as u64,
                                                                    room_id: product_payload.room_id,
                                                                    platform: PlatformType::Douyin,
                                                                    gift_name: None,
                                                                    gift_count: None,
                                                                    gift_price: None,
                                                                    gift_total: None,
                                                                    super_chat_price: None,
                                                                    super_chat_duration: None,
                                                                    combo_count: None,
                                                                    combo_user: None,
                                                                    raw: raw_value,
                                                                    other: None,
                                                                };
                                                                logger().debug(format!("📤 调用回调函数广播消息 #{} (ID: {})
                                                                    消息类型: {:?}", message_count, sub_message_id, platform_message.message_type));
                                                                callback(platform_message);
                                                                logger().debug(format!("✅ 回调函数调用完成 #{} (ID: {})", message_count, sub_message_id));
                                                            }
                                                            Ok(None) => {
                                                                logger().debug(format!("⏭️ 商品变更消息解析结果为None，跳过 #{} (ID: {})
                                                                    房间ID: {}", message_count, sub_message_id, room_id));
                                                            }
                                                            Err(e) => {
                                                                logger().error(format!("❌ 解析商品变更消息错误 #{} (ID: {})
                                                                    错误: {}", message_count, sub_message_id, e));
                                                            }
                                                        }
                                                    },
                                                    "WebcastLiveShoppingMessage" => {
                                                        logger().debug(format!("🛍️ 处理直播购物消息 #{} (ID: {})
                                                            开始解析直播购物消息", message_count, sub_message_id));
                                                        match parse_live_shopping_message(
                                                            &msg.payload,
                                                            &room_id,
                                                        ) {
                                                            Ok(Some(shopping_payload)) => {
                                                                logger().info(format!("✅ 解析直播购物消息成功 #{} (ID: {})
                                                                    {}", message_count, sub_message_id, shopping_payload.content));
                                                                // Convert to PlatformMessage and call callback
                                                                let raw_value = Some(shopping_payload.raw.clone());
                                                                let platform_message = PlatformMessage {
                                                                    id: Some(sub_message_id.clone()),
                                                                    message_type: MessageType::Other("live_shopping".to_string()),
                                                                    user: shopping_payload.user,
                                                                    content: shopping_payload.content,
                                                                    user_level: Some(shopping_payload.user_level as i64),
                                                                    fans_level: Some(shopping_payload.fans_club_level as i32),
                                                                    fans_club_level: None,
                                                                    badge_name: None,
                                                                    badge_level: None,
                                                                    uid: None,
                                                                    color: None,
                                                                    timestamp: tokio::time::Instant::now().elapsed().as_millis() as u64,
                                                                    room_id: shopping_payload.room_id,
                                                                    platform: PlatformType::Douyin,
                                                                    gift_name: None,
                                                                    gift_count: None,
                                                                    gift_price: None,
                                                                    gift_total: None,
                                                                    super_chat_price: None,
                                                                    super_chat_duration: None,
                                                                    combo_count: None,
                                                                    combo_user: None,
                                                                    raw: raw_value,
                                                                    other: None,
                                                                };
                                                                logger().debug(format!("📤 调用回调函数广播消息 #{} (ID: {})
                                                                    消息类型: {:?}", message_count, sub_message_id, platform_message.message_type));
                                                                callback(platform_message);
                                                                logger().debug(format!("✅ 回调函数调用完成 #{} (ID: {})", message_count, sub_message_id));
                                                            }
                                                            Ok(None) => {
                                                                logger().debug(format!("⏭️ 直播购物消息解析结果为None，跳过 #{} (ID: {})
                                                                    房间ID: {}", message_count, sub_message_id, room_id));
                                                            }
                                                            Err(e) => {
                                                                logger().error(format!("❌ 解析直播购物消息错误 #{} (ID: {})
                                                                    错误: {}", message_count, sub_message_id, e));
                                                            }
                                                        }
                                                    },
                                                    "WebcastRoomStreamAdaptationMessage" => {
                                                        logger().debug(format!("📡 处理直播间流适配消息 #{} (ID: {})
                                                            开始解析直播间流适配消息", message_count, sub_message_id));
                                                        match parse_room_stream_adaptation_message(
                                                            &msg.payload,
                                                            &room_id,
                                                        ) {
                                                            Ok(Some(adaptation_payload)) => {
                                                                logger().info(format!("✅ 解析直播间流适配消息成功 #{} (ID: {})
                                                                    {}", message_count, sub_message_id, adaptation_payload.content));
                                                                // Convert to PlatformMessage and call callback
                                                                let raw_value = Some(adaptation_payload.raw.clone());
                                                                let platform_message = PlatformMessage {
                                                                    id: Some(sub_message_id.clone()),
                                                                    message_type: MessageType::Other("room_stream_adaptation".to_string()),
                                                                    user: adaptation_payload.user,
                                                                    content: adaptation_payload.content,
                                                                    user_level: Some(adaptation_payload.user_level as i64),
                                                                    fans_level: Some(adaptation_payload.fans_club_level as i32),
                                                                    fans_club_level: None,
                                                                    badge_name: None,
                                                                    badge_level: None,
                                                                    uid: None,
                                                                    color: None,
                                                                    timestamp: tokio::time::Instant::now().elapsed().as_millis() as u64,
                                                                    room_id: adaptation_payload.room_id,
                                                                    platform: PlatformType::Douyin,
                                                                    gift_name: None,
                                                                    gift_count: None,
                                                                    gift_price: None,
                                                                    gift_total: None,
                                                                    super_chat_price: None,
                                                                    super_chat_duration: None,
                                                                    combo_count: None,
                                                                    combo_user: None,
                                                                    raw: raw_value,
                                                                    other: None,
                                                                };
                                                                logger().debug(format!("📤 调用回调函数广播消息 #{} (ID: {})
                                                                    消息类型: {:?}", message_count, sub_message_id, platform_message.message_type));
                                                                callback(platform_message);
                                                                logger().debug(format!("✅ 回调函数调用完成 #{} (ID: {})", message_count, sub_message_id));
                                                            }
                                                            Ok(None) => {
                                                                logger().debug(format!("⏭️ 直播间流适配消息解析结果为None，跳过 #{} (ID: {})
                                                                    房间ID: {}", message_count, sub_message_id, room_id));
                                                            }
                                                            Err(e) => {
                                                                logger().error(format!("❌ 解析直播间流适配消息错误 #{} (ID: {})
                                                                    错误: {}", message_count, sub_message_id, e));
                                                            }
                                                        }
                                                    },
                                                    _ => {
                                                        logger().info(format!("🔄 处理其他类型消息 #{} (ID: {})
                                                            类型: {}, 房间ID: {}", message_count, sub_message_id, msg.method, room_id));
                                                    }
                                                }
                                            }
                                            
                                            // 记录消息处理时间
                                            let elapsed = start_time.elapsed();
                                            logger().debug(format!("⏱️ 消息 #{} (ID: {}) 处理完成，耗时: {:?}", message_count, message_id, elapsed));
                                        }
                                        Err(e) => {
                                            logger().error(format!("❌ 解析Response错误 #{} (ID: {})
                                                错误: {}", message_count, message_id, e));
                                        }
                                    }
                                } else if push_frame.payload_type == "ack" {
                                    logger().debug(format!("📩 收到服务器ACK #{} (ID: {})
                                        日志ID: {}", message_count, message_id, push_frame.log_id));
                                } else if push_frame.payload_type == "hb" {
                                    logger().debug(format!("💓 收到服务器心跳消息 #{} (ID: {})
                                        日志ID: {}", message_count, message_id, push_frame.log_id));
                                } else {
                                    logger().debug(format!("📦 收到其他类型PushFrame #{} (ID: {})
                                        类型: {}", message_count, message_id, push_frame.payload_type));
                                }
                            }
                            Err(e) => {
                                logger().error(format!("❌ 解析PushFrame错误 #{} (ID: {})
                                    错误: {}", message_count, message_id, e));
                            }
                        }
                    },
                    WsMessage::Ping(ping_data) => {
                        logger().debug(format!("🔔 收到Ping消息 #{} (ID: {})
                            数据长度: {} 字节", message_count, message_id, ping_data.len()));
                        if ack_tx.send(WsMessage::Pong(ping_data)).await.is_err() {
                            logger().error(format!("❌ 发送Pong消息失败 #{} (ID: {})
                                房间ID: {}", message_count, message_id, room_id));
                        } else {
                            logger().debug(format!("✅ 发送Pong消息成功 #{} (ID: {})
                                房间ID: {}", message_count, message_id, room_id));
                        }
                    },
                    WsMessage::Pong(_) => {
                        logger().debug(format!("🏓 收到Pong消息 #{} (ID: {})
                            房间ID: {}", message_count, message_id, room_id));
                    },
                    WsMessage::Text(text) => {
                        logger().debug(format!("📝 收到文本消息 #{} (ID: {})
                            内容: {}", message_count, message_id, text));
                    },
                    WsMessage::Close(close_frame) => {
                        logger().info(format!("🔚 服务器关闭WebSocket连接 #{} (ID: {})
                            关闭帧: {:?}, 房间ID: {}", message_count, message_id, close_frame, room_id));
                        break;
                    },
                    WsMessage::Frame(frame) => {
                        logger().info(format!("📦 收到原始帧 #{} (ID: {})
                            帧: {:?}, 房间ID: {}", message_count, message_id, frame, room_id));
                    },
                }
            }
            Err(e) => {
                logger().error(format!(
                    "❌ WebSocket接收错误 #{}: {}", message_count, e
                ));
                logger().info(format!("🔄 由于接收错误，退出消息接收循环，房间ID: {}", room_id));
                break;
            }
        }
    }
    logger().info(format!("📤 消息接收循环结束，共处理 {} 条消息，房间ID: {}", message_count, room_id));
    logger().info(format!("Message handler finished for room_id: {}", room_id));
    Ok(())
}


