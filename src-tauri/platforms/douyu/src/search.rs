use md5::Digest; // For hasher
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    redirect::Policy,
    Client,
};
use std::time::{SystemTime, UNIX_EPOCH}; // For timestamp for did // For URL encoding keyword

// Renamed from search_anchor to avoid ambiguity with Tauri command
pub async fn perform_anchor_search(keyword: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut default_headers = HeaderMap::new();
    default_headers.insert(
        "User-Agent",
        HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36"),
    );

    let client = Client::builder()
        .redirect(Policy::limited(10))
        .no_proxy()
        .default_headers(default_headers)
        .build()?;

    let mut hasher = md5::Md5::new();
    hasher.update(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_nanos()
            .to_string(),
    );
    let did = format!("{:x}", hasher.finalize());

    let url = format!(
        "https://www.douyu.com/japi/search/api/searchUser?kw={}&page=1&pageSize=20&filterType=0",
        percent_encode(keyword.as_bytes(), NON_ALPHANUMERIC)
    );

    let text = client
        .get(url)
        .header("Referer", "https://www.douyu.com/search/")
        .header("Cookie", format!("dy_did={}; acf_did={}", did, did))
        .send()
        .await?
        .text()
        .await?;

    Ok(text)
}

// 搜索主播的结构体定义
#[derive(serde::Deserialize, Debug)]
pub struct AnchorSearchResult {
    pub uid: String,
    pub nickname: String,
    pub room_id: String,
    pub room_name: String,
    pub avatar: String,
    pub hot: i32,
}

// 搜索主播的响应结构体
#[derive(serde::Deserialize, Debug)]
struct AnchorSearchResponse {
    error: i32,
    data: Option<AnchorSearchData>,
}

#[derive(serde::Deserialize, Debug)]
struct AnchorSearchData {
    list: Vec<AnchorSearchResult>,
}

// 与LivePlatform trait兼容的搜索主播函数
pub async fn search_anchor(
    keyword: &str,
    page: Option<u32>,
    page_size: Option<u32>
) -> Result<(Vec<AnchorSearchResult>, Option<serde_json::Value>), Box<dyn std::error::Error>> {
    let page = page.unwrap_or(1);
    let page_size = page_size.unwrap_or(20);
    
    let mut default_headers = HeaderMap::new();
    default_headers.insert(
        "User-Agent",
        HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36"),
    );

    let client = Client::builder()
        .redirect(Policy::limited(10))
        .no_proxy()
        .default_headers(default_headers)
        .build()?;

    let mut hasher = md5::Md5::new();
    hasher.update(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_nanos()
            .to_string(),
    );
    let did = format!("{:x}", hasher.finalize());

    let url = format!(
        "https://www.douyu.com/japi/search/api/searchUser?kw={}&page={}&pageSize={}&filterType=0",
        percent_encode(keyword.as_bytes(), NON_ALPHANUMERIC),
        page,
        page_size
    );

    let text = client
        .get(url)
        .header("Referer", "https://www.douyu.com/search/")
        .header("Cookie", format!("dy_did={}; acf_did={}", did, did))
        .send()
        .await?
        .text()
        .await?;
    
    // 解析响应
    let response: AnchorSearchResponse = serde_json::from_str(&text)?;
    
    if response.error != 0 {
        return Err(format!("API error: {}", response.error).into());
    }
    
    let data = response.data.ok_or("No data returned")?;
    
    // 解析原始JSON数据
    let raw_data: serde_json::Value = serde_json::from_str(&text)?;
    
    Ok((data.list, Some(raw_data)))
}
