use html_escape::decode_html_entities;
use serde::Deserialize;
use serde_json::Value;


use shared::interface::{PlatformError, StreamQuality, StreamUrl};
use shared::logger::Logger;
use shared::types::StreamVariant;

#[derive(Deserialize, Debug)]
struct BetardRoomInfo {
    room_id: Option<Value>,
    show_status: Option<Value>,
}

#[derive(Deserialize, Debug)]
struct BetardResponse {
    room: Option<BetardRoomInfo>,
}

#[derive(Clone, Debug)]
struct DouyuPlayInfo {
    variants: Vec<DouyuRateVariant>,
    cdns: Vec<String>,
}

#[derive(Clone, Debug)]
struct DouyuRateVariant {
    name: String,
    rate: i32,
    bit: Option<i32>,
}

fn value_to_i32(value: &Value) -> Option<i32> {
    match value {
        Value::Number(num) => num.as_i64().map(|n| n as i32),
        Value::String(s) => s.parse::<i32>().ok(),
        _ => None,
    }
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::Number(num) => Some(num.to_string()),
        Value::String(s) => Some(s.to_string()),
        _ => None,
    }
}

struct DouYu {
    #[allow(dead_code)]
    did: String,
    rid: String,
    client: shared::http_client::HttpClient,
    logger: Logger,
}

const DEFAULT_DOUYU_CDN: &str = "ws-h5";
#[allow(dead_code)]
const DEFAULT_DOUYU_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36";
const DEFAULT_DOUYU_DID: &str = "10000000000000000000000000001501";

fn normalize_douyu_cdn(input: Option<&str>) -> &'static str {
    match input
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .as_deref()
    {
        Some("ws-h5") => "ws-h5",
        Some("tct-h5") => "tct-h5",
        Some("ali-h5") => "ali-h5",
        Some("hs-h5") => "hs-h5",
        _ => DEFAULT_DOUYU_CDN,
    }
}

impl DouYu {
    fn new(client: shared::http_client::HttpClient, rid: &str) -> Self {
        Self {
            did: DEFAULT_DOUYU_DID.to_string(),
            rid: rid.to_string(),
            client,
            logger: Logger::new(Some(shared::interface::PlatformType::Douyu), "douyu_stream_url"),
        }
    }

    async fn fetch_room_detail(&self) -> Result<(String, bool), Box<dyn std::error::Error>> {
        let url = format!("https://www.douyu.com/betard/{}", self.rid);
        self.logger.debug(format!("Fetching room detail from: {}", url));
        
        // Create headers for this request
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::REFERER,
            reqwest::header::HeaderValue::from_str(&format!("https://www.douyu.com/{}", self.rid))?
        );
        
        let text = self.client.get_text_with_headers(&url, Some(headers)).await?;
        let json: BetardResponse = serde_json::from_str(&text)?;

        let room = json.room.ok_or("Missing room data")?;
        let room_id_value = room.room_id.ok_or("Missing room_id")?;
        let room_id = value_to_string(&room_id_value).ok_or("Invalid room_id")?;
        let show_status = room
            .show_status
            .as_ref()
            .and_then(value_to_i32)
            .unwrap_or(0);
        
        let is_live = show_status == 1;
        self.logger.info(format!("Room detail fetched: room_id={}, is_live={}", room_id, is_live));
        
