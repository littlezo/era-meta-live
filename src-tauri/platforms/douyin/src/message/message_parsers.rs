use super::gen::{ChatMessage, LikeMessage, MemberMessage, RoomStatsMessage, GiftMessage, ControlMessage, SocialMessage, UpdateFanTicketMessage, RoomUserSeqMessage, EmojiChatMessage, FansclubMessage, RoomRankMessage, ProductChangeMessage, LiveShoppingMessage, RoomStreamAdaptationMessage}; // Updated to directly use types from gen
use shared::MessageFrontendPayload;
use prost::Message as ProstMessage; // For .decode() // Use shared payload type

use shared::interface::PlatformType;
use shared::logger::Logger;

// 创建静态日志记录器
static LOGGER: std::sync::OnceLock<Logger> = std::sync::OnceLock::new();

fn logger() -> &'static Logger {
    LOGGER.get_or_init(|| {
        Logger::new(Some(PlatformType::Douyin), "douyin::message::message_parsers")
    })
}

// Parser for ChatMessage
pub fn parse_chat_message(
    bytes: &[u8],
    room_id: &str,
) -> Result<Option<MessageFrontendPayload>, Box<dyn std::error::Error + Send + Sync>> {
    match ChatMessage::decode(bytes) {
        Ok(chat_msg) => {
            // Serialize the complete ChatMessage to JSON value directly
            let raw_json = serde_json::to_value(&chat_msg)?;
            
            if let Some(user) = chat_msg.user {
                // 获取用户等级
                let user_level = user.pay_grade.as_ref().map(|pg| pg.level).unwrap_or(0);
                // 获取粉丝牌等级
                let fans_club_level = user
                    .fans_club
                    .as_ref()
                    .and_then(|fc| fc.data.as_ref())
                    .map(|fcd| fcd.level)
                    .unwrap_or(0);

                Ok(Some(MessageFrontendPayload {
                    room_id: room_id.to_string(),
                    user: user.nick_name.clone(),
                    content: chat_msg.content.clone(),
                    user_level,
                    fans_club_level,
                    raw: raw_json,
                }))
            } else {
                // 对于没有用户信息的聊天消息 (例如系统消息)
                logger().debug(format!(
                    "    【聊天msg】Content: {} (no user info)",
                    chat_msg.content
                ));
                Ok(Some(MessageFrontendPayload {
                    room_id: room_id.to_string(),
                    user: "系统".to_string(),
                    content: chat_msg.content.clone(),
                    user_level: 0,
                    fans_club_level: 0,
                    raw: raw_json,
                }))
            }
        }
        Err(e) => {
            Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }
    }
}

// Demo 中此函数返回 Result<(), ...> 并且只打印，这里保持原有返回 Option<MessageFrontendPayload> 结构
// 如果不需要将进场消息发送到前端，可以保持返回 Ok(None)
#[allow(dead_code)] // ADDED to suppress warning
pub fn parse_member_message(
    payload: &[u8],
    room_id: &str,
) -> Result<Option<MessageFrontendPayload>, Box<dyn std::error::Error + Send + Sync>> {
    match MemberMessage::decode(payload) {
        Ok(member_msg) => {
            // Serialize the complete MemberMessage to JSON value directly
            let raw_json = serde_json::to_value(&member_msg)?;
            
            if let Some(user) = member_msg.user {
                let user_level = user.pay_grade.as_ref().map(|pg| pg.level).unwrap_or(0);
                let fans_club_level = user
                    .fans_club
                    .as_ref()
                    .and_then(|fc| fc.data.as_ref())
                    .map(|fcd| fcd.level)
                    .unwrap_or(0);

                logger().debug(format!(
                    "    【进场msg】[用户等级: {}][粉丝牌等级: {}]{} 进入了直播间",
                    user_level, fans_club_level, user.nick_name
                ));
                Ok(Some(MessageFrontendPayload {
                    room_id: room_id.to_string(),
                    user: user.nick_name.clone(),
                    content: "进入了直播间".to_string(),
                    user_level,
                    fans_club_level,
                    raw: raw_json,
                }))
            } else {
                logger().debug("    【进场msg】MemberMessage without user details.");
                Ok(None)
            }
        }
        Err(e) => {
            logger().error(format!("    【X】Failed to parse MemberMessage in parser: {}", e));
            Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }
    }
}

