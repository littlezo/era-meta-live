use chrono::Utc;
use futures_util::{stream::SplitStream, SinkExt, StreamExt};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{self, Sender};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;
use tokio_tungstenite::{connect_async, MaybeTlsStream};
use urlencoding;
// use url::Url; // REMOVED AGAIN
// use rand::Rng; // REMOVED AGAIN

use shared::interface::PlatformType;
use shared::logger::Logger;

// 创建静态日志记录器
static LOGGER: std::sync::OnceLock<Logger> = std::sync::OnceLock::new();

fn logger() -> &'static Logger {
    LOGGER.get_or_init(|| {
        Logger::new(Some(PlatformType::Douyin), "douyin::message::websocket")
    })
}

// Adjusted imports to use `super` for sibling modules within `message`
use super::gen::PushFrame; // Removed ::douyin
use super::signature; // For generate_signature
use super::web_fetcher::DouyinLiveWebFetcher;
use prost::Message as ProstMessage; // For encoding heartbeat

// Define a type alias for the WebSocket stream for brevity
pub type WsStream = tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>;

// This function will establish the connection and spawn send/heartbeat tasks.
// It returns the read half of the stream and the sender for the outgoing message channel.
pub async fn connect_and_manage_websocket(
    _fetcher: &DouyinLiveWebFetcher, // Changed to immutable reference as we only read from it now
    room_id: &str,
    cookie_header: &str,
    user_unique_id: &str,
) -> Result<(SplitStream<WsStream>, Sender<WsMessage>), Box<dyn std::error::Error + Send + Sync>> {
    logger().info(format!("🚀 开始建立WebSocket连接，房间ID: {room_id}, 用户唯一ID: {user_unique_id}"));
    
    let ws_cookie_header = cookie_header.to_string();
    let current_timestamp_ms = Utc::now().timestamp_millis();
    let first_req_ms = current_timestamp_ms - 100;
    let cursor = format!(
        "d-1_u-1_fh-7392091211001140287_t-{}_r-1",
        current_timestamp_ms
    );
    let internal_ext_original = format!(
        "internal_src:dim|wss_push_room_id:{}|wss_push_did:{}|first_req_ms:{}|fetch_time:{}|seq:1|wss_info:0-{}-0-0|wrds_v:7392094459690748497",
        room_id, user_unique_id, first_req_ms, current_timestamp_ms, current_timestamp_ms
    ).replace("\n", "").replace(" ", "");

    let wss_url_str_for_signature = format!(
        "wss://webcast5-ws-web-hl.douyin.com/webcast/im/push/v2/?app_name=douyin_web&version_code=180800&webcast_sdk_version=1.0.14-beta.0&update_version_code=1.0.14-beta.0&compress=gzip&device_platform=web&cookie_enabled=true&screen_width=1536&screen_height=864&browser_language=zh-CN&browser_platform=Win32&browser_name=Mozilla&browser_version=5.0%20(Windows%20NT%2010.0;%20Win64;%20x64)%20AppleWebKit/537.36%20(KHTML,%20like%20Gecko)%20Chrome/126.0.0.0%20Safari/537.36&browser_online=true&tz_name=Asia/Shanghai&cursor={}&internal_ext={}&host=https://live.douyin.com&aid=6383&live_id=1&did_rule=3&endpoint=live_pc&support_wrds=1&user_unique_id={}&im_path=/webcast/im/fetch/&identity=audience&need_persist_msg_count=15&insert_task_id=&live_reason=&room_id={}&heartbeatDuration=0",
        &cursor,
        &internal_ext_original,
        user_unique_id,
        room_id
    );
    
    logger().debug("🔐 开始生成WebSocket签名");
    let signature = match signature::generate_signature(&wss_url_str_for_signature).await {
        Ok(sign) => {
            logger().debug("✅ 签名生成成功");
            sign
        },
        Err(e) => {
            logger().error(format!("❌ 生成WebSocket签名失败: {}", e));
            return Err(e.into());
        }
    };
    
    let internal_ext_encoded = urlencoding::encode(&internal_ext_original);
    let final_wss_url_str = format!(
        "{}&signature={}",
        format!(
            "wss://webcast5-ws-web-hl.douyin.com/webcast/im/push/v2/?app_name=douyin_web&version_code=180800&webcast_sdk_version=1.0.14-beta.0&update_version_code=1.0.14-beta.0&compress=gzip&device_platform=web&cookie_enabled=true&screen_width=1536&screen_height=864&browser_language=zh-CN&browser_platform=Win32&browser_name=Mozilla&browser_version=5.0%20(Windows%20NT%2010.0;%20Win64;%20x64)%20AppleWebKit/537.36%20(KHTML,%20like%20Gecko)%20Chrome/126.0.0.0%20Safari/537.36&browser_online=true&tz_name=Asia/Shanghai&cursor={}&internal_ext={}&host=https://live.douyin.com&aid=6383&live_id=1&did_rule=3&endpoint=live_pc&support_wrds=1&user_unique_id={}&im_path=/webcast/im/fetch/&identity=audience&need_persist_msg_count=15&insert_task_id=&live_reason=&room_id={}&heartbeatDuration=0",
                &cursor,
                &internal_ext_encoded,
                user_unique_id,
                room_id
            ),
            signature
        );

    logger().debug("🌐 准备建立WebSocket连接");
    
    logger().info("🔄 正在建立WebSocket连接...");
    
    // 重试机制，最多尝试3次连接
    let mut connection_result = None;
    
    for attempt in 1..=3 {
        logger().debug(format!("📞 WebSocket连接尝试 {}/3", attempt));
        
        // 每次重试都从完整URL重新创建请求，确保与原始实现完全一致
        let mut retry_request = final_wss_url_str.clone().into_client_request()?;
        let headers = retry_request.headers_mut();
        // 与原始实现完全一致的请求头设置
        headers.insert("accept", "application/json, text/plain, */*".parse()?);
        headers.insert("accept-language", "zh-CN,zh;q=0.9,en;q=0.8".parse()?);
        headers.insert("cache-control", "no-cache".parse()?);
        headers.insert("pragma", "no-cache".parse()?);
        headers.insert(
            "sec-websocket-extensions",
            "permessage-deflate; client_max_window_bits".parse()?,
        );
        headers.insert("sec-websocket-version", "13".parse()?);
        headers.insert("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36".parse()?);
        headers.insert("Cookie", ws_cookie_header.parse()?);
        
        logger().debug(format!("📝 发送WebSocket连接请求 #{}，URL: {}", attempt, final_wss_url_str.split('?').next().unwrap()));
        
        // 直接调用connect_async，与原始实现一致
        match connect_async(retry_request).await {
            Ok((stream, resp)) => {
                logger().info(format!("✅ WebSocket连接建立成功 #{}，状态码: {}", attempt, resp.status()));
                logger().debug(format!("📋 WebSocket响应头: {:?}", resp.headers()));
                connection_result = Some(stream);
                break;
            },
            Err(e) => {
                logger().error(format!("❌ WebSocket连接建立失败 (尝试 {}/3): {}", attempt, e));
                
                // 如果不是最后一次尝试，等待一段时间后重试
                if attempt < 3 {
                    logger().info("⏱️ 等待1秒后重试WebSocket连接...");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    logger().info("🔄 准备重试WebSocket连接...");
                }
            }
        }
    }
    
    let ws_stream = connection_result.ok_or("WebSocket连接建立失败，已尝试3次")?;

    // 显式指定类型，避免类型推断错误
    let (mut write, read) = ws_stream.split(); // read will be returned

    // Channel for sending messages to the WebSocket Sink
    let (tx, mut rx) = mpsc::channel::<WsMessage>(32);

    // Spawn task for sending heartbeats and other messages from the channel
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(5));
        logger().info("💓 WebSocket心跳任务已启动，每5秒发送一次心跳");
        let mut heartbeat_count = 0;

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    heartbeat_count += 1;
                    logger().debug(format!("💓 准备发送第 {} 次心跳消息", heartbeat_count));
                    
                    // 每次发送心跳时重新创建消息，避免克隆问题
                    let heartbeat_msg_proto = PushFrame {
                        payload_type: "hb".to_string(),
                        log_id: 0,
                        payload: vec![],
                        ..Default::default()
                    };
                    let mut heartbeat_buf = Vec::new();
                    if let Err(e) = heartbeat_msg_proto.encode(&mut heartbeat_buf) {
                        logger().error(format!("❌ 心跳消息编码失败: {}", e));
                        continue;
                    }
                    let ws_ping_msg = WsMessage::Ping(heartbeat_buf);
                    
                    logger().debug(format!("📤 发送心跳消息 #{}，长度: {} 字节", heartbeat_count, ws_ping_msg.len()));
                    
                    if let Err(e) = write.send(ws_ping_msg).await {
                        logger().error(format!("❌ 心跳任务发送错误: {}, 连接可能已断开", e));
                        
                        // 详细记录错误类型
                        match e {
                            tokio_tungstenite::tungstenite::Error::ConnectionClosed => {
                                logger().error("❌ WebSocket连接已关闭");
                                break;
                            },
                            tokio_tungstenite::tungstenite::Error::AlreadyClosed => {
                                logger().error("❌ WebSocket连接已关闭");
                                break;
                            },
                            tokio_tungstenite::tungstenite::Error::Io(e) => {
                                logger().error(format!("❌ WebSocket IO错误: {}", e));
                                // 尝试继续运行，可能是临时网络问题
                            },
                            tokio_tungstenite::tungstenite::Error::Tls(e) => {
                                logger().error(format!("❌ WebSocket TLS错误: {}", e));
                                break;
                            },
                            _ => {
                                logger().error(format!("❌ WebSocket其他错误: {}", e));
                            }
                        }
                    } else {
                        logger().debug(format!("✅ 心跳消息 #{} 发送成功", heartbeat_count));
                    }
                }
                Some(msg_to_send) = rx.recv() => {
                    logger().debug(format!("📤 准备发送消息，类型: {:?}, 长度: {} 字节", 
                        msg_to_send, msg_to_send.len()));
                    
                    if let Err(e) = write.send(msg_to_send).await {
                        logger().error(format!("❌ 消息发送任务错误: {}, 连接可能已断开", e));
                        
                        // 详细记录错误类型
                        match e {
                            tokio_tungstenite::tungstenite::Error::ConnectionClosed => {
                                logger().error("❌ WebSocket连接已关闭");
                                break;
                            },
                            tokio_tungstenite::tungstenite::Error::AlreadyClosed => {
                                logger().error("❌ WebSocket连接已关闭");
                                break;
                            },
                            tokio_tungstenite::tungstenite::Error::Io(e) => {
                                logger().error(format!("❌ WebSocket IO错误: {}", e));
                                // 尝试继续运行，可能是临时网络问题
                            },
                            tokio_tungstenite::tungstenite::Error::Tls(e) => {
                                logger().error(format!("❌ WebSocket TLS错误: {}", e));
                                break;
                            },
                            _ => {
                                logger().error(format!("❌ WebSocket其他错误: {}", e));
                            }
                        }
                    } else {
                        logger().debug("✅ 消息发送成功");
                    }
                }
                else => {
                    // Channel closed, no more messages to send from other tasks
                    logger().info("📡 WebSocket发送通道已关闭");
                    break;
                }
            }
        }
        
        logger().info("🔚 WebSocket发送/心跳任务已结束");
    });

    Ok((read, tx)) // Return the read stream and the sender for other tasks to send messages
}