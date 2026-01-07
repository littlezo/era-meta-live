use futures_util::{SinkExt, StreamExt};
use tars_stream::prelude::*;
use tokio::sync::oneshot;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};

// 使用统一的MessageCallback类型和Logger
use shared::interface::{MessageCallback, Message, MessageType, PlatformType, MessageListener, ListenerStatus, PlatformError};
use shared::logger::Logger;
use shared::EventBroadcaster;
use shared::PlatformEvent;

const WS_URL: &str = "wss://cdnws.api.huya.com";
// 恢复 HEARTBEAT 常量（被误删），供心跳发送使用
const HEARTBEAT: &'static [u8] = b"\x00\x03\x1d\x00\x00\x69\x00\x00\x00\x69\x10\x03\x2c\x3c\x4c\x56\x08\x6f\x6e\x6c\x69\x6e\x65\x75\x69\x66\x0f\x4f\x6e\x55\x73\x65\x72\x48\x65\x61\x72\x74\x42\x65\x61\x74\x7d\x00\x00\x3c\x08\x00\x01\x06\x04\x74\x52\x65\x71\x1d\x00\x00\x2f\x0a\x0a\x0c\x16\x00\x26\x00\x36\x07\x61\x64\x72\x5f\x77\x61\x70\x46\x00\x0b\x12\x03\xae\xf0\x0f\x22\x03\xae\xf0\x0f\x3c\x42\x6d\x52\x02\x60\x5c\x60\x01\x7c\x82\x00\x0b\xb0\x1f\x9c\xac\x0b\x8c\x98\x0c\xa8\x0c";
// const HEARTBEAT_BASE64: &str = "ABQdAAwsNgBM"; // same as Python
#[allow(dead_code)]
const HEARTBEAT_BASE64: &str = "ABQdAAwsNgBM"; // same as Python

// Minimal JCE/TARS codec for required Huya structures