// Parser for LikeMessage (点赞消息)
// Demo 中此函数返回 Result<(), ...> 并且只打印
#[allow(dead_code)] // ADDED to suppress warning
pub fn parse_like_message(
    payload: &[u8],
    room_id: &str,
) -> Result<Option<MessageFrontendPayload>, Box<dyn std::error::Error + Send + Sync>> {
    match LikeMessage::decode(payload) {
        Ok(like_msg) => {
            // Serialize the complete LikeMessage to JSON value directly
            let raw_json = serde_json::to_value(&like_msg)?;
            
            if let Some(user) = like_msg.user {
                let user_level = user.pay_grade.as_ref().map(|pg| pg.level).unwrap_or(0);
                let fans_club_level = user
                    .fans_club
                    .as_ref()
                    .and_then(|fc| fc.data.as_ref())
                    .map(|fcd| fcd.level)
                    .unwrap_or(0);
                
                logger().debug(format!(
                    "    【点赞msg】{} 点了{}个赞",
                    user.nick_name, like_msg.count
                ));
                Ok(Some(MessageFrontendPayload {
                    room_id: room_id.to_string(),
                    user: user.nick_name.clone(),
                    content: format!("点了{}个赞", like_msg.count),
                    user_level,
                    fans_club_level,
                    raw: raw_json,
                }))
            } else {
                logger().debug(format!("    【点赞msg】点赞 {} 个 (无用户信息)", like_msg.count));
                Ok(Some(MessageFrontendPayload {
                    room_id: room_id.to_string(),
                    user: "未知用户".to_string(),
                    content: format!("点了{}个赞", like_msg.count),
                    user_level: 0,
                    fans_club_level: 0,
                    raw: raw_json,
                }))
            }
        }
        Err(e) => {
            logger().error(format!("    【X】Failed to parse LikeMessage in parser: {}", e));
            Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }
    }
}

#[allow(dead_code)] // ADDED to suppress warning
pub fn parse_room_stats_message(
    payload: &[u8],
    _current_room_id: &str,
) -> Result<Option<MessageFrontendPayload>, Box<dyn std::error::Error + Send + Sync>> {
    match RoomStatsMessage::decode(payload) {
        Ok(stats_msg) => {
            let raw_json = serde_json::to_value(&stats_msg)?;
            logger().debug(format!("    【直播间统计msg】{} [{:?}]", stats_msg.display_long,raw_json));
            Ok(None) // 统计信息通常不作为普通弹幕显示
        }
        Err(e) => {
            logger().error(format!("    【X】Failed to parse RoomStatsMessage in parser: {}", e));
            Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }
    }
}

// Parser for EmojiChatMessage
#[allow(dead_code)] // ADDED to suppress warning
pub fn parse_emoji_chat_message(
    payload: &[u8],
    room_id: &str,
) -> Result<Option<MessageFrontendPayload>, Box<dyn std::error::Error + Send + Sync>> {
    match EmojiChatMessage::decode(payload) {
        Ok(emoji_msg) => {
            // Serialize the complete EmojiChatMessage to JSON value directly
            let raw_json = serde_json::to_value(&emoji_msg)?;
            
            if let Some(user) = emoji_msg.user {
                let user_level = user.pay_grade.as_ref().map(|pg| pg.level).unwrap_or(0);
                let fans_club_level = user
                    .fans_club
                    .as_ref()
                    .and_then(|fc| fc.data.as_ref())
                    .map(|fcd| fcd.level)
                    .unwrap_or(0);

                logger().debug(format!(
                    "    【表情聊天msg】[用户等级: {}][粉丝牌等级: {}]{} 发送了表情消息",
                    user_level, fans_club_level, user.nick_name
                ));
                Ok(Some(MessageFrontendPayload {
                    room_id: room_id.to_string(),
                    user: user.nick_name.clone(),
                    content: emoji_msg.default_content.clone(),
                    user_level,
                    fans_club_level,
                    raw: raw_json,
                }))
            } else {
                logger().debug("    【表情聊天msg】EmojiChatMessage without user details.");
                Ok(None)
            }
        }
        Err(e) => {
            logger().error(format!("    【X】Failed to parse EmojiChatMessage in parser: {}", e));
            Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }
    }
}

