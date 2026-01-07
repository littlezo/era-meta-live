use serde::Deserialize;


use shared::interface::{Category, PlatformError, PlatformType};
use shared::logger::Logger;
use shared::http_client::HttpClient;

// 创建静态日志记录器
static LOGGER: std::sync::OnceLock<Logger> = std::sync::OnceLock::new();

fn logger() -> &'static Logger {
    LOGGER.get_or_init(|| {
        Logger::new(Some(PlatformType::Douyu), "douyu::categories")
    })
}

// Structs for deserializing Douyu's m.douyu.com/api/cate/list response
// Based on provided list.json

#[derive(Deserialize, Debug, Clone)]
struct JsonCate1Item {
    #[serde(rename = "cate1Id")]
    id: i32,
    #[serde(rename = "cate1Name")]
    name: String,
}

#[derive(Deserialize, Debug, Clone)]
struct JsonCate2Item {
    #[serde(rename = "cate1Id")]
    parent_id: i32, // To link with JsonCate1Item.id
    #[serde(rename = "cate2Id")]
    id: i32,
    #[serde(rename = "cate2Name")]
    name: String,
    icon: String, // Assuming this is the desired icon URL
}

#[derive(Deserialize, Debug)]
struct DouyuCategoryDataRaw {
    #[serde(rename = "cate1Info")]
    cate1_info: Option<Vec<JsonCate1Item>>,
    #[serde(rename = "cate2Info")]
    cate2_info: Option<Vec<JsonCate2Item>>,
}

#[derive(Deserialize, Debug)]
struct DouyuCategoryApiResponse {
    #[serde(alias = "code")]
    error: i32, // Douyu uses 'code' for error status
    msg: Option<String>,
    data: Option<DouyuCategoryDataRaw>,
}

// Helper structs for the transformation (intermediate step before common types)
#[derive(Debug, Clone)]
struct RawFrontendCate2Item {
    id: String,
    name: String,
    icon: String,
}

#[derive(Debug, Clone)]
struct RawFrontendCate1Item {
    id: String,
    name: String,
    cate2_list: Vec<RawFrontendCate2Item>,
}

pub async fn fetch_categories(client: &HttpClient) -> Result<Vec<Category>, PlatformError> {
    logger().info("fetch_categories called");
    match fetch_categories_douyu_raw(client).await {
        Ok(raw_data) => {
            let mut categories: Vec<Category> = Vec::new();
            
            // 转换为统一的Category类型
            for raw_c1 in raw_data {
                // 添加一级分类
                categories.push(Category {
                    category_id: raw_c1.id.clone(),
                    name: raw_c1.name.clone(),
                    parent_id: None,
                    icon_url: None,
                    order: None,
                    other: Some(serde_json::json!({"id": raw_c1.id.clone(), "name": raw_c1.name.clone()})),
                    raw: None,
                });
                
                // 添加二级分类
                for raw_c2 in raw_c1.cate2_list {
                    // 先克隆需要在多个地方使用的字段
                    let c2_id = raw_c2.id.clone();
                    let c2_name = raw_c2.name.clone();
                    let c2_icon = raw_c2.icon.clone();
                    let c1_id = raw_c1.id.clone();
                    
                    categories.push(Category {
                        category_id: c2_id.clone(),
                        name: c2_name.clone(),
                        parent_id: Some(c1_id.clone()),
                        icon_url: Some(c2_icon.clone()),
                        order: None,
                        other: Some(serde_json::json!({"id": c2_id, "name": c2_name, "icon": c2_icon, "parent_id": c1_id})),
                        raw: None,
                    });
                }
            }
            
            Ok(categories)
        }
        Err(e) => {
            logger().error(format!("Error in fetch_categories: {}", e));
            Err(PlatformError::Api(e))
        }
    }
}

