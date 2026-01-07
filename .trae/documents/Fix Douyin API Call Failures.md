# Fix Douyin API Call Failures

## Problem Analysis
The Douyin API is returning empty responses, causing JSON parsing failures. The logs show:
```
ERROR shared::http_client: cFailed to parse JSON response from `https://live.douyin.com/webcast/room/web/enter/?...`: EOF while parsing a value at line 1 column 0
ERROR shared::http_client: Response content: ``
```

**Root Causes Identified:**
1. **Incorrect URL Parameters**: Using `web_rid` instead of `room_id`
2. **Missing Critical Parameters**: Not including `user_unique_id`
3. **Incorrect Response Structure**: API returns `{"data": {"room": {...}}}` but code looks for `{"data": {"data": [...]}}`
4. **a_bogus Signature Issues**: Generated signature might be incorrect
5. **Empty msToken**: Passing empty token could cause server rejection

## Solution Plan

### 1. Update `web_api.rs` - Fix Core API Logic
- **Simplify URL Structure**: Use basic URL format without complex parameters
- **Use Correct Parameter Names**: Replace `web_rid` with `room_id`
- **Remove a_bogus**: Simplify by removing problematic signature generation
- **Update Response Parsing**: Extract room data from `json["data"]["room"]`
- **Fix Parameter Handling**: Only include necessary parameters

### 2. Update `fetch_room_from_api` Function
```rust
// Current (failing):
let params = vec![("web_rid", web_id), ("msToken", ""), ...];
let query = serde_urlencoded::to_string(&params)?;
let sign = generate_a_bogus(&query, DEFAULT_USER_AGENT);
let api = format!("https://live.douyin.com/webcast/room/web/enter/?{}&a_bogus={}", query, sign);
// Look for: json["data"]["data"][0]

// Fix to:
let base_url = "https://live.douyin.com/webcast/room/web/enter/?aid=6383&app_name=douyin_web&live_id=1&device_platform=web&language=zh-CN&cookie_enabled=true";
let url = format!("{}&room_id={}", base_url, web_id);
// Look for: json["data"]["room"]
```

### 3. Ensure Direct Connection HTTP Client
- Verify HTTP client used is `new_direct_connection()` to bypass proxies
- Update `fetch_room_info` to ensure proper client is used

### 4. Improve Error Handling
- Add detailed logging for request/response cycles
- Handle empty responses gracefully

## Files to Modify
1. `/Volumes/Data/project/era-meta-live/src-tauri/platforms