// Parser for GiftMessage
#[allow(dead_code)] // ADDED to suppress warning
pub fn parse_gift_message(
    payload: &[u8],
    room_id: &str,
) -> Result<Option<MessageFrontendPayload>, Box<dyn std::error::Error + Send + Sync>> {
    match GiftMessage::decode(payload) {
        Ok(gift_msg) => {
            // Serialize the complete GiftMessage to JSON value directly
            let raw_json = serde_json::to_value(&gift_msg)?;

            if let Some(user) = gift_msg.user {
                let user_level = user.pay_grade.as_ref().map(|pg| pg.level).unwrap_or(0);
                let fans_club_level = user
                    .fans_club
                    .as_ref()
                    .and_then(|fc| fc.data.as_ref())
                    .map(|fcd| fcd.level)
                    .unwrap_or(0);

                let gift_name = gift_msg.gift.as_ref().map(|g| g.name.clone()).unwrap_or_default();
                let gift_count = gift_msg.repeat_count + 1; // 重复次数+1就是实际赠送数量

                logger().debug(format!(
                    "    【礼物msg】[用户等级: {}][粉丝牌等级: {}]{} 赠送了 {} x{}",
                    user_level, fans_club_level, user.nick_name, gift_name, gift_count
                ));
                Ok(Some(MessageFrontendPayload {
                    room_id: room_id.to_string(),
                    user: user.nick_name.clone(),
                    content: format!("赠送了 {} x{}", gift_name, gift_count),
                    user_level,
                    fans_club_level,
                    raw: raw_json,
                }))
            } else {
                logger().debug("    【礼物msg】GiftMessage without user details.");
                Ok(None)
            }
        }
        Err(e) => {
            logger().error(format!("    【X】Failed to parse GiftMessage in parser: {}", e));
            Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }
    }
}

// Parser for ControlMessage
#[allow(dead_code)] // ADDED to suppress warning
pub fn parse_control_message(
    payload: &[u8],
    room_id: &str,
) -> Result<Option<MessageFrontendPayload>, Box<dyn std::error::Error + Send + Sync>> {
    match ControlMessage::decode(payload) {
        Ok(control_msg) => {
            // Serialize the complete ControlMessage to JSON value directly
            let raw_json = serde_json::to_value(&control_msg)?;
            
            let status = match control_msg.status {
                3 => "下播".to_string(),
                _ => format!("未知状态: {}", control_msg.status),
            };
            logger().debug(format!("    【控制msg】直播间 {}: {}", room_id, status));
            Ok(Some(MessageFrontendPayload {
                room_id: room_id.to_string(),
                user: "系统".to_string(),
                content: format!("直播间已{}", status),
                user_level: 0,
                fans_club_level: 0,
                raw: raw_json,
            }))
        }
        Err(e) => {
            logger().error(format!("    【X】Failed to parse ControlMessage in parser: {}", e));
            Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }
    }
}

// Parser for SocialMessage
#[allow(dead_code)] // ADDED to suppress warning
pub fn parse_social_message(
    payload: &[u8],
    room_id: &str,
) -> Result<Option<MessageFrontendPayload>, Box<dyn std::error::Error + Send + Sync>> {
    match SocialMessage::decode(payload) {
        Ok(social_msg) => {
            // Serialize the complete SocialMessage to JSON value directly
            let raw_json = serde_json::to_value(&social_msg)?;
            
            if let Some(user) = social_msg.user {
                let action = match social_msg.action {
                    1 => "关注了主播".to_string(),
                    _ => format!("未知社交行为: {}", social_msg.action),
                };
                
                let user_level = user.pay_grade.as_ref().map(|pg| pg.level).unwrap_or(0);
                let fans_club_level = user
                    .fans_club
                    .as_ref()
                    .and_then(|fc| fc.data.as_ref())
                    .map(|fcd| fcd.level)
                    .unwrap_or(0);
                
                logger().debug(format!(
                    "    【社交msg】[用户等级: {}]{} {}",
                    user_level, user.nick_name, action
                ));
                Ok(Some(MessageFrontendPayload {
                    room_id: room_id.to_string(),
                    user: user.nick_name.clone(),
                    content: action,
                    user_level,
                    fans_club_level,
                    raw: raw_json,
                }))
            } else {
                logger().debug("    【社交msg】SocialMessage without user details.");
                Ok(None)
            }
        }
        Err(e) => {
            logger().error(format!("    【X】Failed to parse SocialMessage in parser: {}", e));
            Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }
    }
}

