// src/websocket.rs
use native_tls::TlsStream;
use serde_json::Value;
use std::collections::VecDeque;
use std::net::TcpStream;
use std::time::{Duration, Instant};
use tungstenite::{client, Message, WebSocket};
use url::Url;

use shared::interface::PlatformType;
use shared::logger::Logger;

use super::auth::{init_server_no_cookie, init_server_with_cookie};
use super::models::{BiliMessage, MessageServer, MsgHead};

// 创建日志记录器
static LOGGER: std::sync::OnceLock<Logger> = std::sync::OnceLock::new();

fn logger() -> &'static Logger {
    LOGGER.get_or_init(|| {
        Logger::new(Some(PlatformType::Bilibili), "bilibili::websocket")
    })
}

pub struct BiliLiveClient {
    ws: WebSocket<TlsStream<TcpStream>>,
    auth_msg: String,
    // Keep server host list for reconnection
    host_list: Value,
    // Heartbeat scheduling
    last_heartbeat: Instant,
    heartbeat_interval: Duration,
    // Pending messages parsed from current/previous frames
    pending: VecDeque<BiliMessage>,
}

impl BiliLiveClient {
    pub fn new_with_cookie(cookies: &str, room_id: &str) -> Self {
        let (v, auth) = init_server_with_cookie(cookies, room_id);
        logger().debug(format!("[websocket] server_info host_list: {:?}", v["host_list"]));
        let ws = connect(v["host_list"].clone());
        logger().info(format!("[websocket] connected via cookie for room {}", room_id));
        BiliLiveClient {
            ws,
            auth_msg: serde_json::to_string(&auth).unwrap(),
            host_list: v["host_list"].clone(),
            last_heartbeat: Instant::now(),
            heartbeat_interval: Duration::from_secs(30),
            pending: VecDeque::new(),
        }
    }

    pub fn new_without_cookie(room_id: &str) -> Self {
        let (v, auth) = init_server_no_cookie(room_id);
        logger().debug(format!("[websocket] server_info host_list: {:?}", v["host_list"]));
        let ws = connect(v["host_list"].clone());
        logger().info(format!("[websocket] connected without cookie for room {}", room_id));
        BiliLiveClient {
            ws,
            auth_msg: serde_json::to_string(&auth).unwrap(),
            host_list: v["host_list"].clone(),
            last_heartbeat: Instant::now(),
            heartbeat_interval: Duration::from_secs(30),
            pending: VecDeque::new(),
        }
    }

    pub fn send_auth(&mut self) {
        let pkt = make_packet(self.auth_msg.as_str(), Operation::AUTH);
        logger().debug(format!("[websocket] sending auth packet, len={}", pkt.len()));
        let _ = self.ws.send(Message::Binary(pkt));
    }

    pub fn send_heart_beat(&mut self) {
        let pkt = make_packet("{}", Operation::HEARTBEAT);
        logger().debug(format!("[websocket] sending heartbeat, len={}", pkt.len()));
        let _ = self.ws.send(Message::Binary(pkt));
        // update heartbeat timestamp
        self.last_heartbeat = Instant::now();
    }

    // Periodically send heartbeat to keep the connection alive
    fn maybe_send_heartbeat(&mut self) {
        if self.last_heartbeat.elapsed() >= self.heartbeat_interval {
            logger().debug("[websocket] periodic heartbeat due");
            self.send_heart_beat();
        }
    }

    // Try to reconnect using the cached host list, and re-authenticate
    fn reconnect(&mut self) {
        for attempt in 1..=2 {
            logger().warn(format!("[websocket] attempting reconnect (attempt {attempt}/2)..."));
            match std::panic::catch_unwind({
                let host_list = self.host_list.clone();
                move || connect(host_list)
            }) {
                Ok(new_ws) => {
                    self.ws = new_ws;
                    logger().info(
                        "[websocket] reconnect successful on attempt {attempt}, resending auth"
                    );
                    self.send_auth();
                    return;
                }
                Err(_) => {
                    logger().error(format!("[websocket] reconnect attempt {attempt} failed"));
                }
            }
        }
        logger().error("[websocket] reconnect failed after 2 attempts; will retry on next read cycle");
    }

