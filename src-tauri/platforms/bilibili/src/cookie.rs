use serde::Serialize;

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BilibiliCookieResult {
    pub cookie: Option<String>,
    pub has_sessdata: bool,
    pub has_bili_jct: bool,
}

/// 检查Bilibili Cookie是否包含必要的字段
/// 
/// # 参数
/// - `cookie`: Cookie字符串
/// 
/// # 返回
/// - `BilibiliCookieResult`: 包含Cookie有效性检查结果
pub fn check_bilibili_cookie(cookie: Option<String>) -> BilibiliCookieResult {
    let mut has_sessdata = false;
    let mut has_bili_jct = false;
    
    let cookie_str = cookie.as_ref();
    if let Some(cookie_str) = cookie_str {
        let cookie_pairs = cookie_str.split(';');
        for pair in cookie_pairs {
            let pair = pair.trim();
            if pair.is_empty() {
                continue;
            }
            
            let parts: Vec<&str> = pair.splitn(2, '=').collect();
            if parts.len() != 2 {
                continue;
            }
            
            let name = parts[0].trim().to_lowercase();
            let value = parts[1].trim();
            
            if name == "sessdata" && !value.is_empty() {
                has_sessdata = true;
            } else if name == "bili_jct" && !value.is_empty() {
                has_bili_jct = true;
            }
        }
    }
    
    BilibiliCookieResult {
        cookie,
        has_sessdata,
        has_bili_jct,
    }
}
