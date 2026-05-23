use std::time::Duration;

use anyhow::anyhow;

pub async fn parse_google_response(res: reqwest::Response) -> anyhow::Result<serde_json::Value> {
    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    let body: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|_| {
        serde_json::json!({
            "raw": text
        })
    });

    if !status.is_success() {
        let message = body["error_description"]
            .as_str()
            .or_else(|| body["error"]["message"].as_str())
            .or_else(|| body["error"].as_str())
            .unwrap_or("Google API 请求失败");
        return Err(anyhow!("{} ({})", message, status));
    }

    Ok(body)
}

pub async fn send_with_retry<F>(mut build_request: F) -> anyhow::Result<serde_json::Value>
where
    F: FnMut() -> reqwest::RequestBuilder,
{
    let mut last_error = None;
    for attempt in 0..3 {
        match build_request().send().await {
            Ok(resp) => {
                let status = resp.status();
                if status.as_u16() == 429 || status.is_server_error() {
                    last_error = Some(
                        parse_google_response(resp)
                            .await
                            .err()
                            .unwrap_or_else(|| anyhow!("Google API 临时错误 ({})", status)),
                    );
                } else {
                    return parse_google_response(resp).await;
                }
            }
            Err(e) => last_error = Some(e.into()),
        }
        tokio::time::sleep(Duration::from_millis(400 * (attempt + 1) as u64)).await;
    }

    Err(last_error.unwrap_or_else(|| anyhow!("Google API 请求失败")))
}