    // Parse one frame and collect all messages into pending queue
    pub fn parse_ws_message(&mut self, resv: Vec<u8>) -> Option<BiliMessage> {
        logger().debug(format!("[websocket] parse_ws_message: total_len={}", resv.len()));
        let mut offset = 0;
        let header = &resv[0..16];
        let mut head_1 = get_msg_header(header);
        logger().debug(
            format!("[websocket] header op={} ver={} pack_len={} seq={} hdr_size={}",
                head_1.operation,
                head_1.ver,
                head_1.pack_len,
                head_1.seq_id,
                head_1.raw_header_size
            )
        );
        if head_1.operation == 5 || head_1.operation == 8 {
            loop {
                let body: &[u8] = &resv[offset + 16..offset + (head_1.pack_len as usize)];
                logger().debug(
                    format!("[websocket] chunk offset={} pack_len={} ver={} op={}",
                        offset,
                        head_1.pack_len,
                        head_1.ver,
                        head_1.operation
                    )
                );
                if let Some(msg) = self.parse_business_message(head_1, body) {
                    // push and continue to collect more messages
                    self.pending.push_back(msg);
                }
                offset += head_1.pack_len as usize;
                if offset >= resv.len() {
                    break;
                }
                let temp_head = &resv[offset..(offset + 16)];
                head_1 = get_msg_header(temp_head);
            }
        } else if head_1.operation == 3 {
            let mut body: [u8; 4] = [0, 0, 0, 0];
            body[0] = resv[16];
            body[1] = resv[17];
            body[2] = resv[18];
            body[3] = resv[19];
            let _popularity = i32::from_be_bytes(body);
            logger().debug(
                format!("[websocket] popularity message op=3; popularity={}",
                    _popularity
                )
            );
        } else {
            logger().debug(format!("[websocket] unknown op={}, ignoring", head_1.operation));
        }
        None
    }

    fn parse_business_message(&mut self, h: MsgHead, b: &[u8]) -> Option<BiliMessage> {
        logger().debug(
            format!("[websocket] parse_business_message op={} ver={} body_len={} ",
                h.operation,
                h.ver,
                b.len()
            )
        );
        if h.operation == 5 {
            if h.ver == 3 {
                let res: Vec<u8> = match decompress(b) {
                    Ok(r) => r,
                    Err(e) => {
                        logger().error(format!("[websocket] decompress error: {:?}", e));
                        return None;
                    }
                };
                logger().debug(format!("[websocket] decompressed len={}", res.len()));
                return self.parse_ws_message(res);
            } else if h.ver == 0 {
                let s = match String::from_utf8(b.to_vec()) {
                    Ok(s) => s,
                    Err(e) => {
                        logger().error(format!("[websocket] utf8 error: {:?}", e));
                        return None;
                    }
                };
                logger().trace(format!("[websocket] ver0 business json str: {}", s));
                let res_json: Value = match serde_json::from_str(s.as_str()) {
                    Ok(v) => v,
                    Err(e) => {
                        logger().error(format!("[websocket] json parse error: {:?}", e));
                        return None;
                    }
                };
                logger().debug(
                    format!("[websocket] business cmd={}",
                        res_json["cmd"].as_str().unwrap_or("<unknown>")
                    )
                );
                if let Some(m) = handle(res_json) {
                    // push into queue, but do not return immediately
                    self.pending.push_back(m);
                }
                None
            } else {
                logger().debug(format!("[websocket] unknown compression ver={}, skip", h.ver));
                None
            }
        } else if h.operation == 8 {
            logger().debug("[websocket] op=8 (auth reply), sending heartbeat");
            self.send_heart_beat();
            None
        } else {
            logger().debug(format!("[websocket] unsupported business op={}, skip", h.operation));
            None
        }
    }