// Internal function to fetch and parse to the old frontend-specific structure
async fn fetch_categories_douyu_raw(client: &HttpClient) -> Result<Vec<RawFrontendCate1Item>, String> {
    let url = "https://m.douyu.com/api/cate/list";

    // Create headers for this request
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static("Mozilla/5.0 (iPhone; CPU iPhone OS 13_2_3 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/13.0.3 Mobile/15E148 Safari/604.1")
    );

    let body_text = client.get_text_with_headers(url, Some(headers)).await?;
    
    match serde_json::from_str::<DouyuCategoryApiResponse>(&body_text) {
        Ok(parsed_response) => {
            if parsed_response.error == 0 {
                if let Some(douyu_data) = parsed_response.data {
                    let mut cate1_list: Vec<RawFrontendCate1Item> = Vec::new();
                    let all_raw_c2_items = douyu_data.cate2_info.unwrap_or_default();

                    if let Some(raw_cate1_list) = douyu_data.cate1_info {
                        for raw_c1_item in raw_cate1_list {
                            let mut c1_specific_cate2_list: Vec<RawFrontendCate2Item> = 
                                Vec::new();
                            for raw_c2_item in &all_raw_c2_items {
                                if raw_c2_item.parent_id == raw_c1_item.id {
                                    c1_specific_cate2_list.push(RawFrontendCate2Item {
                                        id: raw_c2_item.id.to_string(),
                                        name: raw_c2_item.name.clone(),
                                        icon: raw_c2_item.icon.clone(),
                                    });
                                }
                            }
                            cate1_list.push(RawFrontendCate1Item {
                                id: raw_c1_item.id.to_string(),
                                name: raw_c1_item.name.clone(),
                                cate2_list: c1_specific_cate2_list,
                            });
                        }
                    }
                    Ok(cate1_list)
                } else {
                    Err(format!(
                        "Data field is missing. Code: {}, Msg: {:?}",
                        parsed_response.error,
                        parsed_response.msg
                    ))
                }
            } else {
                Err(format!(
                    "Category API error. Code: {}, Msg: {:?}",
                    parsed_response.error,
                    parsed_response.msg
                ))
            }
        }
        Err(e) => Err(format!(
            "Failed to parse category JSON: {}, Body: {}",
            e,
            body_text
        )),
    }
}

// Structs for three category
#[derive(Deserialize, Debug, Clone)]
struct DouyuThreeCateItemRaw {
    #[serde(alias = "tagId", alias = "cateId")] // Common ID fields
    id: String, // Assuming ID is string, could be number
    #[serde(alias = "tagName", alias = "cateName")] // Common name fields
    name: String,
    #[serde(alias = "icon", alias = "pic", alias = "iconUrl")] // Common icon fields
    icon_url: Option<String>,
}

#[derive(Deserialize, Debug)]
struct DouyuThreeCateApiResponse {
    error: i32,
    msg: Option<String>,
    // data field now directly expects a list of items, or null/missing
    data: Option<Vec<DouyuThreeCateItemRaw>>,
}

pub async fn fetch_three_cate(client: &HttpClient, tag_id: i32) -> Result<Vec<(String, String, Option<String>)>, String> {
    let tag_id_str = tag_id.to_string();
    let url = format!(
        "https://capi.douyucdn.cn/api/v1/getThreeCate?tag_id={}&client_sys=android",
        tag_id_str
    );
    logger().info(format!("fetch_three_cate called for tag_id: {}", tag_id_str));

    match client.get_text(&url).await {
        Ok(body_text) => {
            match serde_json::from_str::<DouyuThreeCateApiResponse>(&body_text) {
                Ok(parsed_response) => {
                    if parsed_response.error == 0 {
                        // Directly use parsed_response.data which is Option<Vec<DouyuThreeCateItemRaw>>
                        if let Some(items_list) = parsed_response.data {
                            if !items_list.is_empty() {
                                let result = items_list
                                    .into_iter()
                                    .map(|item| (item.id, item.name, item.icon_url))
                                    .collect();
                                Ok(result)
                            } else {
                                logger().info(format!("fetch_three_cate for {} returned success but empty list (data array was empty).", tag_id_str));
                                Ok(Vec::new()) // Return empty vec if list is empty but no API error
                            }
                        } else {
                            // This case means the "data" field was null or missing, but error code was 0.
                            logger().info(format!("fetch_three_cate for {} returned success but data field was null or missing.", tag_id_str));
                            Ok(Vec::new())
                        }
                    } else {
                        Err(format!(
                            "ThreeCate API error for tag_id {}. Code: {}, Msg: {:?}",
                            tag_id_str, parsed_response.error, parsed_response.msg
                        ))
                    }
                }
                Err(e) => Err(format!(
                    "Failed to parse three_cate JSON for tag_id {}: {}, Body: {}",
                    tag_id_str, e, body_text
                )),
            }
        }
        Err(e) => {
            logger().error(format!("fetch_three_cate request failed for tag_id {}: {}", tag_id_str, e));
            Err(format!("Request failed for tag_id {}: {}", tag_id_str, e))
        }
    }
}