        Ok((room_id, is_live))
    }

    #[allow(dead_code)]
    #[allow(dead_code)]
    async fn get_h5_enc(&self, room_id: &str) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!("https://www.douyu.com/swf_api/homeH5Enc?rids={}", room_id);
        
        // Create headers for this request
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::REFERER,
            reqwest::header::HeaderValue::from_str(&format!("https://www.douyu.com/{}", room_id))?
        );
        
        let text = self.client.get_text_with_headers(&url, Some(headers)).await?;
        let json: Value = serde_json::from_str(&text)?;

        let error_code = json.get("error").and_then(value_to_i32).unwrap_or(-1);
        if error_code != 0 {
            return Err(format!("homeH5Enc error: {}", error_code).into());
        }

        let key = format!("room{}", room_id);
        let crptext = json
            .get("data")
            .and_then(|v| v.get(&key))
            .and_then(|v| v.as_str())
            .ok_or("Missing homeH5Enc data")?;
        Ok(crptext.to_string())
    }

    async fn build_sign_params(
        &self,
        _room_id: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // 暂时简化实现，返回空字符串，后续需要实现Rust版本的签名算法
        Ok("" .to_string())
    }

    async fn get_play_qualities(
        &self,
        room_id: &str,
        sign_data: &str,
    ) -> Result<DouyuPlayInfo, Box<dyn std::error::Error>> {
        let payload = format!(
            "{}&cdn=&rate=-1&ver=Douyu_223061205&iar=1&ive=1&hevc=0&fa=0",
            sign_data
        );
        let url = format!("https://www.douyu.com/lapi/live/getH5Play/{}", room_id);
        
        // Create headers for this request
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/x-www-form-urlencoded")
        );
        
        // Use the shared HttpClient's post_form_json method
        let json: Value = self.client
            .post_form_json(&url, &payload)
            .await?;

        let error_code = json.get("error").and_then(value_to_i32).unwrap_or(-1);
        if error_code != 0 {
            let msg = json
                .get("msg")
                .and_then(|v| v.as_str())
                .unwrap_or("getH5Play failed");
            return Err(format!("getH5Play error {}: {}", error_code, msg).into());
        }

        let data = json.get("data").ok_or("No data field in response")?;
        let cdns = data
            .get("cdnsWithName")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        item.get("cdn")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();

        let mut cdns_sorted = cdns;
        cdns_sorted.sort_by(|a, b| {
            let a_is_scdn = a.starts_with("scdn");
            let b_is_scdn = b.starts_with("scdn");
            (a_is_scdn, a).cmp(&(b_is_scdn, b))
        });

        let variants = data
            .get("multirates")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let name = item
                            .get("name")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())?;
                        let rate_value = item.get("rate").and_then(value_to_i32)?;
                        let bit_value = item.get("bit").and_then(value_to_i32);
                        Some(DouyuRateVariant {
                            name,
                            rate: rate_value,
                            bit: bit_value,
                        })
                    })
                    .collect::<Vec<DouyuRateVariant>>()
            })
            .unwrap_or_default();

        Ok(DouyuPlayInfo {
            variants,
            cdns: cdns_sorted,
        })
    }

    async fn get_play_url(
        &self,
        room_id: &str,
        sign_data: &str,
        rate: i32,
        cdn: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let payload = format!(
            "{}&cdn={}&rate={}&ver=Douyu_223061205&iar=1&ive=1&hevc=0&fa=0",
            sign_data, cdn, rate
        );
        let url = format!("https://www.douyu.com/lapi/live/getH5Play/{}", room_id);
        self.logger.debug(format!("Fetching play URL from: {} with cdn={}, rate={}", url, cdn, rate));
        
        // Use the shared HttpClient's post_form_json method
        let json: Value = self.client
            .post_form_json(&url, &payload)
            .await?;

        let error_code = json.get("error").and_then(value_to_i32).unwrap_or(-1);
        if error_code != 0 {
            let msg = json
                .get("msg")
                .and_then(|v| v.as_str())
                .unwrap_or("getH5Play failed");
            self.logger.error(format!("getH5Play API error {}: {}", error_code, msg));
            return Err(format!("getH5Play error {}: {}", error_code, msg).into());
        }

        let data = json.get("data").ok_or("No data field in response")?;
        let rtmp_url = data
            .get("rtmp_url")
            .and_then(|v| v.as_str())
            .ok_or("No rtmp_url field")?;
        let rtmp_live = data
            .get("rtmp_live")
            .and_then(|v| v.as_str())
            .ok_or("No rtmp_live field")?;
        let rtmp_live = decode_html_entities(rtmp_live).to_string();
        let stream_url = format!("{}/{}", rtmp_url, rtmp_live);
        
        self.logger.info(format!("Successfully generated stream URL for room_id {}: {}", room_id, stream_url));
        Ok(stream_url)
    }

    fn select_cdn(requested: Option<&str>, available: &[String]) -> String {
        if let Some(cdn) = requested {
            let trimmed = cdn.trim();
            if !trimmed.is_empty() {
                let target = trimmed.to_ascii_lowercase();
                if let Some(hit) = available
                    .iter()
                    .find(|item| item.to_ascii_lowercase() == target)
                {
                    return hit.clone();
                }
            }
        }
        available
            .first()
            .cloned()
            .unwrap_or_else(|| normalize_douyu_cdn(requested).to_string())
    }

    pub async fn get_real_url(&self, cdn: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
        let (real_room_id, is_live) = self.fetch_room_detail().await?;
        if !is_live {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "主播未开播",
            )));
        }

        let sign_data = self.build_sign_params(&real_room_id).await?;
        let play_info = self.get_play_qualities(&real_room_id, &sign_data).await?;
        let best_rate = play_info
            .variants
            .iter()
            .map(|variant| variant.rate)
            .max()
            .unwrap_or(0);
        let selected_cdn = Self::select_cdn(cdn, &play_info.cdns);
        self.get_play_url(&real_room_id, &sign_data, best_rate, &selected_cdn)
            .await
    }

    pub async fn get_real_url_with_quality(
        &self,
        quality: &str,
        cdn: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let (real_room_id, is_live) = self.fetch_room_detail().await?;
        if !is_live {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "主播未开播",
            )));
        }

        let sign_data = self.build_sign_params(&real_room_id).await?;
        let play_info = self.get_play_qualities(&real_room_id, &sign_data).await?;
        let selected_rate = Self::resolve_rate_for_quality(quality, &play_info.variants)
            .or_else(|| play_info.variants.iter().map(|v| v.rate).max())
            .unwrap_or(0);
        self.logger.info(format!(
            "Requested quality '{}', resolved rate {} (available variants: {:?})",
            quality, selected_rate, play_info.variants
        ));
        let selected_cdn = Self::select_cdn(cdn, &play_info.cdns);
        self.get_play_url(&real_room_id, &sign_data, selected_rate, &selected_cdn)
            .await
    }

    fn resolve_rate_for_quality(quality: &str, variants: &[DouyuRateVariant]) -> Option<i32> {
        if variants.is_empty() {
            return None;
        }

        let trimmed = quality.trim();
        let ascii_lower = trimmed.to_ascii_lowercase();
        let canonical = if trimmed.contains('原') || ascii_lower == "origin" {
            "原画"
        } else if trimmed.contains('高') || ascii_lower == "high" {
            "高清"
        } else if trimmed.contains('标') || ascii_lower == "standard" {
            "标清"
        } else {
            trimmed
        };

        let find_by_keywords = |keywords: &[&str], exclude_zero: bool| -> Option<i32> {
            for keyword in keywords {
                if let Some(item) = variants.iter().find(|v| v.name.contains(keyword)) {
                    if exclude_zero && item.rate == 0 {
                        continue;
                    }
                    return Some(item.rate);
                }
            }
            None
        };

        match canonical {
            "原画" => {
                if let Some(item) = variants.iter().find(|v| v.rate == 0) {
                    return Some(item.rate);
                }
                if let Some(rate) = find_by_keywords(&["原画", "蓝光8M", "蓝光"], false) {
                    return Some(rate);
                }
                variants.iter().map(|v| v.rate).min()
            }
            "高清" => {
                if let Some(item) = variants.iter().find(|v| v.rate == 4) {
                    return Some(item.rate);
                }
                if let Some(rate) = find_by_keywords(&["蓝光", "蓝光4M"], false) {
                    return Some(rate);
                }
                if let Some(rate) = find_by_keywords(&["超清"], true) {
                    return Some(rate);
                }
                if let Some(rate) = find_by_keywords(&["高清"], true) {
                    return Some(rate);
                }
                variants
                    .iter()
                    .filter(|v| v.rate != 0)
                    .max_by_key(|v| v.bit.unwrap_or(0))
                    .map(|v| v.rate)
                    .or_else(|| {
                        variants
                            .iter()
                            .filter(|v| v.rate != 0)
                            .max_by_key(|v| v.rate)
                            .map(|v| v.rate)
                    })
            }
            "标清" => {
                if let Some(item) = variants.iter().find(|v| v.rate == 3) {
                    return Some(item.rate);
                }
                if let Some(rate) = find_by_keywords(&["超清"], true) {
                    return Some(rate);
                }
                if let Some(rate) = find_by_keywords(&["流畅"], true) {
                    return Some(rate);
                }
                if let Some(rate) = find_by_keywords(&["标清"], true) {
                    return Some(rate);
                }
                if let Some(rate) = find_by_keywords(&["普清"], true) {
                    return Some(rate);
                }
                variants
                    .iter()
                    .filter(|v| v.rate != 0)
                    .min_by_key(|v| v.bit.unwrap_or(i32::MAX))
                    .map(|v| v.rate)
                    .or_else(|| {
                        variants
                            .iter()
                            .filter(|v| v.rate != 0)
                            .min_by_key(|v| v.rate)
                            .map(|v| v.rate)
                    })
            }
            _ => {
                if let Some(rate) = find_by_keywords(&[canonical], false) {
                    return Some(rate);
                }
                None
            }
        }
    }
}