    pub fn read_once(&mut self) -> Option<BiliMessage> {
        // If we already have pending messages, deliver one immediately
        if let Some(m) = self.pending.pop_front() {
            return Some(m);
        }

        // ensure heartbeat keeps alive
        self.maybe_send_heartbeat();

        let readable = self.ws.can_read();
        logger().debug(format!("[websocket] can_read={} ", readable));
        if self.ws.can_read() {
            let msg = self.ws.read();
            match msg {
                Ok(m) => {
                    let res: Vec<u8> = m.into_data();
                    logger().debug(format!("[websocket] read frame bytes={} ", res.len()));
                    if res.len() >= 16 {
                        // parse and fill pending queue
                        let _ = self.parse_ws_message(res);
                        // return one
                        return self.pending.pop_front();
                    } else {
                        logger().debug("[websocket] frame too short (<16), ignore");
                    }
                }
                Err(e) => {
                    logger().error(format!("[websocket] read error: {:?}", e));
                    // try to reconnect on read error
                    self.reconnect();
                }
            }
        }
        None
    }
}

pub fn gen_message_server_list(list: &serde_json::Value) -> Vec<MessageServer> {
    let mut res: Vec<MessageServer> = Vec::new();
    if let Some(server_list) = list.as_array() {
        logger().debug(format!("[websocket] host_list size={}", server_list.len()));
        if server_list.is_empty() {
            logger().debug("[websocket] host_list empty, using default server");
            res.push(MessageServer::default());
        } else {
            for s in server_list {
                let host = s["host"]
                    .as_str()
                    .unwrap_or("broadcastlv.chat.bilibili.com");
                let port = s["port"].as_i64().unwrap_or(2243) as i32;
                let wss_port = s["wss_port"].as_i64().unwrap_or(443) as i32;
                let ws_port = s["ws_port"].as_i64().unwrap_or(2244) as i32;
                logger().debug(
                    format!("[websocket] server {}:{} (wss_port={}, ws_port={})
",
                        host,
                        port,
                        wss_port,
                        ws_port
                    )
                );
                res.push(MessageServer {
                    host: host.to_string(),
                    port,
                    wss_port,
                    ws_port,
                });
            }
        }
    } else {
        logger().debug("[websocket] host_list not an array, using default server");
        res.push(MessageServer::default());
    }
    res
}

fn find_server(vd: Vec<MessageServer>) -> (String, String, String) {
    let (host, wss_port) = (vd.get(0).unwrap().host.clone(), vd.get(0).unwrap().wss_port);
    logger().debug(
        format!("[websocket] choose server host={} wss_port={}",
            host,
            wss_port
        )
    );
    (
        host.clone(),
        format!("{}:{}", host.clone(), wss_port),
        format!("wss://{}:{}/sub", host, wss_port),
    )
}