async fn fetch_huya_ids(room_id: &str) -> Result<(i64, i64), String> {
    let url = format!(
        "https://mp.huya.com/cache.php?m=Live&do=profileRoom&roomid={}&showSecret=1",
        room_id
    );
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/118.0.0.0 Safari/537.36")
        .header("Accept", "*/*")
        .header("Origin", "https://www.huya.com")
        .header("Referer", "https://www.huya.com/")
        .send().await.map_err(|e| e.to_string())?;
    let text = resp.text().await.map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    let status = v.get("status").and_then(|x| x.as_i64()).unwrap_or(0);
    if status != 200 {
        return Err("房间未开播或无流信息，无法获取弹幕参数".to_string());
    }

    let data = v.get("data").ok_or_else(|| "缺少data".to_string())?;
    let ayyuid = data
        .get("profileInfo")
        .and_then(|x| x.get("yyid"))
        .and_then(|x| x.as_i64())
        .unwrap_or(0);

    let base_list = data
        .get("stream")
        .and_then(|x| x.get("baseSteamInfoList"))
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();

    let top_sid = if let Some(first) = base_list.get(0) {
        first
            .get("lChannelId")
            .and_then(|x| x.as_i64())
            .unwrap_or(0)
    } else {
        0
    };

    if top_sid == 0 {
        return Err("未找到频道ID，房间可能未开播".to_string());
    }

    let logger = Logger::new(Some(PlatformType::Huya), "huya_message");
    logger.info(format!(
        "fetch_huya_ids: room_id={} yyid={} topSid={}",
        room_id, ayyuid, top_sid
    ));
    Ok((ayyuid, top_sid))
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct HuyaJoinParams {
    pub yyid: i64,
    pub top_sid: i64,
}

pub async fn fetch_huya_join_params(room_id: String) -> Result<HuyaJoinParams, String> {
    match fetch_huya_ids(&room_id).await {
        Ok((ayyuid, top_sid)) => Ok(HuyaJoinParams {
            yyid: ayyuid,
            top_sid,
        }),
        Err(e) => Err(e),
    }
}

/// Huya消息监听器实现
pub struct HuyaMessageListener {
    room_id: String,
    stop_tx: Option<oneshot::Sender<()>>,
    status: ListenerStatus,
    message_count: u32,
}

#[async_trait::async_trait]
impl MessageListener for HuyaMessageListener {
    // 停止消息监听
    async fn stop(&mut self) -> Result<(), PlatformError> {
        let logger = Logger::new(Some(PlatformType::Huya), "huya_message");
        logger.info(format!("stop called for room_id: {}", self.room_id));
        
        // 更新状态
        self.status.status = "DISCONNECTING".to_string();
        self.status.last_update = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // 发送停止信号
        if let Some(tx) = self.stop_tx.take() {
            logger.info("Sending shutdown to Huya listener task.");
            if let Err(e) = tx.send(()) {
                logger.error(format!("Failed to send stop signal: {:?}", e));
            }
        }
        
        // 更新状态
        self.status.status = "DISCONNECTED".to_string();
        self.status.last_update = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        Ok(())
    }
    
    // 获取监听器状态
    fn status(&self) -> ListenerStatus {
        self.status.clone()
    }
    
    // 获取消息计数
    fn message_count(&self) -> u32 {
        self.message_count
    }
}

// 与LivePlatform trait兼容的启动消息监听器函数
pub async fn start_message_listener(
    room_id: &str,
    callback: MessageCallback
) -> Result<Box<dyn MessageListener>, String> {
    let logger = Logger::new(Some(PlatformType::Huya), "huya_message");
    logger.info(format!("start listener room_id={}", room_id));

    // 创建关闭通道
    let (tx_shutdown, mut rx_shutdown) = oneshot::channel::<()>();
    let room_id_clone = room_id.to_string();

    // 创建初始监听器状态
    let initial_status = ListenerStatus {
        platform: PlatformType::Huya,
        room_id: room_id.to_string(),
        status: "CONNECTING".to_string(),
        message_count: 0,
        last_update: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    };
    
    // 创建监听器实例
    let listener = HuyaMessageListener {
        room_id: room_id.to_string(),
        stop_tx: Some(tx_shutdown),
        status: initial_status.clone(),
        message_count: 0,
    };

    tokio::spawn(async move {
        let logger = Logger::new(Some(PlatformType::Huya), "huya_message");
        logger.info(format!("spawned worker for room_id={}", room_id_clone));
        
        // 1) 获取 ws 与注册数据
        let (ws_url, reg_data) = match get_ws_info_tars(&room_id_clone).await {
            Ok(v) => v,
            Err(e) => {
                // 发送错误消息给回调
                let error_msg = Message {
                    id: None,
                    message_type: MessageType::System,
                    user: "系统".to_string(),
                    content: format!("Huya房间信息获取失败: {}", e),
                    user_level: None,
                    fans_level: None,
                    fans_club_level: None,
                    badge_name: None,
                    badge_level: None,
                    uid: None,
                    color: None,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                    room_id: room_id_clone.clone(),
                    platform: PlatformType::Huya,
                    gift_name: None,
                    gift_count: None,
                    gift_price: None,
                    gift_total: None,
                    super_chat_price: None,
                    super_chat_duration: None,
                    combo_count: None,
                    combo_user: None,
                    raw: None,
                    other: None,
                };
                callback(error_msg);
                return;
            }
        };

        logger.info(format!("ws_url={} reg_len={}", ws_url, reg_data.len()));

        // 2) 连接 WebSocket
        logger.info(format!("connecting to {}", ws_url));
        let (ws_stream, _) = match connect_async(&ws_url).await {
            Ok(v) => v,
            Err(e) => {
                // 发送错误消息给回调
                let error_msg = Message {
                    id: None,
                    message_type: MessageType::System,
                    user: "系统".to_string(),
                    content: format!("Huya弹幕连接失败: {}", e),
                    user_level: None,
                    fans_level: None,
                    fans_club_level: None,
                    badge_name: None,
                    badge_level: None,
                    uid: None,
                    color: None,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                    room_id: room_id_clone.clone(),
                    platform: PlatformType::Huya,
                    gift_name: None,
                    gift_count: None,
                    gift_price: None,
                    gift_total: None,
                    super_chat_price: None,
                    super_chat_duration: None,
                    combo_count: None,
                    combo_user: None,
                    raw: None,
                    other: None,
                };
                callback(error_msg);
                return;
            }
        };

        let (mut ws_write, mut ws_read) = ws_stream.split();
        if let Err(e) = ws_write.send(WsMessage::Binary(reg_data)).await {
            // 发送错误消息给回调
            let error_msg = Message {
                id: None,
                message_type: MessageType::System,
                user: "系统".to_string(),
                content: format!("Huya注册数据发送失败: {}", e),
                user_level: None,
                fans_level: None,
                fans_club_level: None,
                badge_name: None,
                badge_level: None,
                uid: None,
                color: None,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                room_id: room_id_clone.clone(),
                platform: PlatformType::Huya,
                gift_name: None,
                gift_count: None,
                gift_price: None,
                gift_total: None,
                super_chat_price: None,
                super_chat_duration: None,
                combo_count: None,
                combo_user: None,
                raw: None,
                other: None,
            };
            callback(error_msg);
            return;
        }

        // 3) 心跳与接收
        let logger_hb = logger.clone();
        let hb_task = async move {
            let mut hb_seq = 0usize;
            loop {
                if let Err(e) = ws_write.send(WsMessage::Binary(HEARTBEAT.into())).await {
                    logger_hb.error(format!("Huya心跳发送失败: {:?}", e));
                    break;
                }
                hb_seq += 1;
                logger_hb.debug(format!("heartbeat sent #{} ", hb_seq));
                sleep(Duration::from_secs(20)).await;
            }
            Err::<(), anyhow::Error>(anyhow::anyhow!("Huya心跳发送失败"))
        };

        let logger_recv = logger.clone();
        let room_id_recv = room_id_clone.clone();
        let recv_task = async move {
            while let Some(m) = ws_read.next().await {
                let m = match m {
                    Ok(x) => x,
                    Err(e) => return Err(anyhow::anyhow!(e)),
                };
                match m {
                    WsMessage::Binary(bin) => {
                        let (top_cmd, nested_cmd) = peek_cmds(&bin);
                        logger_recv.debug(format!(
                            "WS msg: len={} top_cmd={:?} nested_cmd={:?}",
                            bin.len(),
                            top_cmd,
                            nested_cmd
                        ));
                        match decode_msg_tars(&bin) {
                            Ok(Some((nick, text))) => {
                                logger_recv.debug(format!("decoded chat: {} -> {}", nick, text));
                                
                                // 构建统一的Message结构体并发送给回调
                                let message = Message {
                                    id: None,
                                    message_type: MessageType::Danmaku,
                                    user: nick,
                                    content: text,
                                    user_level: None,
                                    fans_level: None,
                                    fans_club_level: None,
                                    badge_name: None,
                                    badge_level: None,
                                    uid: None,
                                    color: None,
                                    timestamp: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_millis() as u64,
                                    room_id: room_id_recv.clone(),
                                    platform: PlatformType::Huya,
                                    gift_name: None,
                                    gift_count: None,
                                    gift_price: None,
                                    gift_total: None,
                                    super_chat_price: None,
                                    super_chat_duration: None,
                                    combo_count: None,
                                    combo_user: None,
                                    raw: None,
                                    other: None,
                                };
                                callback(message.clone());
                                // 使用事件广播器发布消息
                                EventBroadcaster::global().publish(PlatformEvent::Message(message));
                            }
                            Ok(None) => {
                                if top_cmd == Some(7) {
                                    logger_recv.debug(format!(
                                        "non-chat or empty msg, nested={:?}",
                                        nested_cmd
                                    ));
                                }
                            }
                            Err(e) => {
                                logger_recv.error(format!("decode error: {:?}", e));
                            }
                        }
                    }
                    other => {
                        logger_recv.debug(format!("non-binary ws message: {:?}", other));
                    }
                }
            }
            anyhow::Ok(())
        };

        let logger_select = logger;
        let room_id_select = room_id_clone;
        tokio::select! {
            _ = &mut rx_shutdown => {
                // 主动关闭
                logger_select.info(format!("stop signal received, terminating listener for room_id={}", room_id_select));
            }
            it = hb_task => {
                if let Err(e) = it { 
                    logger_select.error(format!("{}", e)); 
                }
            }
            it = recv_task => {
                if let Err(e) = it { 
                    logger_select.error(format!("接收失败: {}", e)); 
                }
            }
        }
    });

    Ok(Box::new(listener))
}

// 采用 tars_stream 的实现（参考 all_in_one.rs），保留 Tauri 命令，对旧 jce 逻辑停用

struct HuyaUser {
    _uid: i64,
    _imid: i64,
    name: String,
    _gender: i32,
}

struct HuyaMessageFmt {
    color: i32,
}

impl StructFromTars for HuyaUser {
    fn _decode_from(decoder: &mut TarsDecoder) -> Result<Self, DecodeErr> {
        let uid = decoder.read_int64(0, false, -1)?;
        let imid = decoder.read_int64(1, false, -1)?;
        let name = decoder.read_string(2, false, "".to_string())?;
        let gender = decoder.read_int32(3, false, -1)?;
        Ok(HuyaUser {
            _uid: uid,
            _imid: imid,
            name,
            _gender: gender,
        })
    }
}

impl StructFromTars for HuyaMessageFmt {
    fn _decode_from(decoder: &mut TarsDecoder) -> Result<Self, DecodeErr> {
        let color = decoder.read_int32(0, false, 16777215)?;
        Ok(HuyaMessageFmt { color })
    }
}

fn peek_cmds(data: &[u8]) -> (Option<i32>, Option<i64>) {
    let mut ios = TarsDecoder::from(data);
    let top_cmd = ios.read_int32(0, false, -1).ok();
    let nested_cmd = ios
        .read_bytes(1, false, Default::default())
        .ok()
        .and_then(|b1| {
            let mut inner = TarsDecoder::from(b1.as_ref());
            inner.read_int32(1, false, -1).ok().map(|v| v as i64)
        });
    (top_cmd, nested_cmd)
}

fn find_uid_in_json(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::Object(map) => {
            for (k, val) in map {
                let key = k.to_lowercase();
                if key == "ayyuid" || key == "yyuid" || key == "lp" || key == "uid" {
                    match val {
                        serde_json::Value::String(s) => {
                            if !s.is_empty() {
                                return Some(s.clone());
                            }
                        }
                        serde_json::Value::Number(n) => return Some(n.to_string()),
                        _ => {}
                    }
                }
                if let Some(found) = find_uid_in_json(val) {
                    return Some(found);
                }
            }
            None
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                if let Some(found) = find_uid_in_json(item) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

fn gen_ua() -> String {
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string()
}

async fn get_ws_info_tars(room_id_or_url: &str) -> Result<(String, Vec<u8>), String> {
    let logger = Logger::new(Some(PlatformType::Huya), "huya_message");
    
    let url = if room_id_or_url.starts_with("http") {
        reqwest::Url::parse(room_id_or_url).map_err(|e| e.to_string())?
    } else {
        reqwest::Url::parse(&format!("https://www.huya.com/{}", room_id_or_url))
            .map_err(|e| e.to_string())?
    };
    let rid = url
        .path_segments()
        .and_then(|s| s.last())
        .ok_or_else(|| "房间ID解析失败".to_string())?;
    logger.info(format!("get_ws_info_tars rid={}", rid));

    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())?;
    let resp_text = client
        .get(format!("https://www.huya.com/{}", rid))
        .header("User-Agent", gen_ua())
        .header("Referer", "https://www.huya.com/")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    logger.debug(format!("fetched room page len={}", resp_text.len()));

    // 先尝试 TT_PROFILE_INFO 提取 lp
    let mut ayyuid = {
        let re_prof = regex::Regex::new(r#"var\s+TT_PROFILE_INFO\s*=\s*(\{[\s\S]*?\});"#)
            .map_err(|e| e.to_string())?;
        if let Some(cap) = re_prof.captures(&resp_text) {
            if let Ok(j) = serde_json::from_str::<serde_json::Value>(&cap[1]) {
                j.pointer("/lp")
                    .map(|v| v.to_string().replace('"', ""))
                    .unwrap_or_default()
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    };
    if ayyuid.is_empty() {
        // 直接匹配 lp
        let re_lp = regex::Regex::new(r#"\\\"lp\\\"\s*:\s*\\\"?(\d+)\\\"?"#)
            .map_err(|e| e.to_string())?;
        if let Some(cap) = re_lp.captures(&resp_text) {
            ayyuid = cap.get(1).unwrap().as_str().to_string();
        }
    }
    if ayyuid.is_empty() {
        // 匹配 ayyuid / yyuid
        let re_ayyuid = regex::Regex::new(r#"\\\"ayyuid\\\"\s*:\s*\\\"?(\d+)\\\"?"#)
            .map_err(|e| e.to_string())?;
        let re_yyuid = regex::Regex::new(r#"\\\"yyuid\\\"\s*:\s*\\\"?(\d+)\\\"?"#)
            .map_err(|e| e.to_string())?;
        if let Some(cap) = re_ayyuid.captures(&resp_text) {
            ayyuid = cap.get(1).unwrap().as_str().to_string();
        } else if let Some(cap) = re_yyuid.captures(&resp_text) {
            ayyuid = cap.get(1).unwrap().as_str().to_string();
        }
    }
    if ayyuid.is_empty() {
        // 回退：调用 mp.huya.com
        let url_api = format!(
            "https://mp.huya.com/cache.php?m=Live&do=profileRoom&roomid={}",
            rid
        );
        let text = client
            .get(&url_api)
            .header("User-Agent", gen_ua())
            .send()
            .await
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;
        if let Ok(j) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(found) = find_uid_in_json(&j) {
                ayyuid = found;
            }
        }
    }
    if ayyuid.is_empty() {
        ayyuid = rid.to_string();
    }
    logger.info(format!("final ayyuid={}", ayyuid));

    let mut topics = Vec::new();
    topics.push(format!("live:{}", ayyuid));
    topics.push(format!("chat:{}", ayyuid));
    logger.debug(format!("topics={:?}", topics));

    let mut oos = TarsEncoder::new();
    oos.write_list(0, &topics).map_err(|e| e.to_string())?;
    oos.write_string(1, &"".to_owned())
        .map_err(|e| e.to_string())?;

    let mut wscmd = TarsEncoder::new();
    wscmd.write_int32(0, 16).map_err(|e| e.to_string())?;
    wscmd
        .write_bytes(1, &oos.to_bytes())
        .map_err(|e| e.to_string())?;
    let b = wscmd.to_bytes();
    logger.debug(format!("reg payload built, len={}", b.len()));

    Ok((WS_URL.to_owned(), b.as_ref().to_vec()))
}

fn decode_msg_tars(data: &[u8]) -> anyhow::Result<Option<(String, String)>> {
    let logger = Logger::new(Some(PlatformType::Huya), "huya_message");
    let mut ret: Option<(String, String)> = None;
    let mut ios = TarsDecoder::from(data);
    let top = ios.read_int32(0, false, -1)?;
    if top != 7 {
        logger.debug(format!("ignore msg: top_cmd={}", top));
        return Ok(ret);
    }
    let b1 = ios.read_bytes(1, false, Default::default())?;
    let mut inner = TarsDecoder::from(b1.as_ref());
    let nested = inner.read_int32(1, false, -1).unwrap_or(-1);
    let b2 = inner.read_bytes(2, false, Default::default())?;
    logger.debug(format!("nested={} payload_len={}", nested, b2.len()));
    let mut payload = TarsDecoder::from(b2.as_ref());

    if nested == 1400 {
        let user = payload
            .read_struct(
                0,
                false,
                HuyaUser {
                    _uid: -1,
                    _imid: -1,
                    name: "".to_owned(),
                    _gender: 1,
                },
            )
            .unwrap_or(HuyaUser {
                _uid: -1,
                _imid: -1,
                name: "".to_owned(),
                _gender: 1,
            });
        let text = payload
            .read_string(3, false, "".to_owned())
            .unwrap_or_default();
        let fmt = payload
            .read_struct(6, false, HuyaMessageFmt { color: 16777215 })
            .unwrap_or(HuyaMessageFmt { color: 16777215 });
        if !text.is_empty() {
            let nick = if !user.name.is_empty() {
                user.name
            } else {
                "匿名".to_string()
            };
            let _color_hex = format!("{:06x}", if fmt.color <= 0 { 16777215 } else { fmt.color });
            logger.debug(format!(
                "decoded nested=1400 nick={} text={}",
                nick, text
            ));
            ret = Some((nick, text));
        } else {
            logger.debug("empty text in nested=1400");
        }
    } else {
        logger.debug(format!("non-chat nested={}, skip", nested));
    }
    Ok(ret)
}
