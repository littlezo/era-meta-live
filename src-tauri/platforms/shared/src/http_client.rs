use reqwest::header::{HeaderMap as ReqwestHeaderMap, HeaderName, HeaderValue, USER_AGENT};
use reqwest::{cookie::Jar, Client, RequestBuilder, Response};
use std::sync::Arc;
use std::time::Duration;
use log::{debug, error, info, warn};

pub const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.0.0 Safari/537.36";
const DEFAULT_TIMEOUT_SECONDS: u64 = 20;
const FOLLOW_POOL_MAX_IDLE_PER_HOST: usize = 2;
const FOLLOW_POOL_IDLE_TIMEOUT_SECONDS: u64 = 15;

#[derive(Debug, Clone)]
pub struct HttpClient {
    pub inner: Client,
    headers: ReqwestHeaderMap,
}

#[allow(dead_code)]
impl HttpClient {
    pub fn new() -> Result<Self, String> {
        let mut default_headers = ReqwestHeaderMap::new();
        default_headers.insert(
            USER_AGENT,
            HeaderValue::from_str(DEFAULT_USER_AGENT)
                .map_err(|e| format!("Invalid default user agent: {}", e))?,
        );

        let cookie_jar = Arc::new(Jar::default());

        let client_builder = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECONDS))
            .cookie_provider(cookie_jar);

        let inner_client = client_builder
            .build()
            .map_err(|e| format!("Failed to build reqwest client: {}", e))?;

        Ok(HttpClient {
            inner: inner_client,
            headers: default_headers,
        })
    }

    /// 创建一个绕过所有代理的直连HTTP客户端
    /// 这个客户端将忽略系统代理设置，直接连接到目标服务器
    pub fn new_direct_connection() -> Result<Self, String> {
        let mut default_headers = ReqwestHeaderMap::new();
        default_headers.insert(
            USER_AGENT,
            HeaderValue::from_str(DEFAULT_USER_AGENT)
                .map_err(|e| format!("Invalid default user agent: {}", e))?,
        );

        let cookie_jar = Arc::new(Jar::default());

        let client_builder = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECONDS))
            .cookie_provider(cookie_jar)
            .no_proxy(); // 关键：禁用所有代理设置

        let inner_client = client_builder
            .build()
            .map_err(|e| format!("Failed to build direct connection reqwest client: {}", e))?;

        Ok(HttpClient {
            inner: inner_client,
            headers: default_headers,
        })
    }

    /// 直连 + 限制连接池规模，用于关注刷新等低并发任务
    pub fn new_direct_limited(max_idle_per_host: usize) -> Result<Self, String> {
        let mut default_headers = ReqwestHeaderMap::new();
        default_headers.insert(
            USER_AGENT,
            HeaderValue::from_str(DEFAULT_USER_AGENT)
                .map_err(|e| format!("Invalid default user agent: {}", e))?,
        );

        let cookie_jar = Arc::new(Jar::default());

        let client_builder = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECONDS))
            .cookie_provider(cookie_jar)
            .no_proxy()
            .pool_max_idle_per_host(max_idle_per_host)
            .pool_idle_timeout(Duration::from_secs(FOLLOW_POOL_IDLE_TIMEOUT_SECONDS));

        let inner_client = client_builder
            .build()
            .map_err(|e| format!("Failed to build direct connection reqwest client: {}", e))?;

        Ok(HttpClient {
            inner: inner_client,
            headers: default_headers,
        })
    }

    // Method to add or update a header for subsequent requests made with this client instance
    pub fn insert_header(&mut self, name: HeaderName, value: &str) -> Result<(), String> {
        let header_value = HeaderValue::from_str(value)
            .map_err(|e| format!("Failed to create header value for {}: {}", name, e))?;
        self.headers.insert(name.clone(), header_value);
        Ok(())
    }

    async fn send_request(&self, request_builder: RequestBuilder) -> Result<Response, String> {
        debug!("Sending HTTP request...");
        request_builder
            .headers(self.headers.clone())
            .send()
            .await
            .map_err(|e| {
                error!("HTTP request failed: {}", e);
                format!("HTTP request execution failed: {}", e)
            })
    }

    pub async fn get(&self, url: &str) -> Result<Response, String> {
        debug!("GET request: {}", url);
        let response = self.send_request(self.inner.get(url)).await?;
        debug!("GET response received: status={}", response.status());
        Ok(response)
    }

    pub async fn get_text(&self, url: &str) -> Result<String, String> {
        debug!("GET text request: {}", url);
        let response = self.get(url).await?;
        let status = response.status();
        let response_text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body from {}: {}", url, e))?;
        debug!("GET text response: status={}, length={} bytes", status, response_text.len());
        if !status.is_success() {
            error!("GET {} failed with status {}: {}", url, status, response_text);
            return Err(format!(
                "GET {} failed with status {}: {}",
                url, status, response_text
            ));
        }
        Ok(response_text)
    }

    /// 获取JSON响应并返回原始响应文本
    pub async fn get_json_with_raw<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<(T, String), String> {
        debug!("GET JSON with raw request: {}", url);
        let response = self.get(url).await?;
        let status = response.status();
        
        // 获取响应文本
        let response_text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body from {}: {}", url, e))?;
        debug!("GET JSON with raw response: status={}, length={} bytes", status, response_text.len());
        
        if !status.is_success() {
            error!("GET JSON {} failed with status {}: {}", url, status, response_text);
            return Err(format!(
                "GET JSON {} failed with status {}: {}",
                url, status, response_text
            ));
        }
        
        // 检查响应文本是否为空
        if response_text.trim().is_empty() {
            error!("GET JSON {} returned empty response body", url);
            return Err(format!("GET JSON {} returned empty response body", url));
        }
        
        let json_response = serde_json::from_str(&response_text)
            .map_err(|e| {
                error!("Failed to parse JSON response from `{}`: {}", url, e);
                error!("Response content: `{}`", response_text);
                format!("Failed to parse JSON response from `{}`: {}", url, e)
            })?;
        Ok((json_response, response_text))
    }
    
    pub async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T, String> {
        self.get_json_with_raw(url).await.map(|(json, _)| json)
    }

    pub async fn post_form(&self, url: &str, form_data: &str) -> Result<Response, String> {
        debug!("POST form request: {}, data={}", url, form_data);
        let response = self
            .send_request(
                self.inner
                    .post(url)
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(form_data.to_string()),
            )
            .await?;
        debug!("POST form response received: status={}", response.status());
        Ok(response)
    }

    /// POST表单数据并获取JSON响应，同时返回原始响应文本
    pub async fn post_form_json_with_raw<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        form_data: &str,
    ) -> Result<(T, String), String> {
        debug!("POST form JSON with raw request: {}, data={}", url, form_data);
        let response = self.post_form(url, form_data).await?;
        let status = response.status();
        
        // 获取响应文本
        let response_text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body from {}: {}", url, e))?;
        debug!("POST form JSON with raw response: status={}, length={} bytes", status, response_text.len());
        
        if !status.is_success() {
            error!("POST FORM {} failed with status {}: {}", url, status, response_text);
            return Err(format!(
                "POST FORM {} failed with status {}: {}",
                url, status, response_text
            ));
        }
        
        // 检查响应文本是否为空
        if response_text.trim().is_empty() {
            error!("POST form JSON {} returned empty response body", url);
            return Err(format!("POST form JSON {} returned empty response body", url));
        }
        
        let json_response = serde_json::from_str(&response_text)
            .map_err(|e| {
                error!("Failed to parse JSON response from `{}`: {}", url, e);
                error!("Response content: `{}`", response_text);
                format!("Failed to parse JSON response from `{}`: {}", url, e)
            })?;
        Ok((json_response, response_text))
    }
    
    pub async fn post_form_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        form_data: &str,
    ) -> Result<T, String> {
        self.post_form_json_with_raw(url, form_data).await.map(|(json, _)| json)
    }

    /// GET JSON响应，带自定义headers并返回原始响应文本
    pub async fn get_json_with_headers_with_raw<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        headers: Option<ReqwestHeaderMap>,
    ) -> Result<(T, String), String> {
        debug!("GET JSON with headers with raw request: {}", url);
        if let Some(h) = &headers {
            debug!("Additional headers: {:?}", h);
        }
        
        let mut request_builder = self.inner.get(url);

        if let Some(additional_headers) = headers {
            request_builder = request_builder.headers(additional_headers);
        }

        let response = self.send_request(request_builder).await?;
        let status = response.status();
        debug!("GET JSON with headers with raw response: status={}", status);
        
        // 获取响应文本
        let response_text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body from {}: {}", url, e))?;
        debug!("GET JSON with headers with raw response content: status={}, length={} bytes", status, response_text.len());
        
        if !status.is_success() {
            error!("GET JSON {} failed with status {}: {}", url, status, response_text);
            return Err(format!(
                "GET JSON {} failed with status {}: {}",
                url, status, response_text
            ));
        }
        
        // 检查响应文本是否为空
        if response_text.trim().is_empty() {
            error!("GET JSON {} returned empty response body", url);
            return Err(format!("GET JSON {} returned empty response body", url));
        }
        
        let json_response = serde_json::from_str(&response_text)
            .map_err(|e| {
                error!("Failed to parse JSON response from `{}`: {}", url, e);
                error!("Response content: `{}`", response_text);
                format!("Failed to parse JSON response from `{}`: {}", url, e)
            })?;
        Ok((json_response, response_text))
    }
    
    pub async fn get_json_with_headers<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        headers: Option<ReqwestHeaderMap>,
    ) -> Result<T, String> {
        self.get_json_with_headers_with_raw(url, headers).await.map(|(json, _)| json)
    }

    pub async fn get_with_cookies(&self, url: &str) -> Result<Response, String> {
        debug!("GET with cookies request: {}", url);
        let request_builder = self.inner.get(url).headers(self.headers.clone());
        self.send_request(request_builder).await
    }

    /// GET文本响应，带自定义headers并返回原始响应文本（总是返回原始文本，无论成功失败）
    pub async fn get_text_with_headers_with_raw(
        &self,
        url: &str,
        headers: Option<ReqwestHeaderMap>,
    ) -> Result<(String, String), String> {
        debug!("GET text with headers with raw request: {}", url);
        if let Some(h) = &headers {
            debug!("Additional headers: {:?}", h);
        }
        
        let mut request_builder = self.inner.get(url).headers(self.headers.clone());

        if let Some(additional_headers) = headers {
            request_builder = request_builder.headers(additional_headers);
        }

        let response = self.send_request(request_builder).await?;
        let status = response.status();
        debug!("GET text with headers with raw response: status={}", status);
        
        // 获取响应文本，无论成功失败
        let response_text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read text response from {}: {}", url, e))?;
        debug!("GET text with headers with raw response content: status={}, length={} bytes", status, response_text.len());
        
        if !status.is_success() {
            error!("GET {} failed with status {}: {}", url, status, response_text);
            return Err(format!(
                "GET {} failed with status {}: {}",
                url, status, response_text
            ));
        }
        
        Ok((response_text.clone(), response_text))
    }
    
    pub async fn get_text_with_headers(
        &self,
        url: &str,
        headers: Option<ReqwestHeaderMap>,
    ) -> Result<String, String> {
        self.get_text_with_headers_with_raw(url, headers).await.map(|(text, _)| text)
    }

    /// 获取当前设置的headers信息用于调试
    pub fn get_debug_headers(&self) -> String {
        let mut debug_info = String::new();
        for (name, value) in &self.headers {
            if let Ok(value_str) = value.to_str() {
                debug_info.push_str(&format!("  {}: {}\n", name, value_str));
            } else {
                debug_info.push_str(&format!("  {}: [non-UTF8 value]\n", name));
            }
        }
        debug_info
    }

    /// 获取当前cookies信息用于调试
    pub fn get_debug_cookies(&self, _url: &str) -> String {
        // 由于reqwest::Client的cookie_store方法在当前版本中不可用
        // 我们返回一个提示信息，表明cookies是通过headers设置的
        "Cookies are set via headers (check Headers section above for Cookie header)".to_string()
    }
}

/// 专用于关注刷新等低并发任务的 HTTP 客户端包装
#[derive(Debug, Clone)]
pub struct FollowHttpClient(pub HttpClient);

impl FollowHttpClient {
    pub fn new() -> Result<Self, String> {
        Ok(Self(HttpClient::new_direct_limited(
            FOLLOW_POOL_MAX_IDLE_PER_HOST,
        )?))
    }
}