// Parser for UpdateFanTicketMessage
#[allow(dead_code)] // ADDED to suppress warning
pub fn parse_update_fan_ticket_message(
    payload: &[u8],
    room_id: &str,
) -> Result<Option<MessageFrontendPayload>, Box<dyn std::error::Error + Send + Sync>> {
    match UpdateFanTicketMessage::decode(payload) {
        Ok(ticket_msg) => {
            // Serialize the complete UpdateFanTicketMessage to JSON value directly
            let raw_json = serde_json::to_value(&ticket_msg)?;
            
            logger().debug(format!(
                "    【粉丝票更新msg】直播间 {}: {}",
                room_id,
                ticket_msg.room_fan_ticket_count_text
            ));
            Ok(Some(MessageFrontendPayload {
                room_id: room_id.to_string(),
                user: "系统".to_string(),
                content: ticket_msg.room_fan_ticket_count_text.clone(),
                user_level: 0,
                fans_club_level: 0,
                raw: raw_json,
            }))
        }
        Err(e) => {
            logger().error(format!("    【X】Failed to parse UpdateFanTicketMessage in parser: {}", e));
            Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }
    }
}

// Parser for RoomUserSeqMessage
#[allow(dead_code)] // ADDED to suppress warning
pub fn parse_room_user_seq_message(
    payload: &[u8],
    _current_room_id: &str,
) -> Result<Option<MessageFrontendPayload>, Box<dyn std::error::Error + Send + Sync>> {
    match RoomUserSeqMessage::decode(payload) {
        Ok(seq_msg) => {
            let raw_json = serde_json::to_value(&seq_msg)?;

            logger().debug(format!(
                "    【直播间用户序列msg】总榜用户数: {}, 座位数: {}, [{:?}]",
                seq_msg.total,
                seq_msg.seats_list.len(),
                raw_json
            ));
            Ok(None) // 通常不需要广播到前端
        }
        Err(e) => {
            logger().error(format!("    【X】Failed to parse RoomUserSeqMessage in parser: {}", e));
            Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }
    }
}

// Parser for FansclubMessage
#[allow(dead_code)] // ADDED to suppress warning
pub fn parse_fansclub_message(
    payload: &[u8],
    room_id: &str,
) -> Result<Option<MessageFrontendPayload>, Box<dyn std::error::Error + Send + Sync>> {
    match FansclubMessage::decode(payload) {
        Ok(fansclub_msg) => {
            // Serialize the complete FansclubMessage to JSON value directly
            let raw_json = serde_json::to_value(&fansclub_msg)?;
            
            if let Some(user) = fansclub_msg.user {
                let action = match fansclub_msg.r#type {
                    1 => "升级了粉丝团等级".to_string(),
                    2 => "加入了粉丝团".to_string(),
                    _ => format!("粉丝团行为: {}", fansclub_msg.r#type),
                };
                
                let user_level = user.pay_grade.as_ref().map(|pg| pg.level).unwrap_or(0);
                let fans_club_level = user
                    .fans_club
                    .as_ref()
                    .and_then(|fc| fc.data.as_ref())
                    .map(|fcd| fcd.level)
                    .unwrap_or(0);
                
                logger().debug(format!(
                    "    【粉丝团msg】[用户等级: {}]{} {}",
                    user_level, user.nick_name, action
                ));
                Ok(Some(MessageFrontendPayload {
                    room_id: room_id.to_string(),
                    user: user.nick_name.clone(),
                    content: fansclub_msg.content.clone(),
                    user_level,
                    fans_club_level,
                    raw: raw_json,
                }))
            } else {
                logger().debug("    【粉丝团msg】FansclubMessage without user details.");
                Ok(None)
            }
        }
        Err(e) => {
            logger().error(format!("    【X】Failed to parse FansclubMessage in parser: {}", e));
            Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }
    }
}