// These functions are kept for backward compatibility but should be updated to use HttpClient
pub async fn get_stream_url(
    room_id: &str,
    cdn: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    // Create a default HttpClient for backward compatibility
    let client = shared::http_client::HttpClient::new()?;
    let douyu = DouYu::new(client, room_id);
    let url = douyu.get_real_url(cdn).await?;
    Ok(url)
}

pub async fn get_stream_url_with_quality(
    room_id: &str,
    quality: &str,
    cdn: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    // Create a default HttpClient for backward compatibility
    let client = shared::http_client::HttpClient::new()?;
    let douyu = DouYu::new(client, room_id);
    let url = douyu.get_real_url_with_quality(quality, cdn).await?;
    Ok(url)
}

pub async fn get_stream_url_for_platform(
    client: &shared::http_client::HttpClient,
    room_id: &str,
    quality: Option<StreamQuality>,
) -> Result<(StreamUrl, Option<Value>), PlatformError> {
    let douyu = DouYu::new(client.clone(), room_id);
    
    // Convert StreamQuality to Douyu's quality string
    let quality_str = match quality {
        Some(StreamQuality::UltraHD) => "超清",
        Some(StreamQuality::HD) => "高清",
        Some(StreamQuality::SD) => "标清",
        Some(StreamQuality::LD) => "流畅",
        Some(StreamQuality::Auto) => "",
        Some(StreamQuality::Custom(ref q)) => q.as_str(),
        Some(StreamQuality::Original) => "原画",
        Some(StreamQuality::K4) => "4K",
        Some(StreamQuality::K2) => "2K",
        Some(StreamQuality::P1080_60) => "1080P60",
        Some(StreamQuality::P720_60) => "720P60",
        None => "",
    };
    
    // Get stream URL
    let stream_url = if quality_str.is_empty() {
        douyu.get_real_url(None).await
    } else {
        douyu.get_real_url_with_quality(quality_str, None).await
    };
    
    let url = stream_url
        .map_err(|e| PlatformError::Api(e.to_string()))?;
    
    // Build StreamVariant
    let stream_variant = StreamVariant {
        url: url.clone(),
        format: Some("flv".to_string()), // Douyu typically uses FLV format
        desc: quality.clone().map(|q| format!("{:?}", q)),
        qn: None,
        protocol: Some("http".to_string()),
    };
    
    // For now, we'll keep the current raw data format
    // In future, we should modify the DouYu struct to return the actual raw API responses
    let raw = Some(serde_json::json!({
        "room_id": room_id,
        "quality": quality_str,
        "stream_url": url.clone(),
        "raw_api_responses": serde_json::json!({
            "room_detail_url": format!("https://www.douyu.com/betard/{}", room_id),
            "quality": quality_str
        })
    }));
    
    // Build unified StreamUrl
    let stream_url = StreamUrl {
        primary_url: url.clone(),
        upstream_url: Some(url.clone()),
        available_streams: vec![stream_variant],
        room_info: None, // We don't have room info here, but could fetch it if needed
        streamer_info: None,
        other: None,
        raw: raw.clone(),
    };
    
    // Return both StreamUrl and raw data
    Ok((stream_url, raw))
}
