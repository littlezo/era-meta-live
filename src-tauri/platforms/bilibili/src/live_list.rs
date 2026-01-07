use md5;
use std::sync::Arc;
use serde_json::Value;

// 引入 generate_bilibili_w_webid 以便在缺失时后端自动初始化
use crate::state::BilibiliState;
use shared::interface::LiveList;
use shared::interface::PlatformType;
use shared::http_client::HttpClient;
use shared::logger::Logger;

// 创建静态日志记录器
static LOGGER: std::sync::OnceLock<Logger> = std::sync::OnceLock::new();

fn logger() -> &'static Logger {
    LOGGER.get_or_init(|| {
        Logger::new(Some(PlatformType::Bilibili), "bilibili::live_list")
    })
}

pub async fn fetch_live_list(
    client: &HttpClient,
    area_id: String,
    parent_area_id: String,
    page: u32,
    state: &Arc<BilibiliState>,
) -> Result<(LiveList, Option<Value>), String> {
    use std::time::{SystemTime, UNIX_EPOCH};

    // 每次请求前都刷新一次 w_webid，避免使用过期的 ID
    let w_webid: String = match state.generate_w_webid().await {
        Ok(id) => {
            logger().debug(format!("Refreshed w_webid: {}", id));
            id
        }
        Err(e) => {
            logger().error(format!("Failed to generate w_webid: {}", e));
            return Err(format!("w_webid 获取失败: {}", e));
        }
    };

    let wts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;
    let pairs = vec![
        ("area_id", area_id.clone()),
        ("page", page.to_string()),
        ("parent_area_id", parent_area_id.clone()),
        ("platform", "web".to_string()),
        ("sort_type", "".to_string()),
        ("vajra_business_key", "".to_string()),
        ("w_webid", w_webid.clone()),
        ("web_location", "444.253".to_string()),
        ("wts", wts.to_string()),
    ];
    let secret = "ea1db124af3c7062474693fa704f4ff8";
    let sign_string = format!(
        "{}{}",
        pairs
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&"),
        secret
    );
    let hash = md5::compute(sign_string.as_bytes());
    let w_rid = format!("{:x}", hash);

    let mut params: Vec<(String, String)> = 
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    params.push(("w_rid".to_string(), w_rid));

    let url = "https://api.live.bilibili.com/xlive/web-interface/v1/second/getList";
    let query_str = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&");
    let full_url = format!("{}?{}", url, query_str);

    logger().debug(format!("Fetch live list: w_webid={}, area_id={}, parent_area_id={}, page={}, wts={}, w_rid={}", w_webid, area_id, parent_area_id, page, wts, &params.iter().find(|(k,_)| k=="w_rid").map(|(_,v)| v.clone()).unwrap_or_default()));
    logger().debug(format!("GET {}", full_url));
    logger().debug(format!(
        "Headers: Referer={}, Cookie={}",
        "https://www.bilibili.com/", "buvid3=i;"
    ));

    // 使用HttpClient的inner访问内部reqwest::Client
    let resp = client.inner
        .get(url)
        .header("Referer", "https://www.bilibili.com/")
        .header("Cookie", "buvid3=i;")
        .query(&params)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("API status: {}", resp.status()));
    }
    let text = resp
        .text()
        .await
        .map_err(|e| format!("Read text failed: {}", e))?;
    
    let json: Value = serde_json::from_str(&text)
        .map_err(|e| format!("Parse JSON failed: {}", e))?;
    
    // 检查API响应是否成功
    if json["code"].as_i64().unwrap_or(1) != 0 {
        return Err(format!("API请求失败: {}", json["message"].as_str().unwrap_or("未知错误")));
    }
    
    // 获取数据部分
    let data = json["data"].clone();
    let _live_items = data["list"].as_array().unwrap_or(&Vec::new());
    
    // 构造LiveList对象
    let live_list = LiveList {
        items: Vec::new(), // 这里将在lib.rs中解析，所以返回空列表
        total: data["total"].as_u64(),
        page: Some(page),
        page_size: Some(30),
        has_more: data["has_more"].as_bool().unwrap_or(false),
        other: Some(data.clone()),
        raw: Some(json.clone()),
    };
    
    Ok((live_list, Some(json)))  
}