// Parser for RoomRankMessage
#[allow(dead_code)] // ADDED to suppress warning
pub fn parse_room_rank_message(
    payload: &[u8],
    _current_room_id: &str,
) -> Result<Option<MessageFrontendPayload>, Box<dyn std::error::Error + Send + Sync>> {
    match RoomRankMessage::decode(payload) {
        Ok(rank_msg) => {
            let raw_json = serde_json::to_value(&rank_msg)?;
            logger().debug(format!("    【直播间排行榜msg】上榜用户数: {}, [{:?}]", rank_msg.ranks_list.len(),raw_json));
            Ok(None) // 通常不需要广播到前端
        }
        Err(e) => {
            logger().error(format!("    【X】Failed to parse RoomRankMessage in parser: {}", e));
            Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }
    }
}

// Parser for ProductChangeMessage
#[allow(dead_code)] // ADDED to suppress warning
pub fn parse_product_change_message(
    payload: &[u8],
    room_id: &str,
) -> Result<Option<MessageFrontendPayload>, Box<dyn std::error::Error + Send + Sync>> {
    match ProductChangeMessage::decode(payload) {
        Ok(product_msg) => {
            // Serialize the complete ProductChangeMessage to JSON value directly
            let raw_json = serde_json::to_value(&product_msg)?;
            
            logger().debug(format!(
                "    【商品变更msg】直播间 {}: 更新了 {} 个商品",
                room_id,
                product_msg.update_product_info_list.len()
            ));
            Ok(Some(MessageFrontendPayload {
                room_id: room_id.to_string(),
                user: "系统".to_string(),
                content: product_msg.update_toast.clone(),
                user_level: 0,
                fans_club_level: 0,
                raw: raw_json,
            }))
        }
        Err(e) => {
            logger().error(format!("    【X】Failed to parse ProductChangeMessage in parser: {}", e));
            Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }
    }
}

// Parser for LiveShoppingMessage
#[allow(dead_code)] // ADDED to suppress warning
pub fn parse_live_shopping_message(
    payload: &[u8],
    room_id: &str,
) -> Result<Option<MessageFrontendPayload>, Box<dyn std::error::Error + Send + Sync>> {
    match LiveShoppingMessage::decode(payload) {
        Ok(shopping_msg) => {
            let raw_json = serde_json::to_value(&shopping_msg)?;
            logger().debug(format!("    【直播购物msg】直播间 {}: 推广ID: {}, [{:?}]", room_id, shopping_msg.promotion_id,raw_json));
            Ok(None) // 通常不需要广播到前端
        }
        Err(e) => {
            logger().error(format!("    【X】Failed to parse LiveShoppingMessage in parser: {}", e));
            Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }
    }
}

// Parser for RoomStreamAdaptationMessage
#[allow(dead_code)] // ADDED to suppress warning
pub fn parse_room_stream_adaptation_message(
    payload: &[u8],
    _current_room_id: &str,
) -> Result<Option<MessageFrontendPayload>, Box<dyn std::error::Error + Send + Sync>> {
    match RoomStreamAdaptationMessage::decode(payload) {
        Ok(adaptation_msg) => {
            let raw_json = serde_json::to_value(&adaptation_msg)?;
            logger().debug(format!("    【直播间流适配msg】类型: {}, [{:?}]", adaptation_msg.adaptation_type,raw_json));
            Ok(None) // 通常不需要广播到前端
        }
        Err(e) => {
            logger().error(format!("    【X】Failed to parse RoomStreamAdaptationMessage in parser: {}", e));
            Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }
    }
}