pub fn connect(v: Value) -> WebSocket<TlsStream<TcpStream>> {
    let message_server = gen_message_server_list(&v);
    let (host, url, ws_url) = find_server(message_server);
    logger().debug(format!("[websocket] connecting tcp {} and ws {}", url, ws_url));
    
    // 重试机制，最多尝试3次连接
    for attempt in 1..=3 {
        logger().debug(format!("[websocket] connection attempt {}/3", attempt));
        
        match native_tls::TlsConnector::new() {
            Ok(connector) => {
                match TcpStream::connect(url.clone()) {
                    Ok(tcp_stream) => {
                        match connector.connect(host.as_str(), tcp_stream) {
                            Ok(tls_stream) => {
                                match Url::parse(ws_url.as_str()) {
                                    Ok(parsed_url) => {
                                        match client(parsed_url, tls_stream) {
                                            Ok((socket, _resp)) => {
                                                logger().info("[websocket] websocket handshake complete");
                                                return socket;
                                            }
                                            Err(e) => {
                                                logger().error(format!("[websocket] client handshake failed: {:?}", e));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        logger().error(format!("[websocket] failed to parse ws_url: {:?}", e));
                                    }
                                }
                            }
                            Err(e) => {
                                logger().error(format!("[websocket] TLS connection failed: {:?}", e));
                            }
                        }
                    }
                    Err(e) => {
                        logger().error(format!("[websocket] TCP connection failed: {:?}", e));
                    }
                }
            }
            Err(e) => {
                logger().error(format!("[websocket] failed to create TLS connector: {:?}", e));
            }
        }
        
        // 如果不是最后一次尝试，等待一段时间后重试
        if attempt < 3 {
            std::thread::sleep(Duration::from_secs(1));
        }
    }
    
    // 所有尝试都失败，panic
    panic!("[websocket] Failed to connect after 3 attempts");
}

pub enum Operation {
    AUTH,
    HEARTBEAT,
}

pub fn make_packet(body: &str, ops: Operation) -> Vec<u8> {
    let json: Value = serde_json::from_str(body).unwrap();
    let temp = json.to_string();
    let body_content: &[u8] = temp.as_bytes();
    let pack_len: [u8; 4] = ((16 + body.len()) as u32).to_be_bytes();
    let raw_header_size: [u8; 2] = (16 as u16).to_be_bytes();
    let ver: [u8; 2] = (1 as u16).to_be_bytes();
    let operation: [u8; 4] = match ops {
        Operation::AUTH => (7 as u32).to_be_bytes(),
        Operation::HEARTBEAT => (2 as u32).to_be_bytes(),
    };
    let seq_id: [u8; 4] = (1 as u32).to_be_bytes();
    let mut res = pack_len.to_vec();
    res.append(&mut raw_header_size.to_vec());
    res.append(&mut ver.to_vec());
    res.append(&mut operation.to_vec());
    res.append(&mut seq_id.to_vec());
    res.append(&mut body_content.to_vec());
    res
}

pub fn get_msg_header(v_s: &[u8]) -> MsgHead {
    let mut pack_len: [u8; 4] = [0; 4];
    let mut raw_header_size: [u8; 2] = [0; 2];
    let mut ver: [u8; 2] = [0; 2];
    let mut operation: [u8; 4] = [0; 4];
    let mut seq_id: [u8; 4] = [0; 4];
    for (i, v) in v_s.iter().enumerate() {
        if i < 4 {
            pack_len[i] = *v;
            continue;
        }
        if i < 6 {
            raw_header_size[i - 4] = *v;
            continue;
        }
        if i < 8 {
            ver[i - 6] = *v;
            continue;
        }
        if i < 12 {
            operation[i - 8] = *v;
            continue;
        }
        if i < 16 {
            seq_id[i - 12] = *v;
            continue;
        }
    }
    MsgHead {
        pack_len: u32::from_be_bytes(pack_len),
        raw_header_size: u16::from_be_bytes(raw_header_size),
        ver: u16::from_be_bytes(ver),
        operation: u32::from_be_bytes(operation),
        seq_id: u32::from_be_bytes(seq_id),
    }
}

pub fn decompress(body: &[u8]) -> std::io::Result<Vec<u8>> {
    use brotlic::DecompressorReader;
    use std::io::Read;
    let mut decompressed_reader: DecompressorReader<&[u8]> = DecompressorReader::new(body);
    let mut decoded_input = Vec::new();
    let _ = decompressed_reader.read_to_end(&mut decoded_input)?;
    Ok(decoded_input)
}

pub fn handle(json: Value) -> Option<BiliMessage> {
    let category = json["cmd"].as_str().unwrap_or("");
    match category {
        "DANMU_MSG" => Some(BiliMessage::Danmu {
            user: json["info"][2][1]
                .as_str()
                .unwrap_or("<unknown>")
                .to_string(),
            text: json["info"][1].as_str().unwrap_or("").to_string(),
        }),
        "SEND_GIFT" => Some(BiliMessage::Gift {
            user: json["info"][2][1]
                .as_str()
                .unwrap_or("<unknown>")
                .to_string(),
            gift: json["info"][1].as_str().unwrap_or("").to_string(),
        }),
        _ => Some(BiliMessage::Unsupported {
            cmd: category.to_string(),
        }),
    }
}
