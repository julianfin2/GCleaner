use std::time::Duration;

use anyhow::anyhow;

pub fn google_client() -> anyhow::Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
        .build()?)
}

pub fn api_path_segment(value: &str) -> anyhow::Result<String> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'='))
    {
        return Err(anyhow!("Google API 资源 ID 无效"));
    }
    Ok(urlencoding::encode(value).into_owned())
}

pub fn api_resource_name(value: &str, collection: &str) -> anyhow::Result<String> {
    let id = value
        .strip_prefix(&format!("{}/", collection))
        .ok_or_else(|| anyhow!("Google API 资源名称无效"))?;
    Ok(format!("{}/{}", collection, api_path_segment(id)?))
}

pub async fn parse_google_response(res: reqwest::Response) -> anyhow::Result<serde_json::Value> {
    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    let body: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|_| {
        serde_json::json!({
            "raw": text
        })
    });

    if !status.is_success() {
        // Error bodies may echo request credentials or other private values.
        return Err(anyhow!("Google API 请求失败 ({})", status));
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
            Err(e) => last_error = Some(e.without_url().into()),
        }
        tokio::time::sleep(Duration::from_millis(400 * (attempt + 1) as u64)).await;
    }

    Err(last_error.unwrap_or_else(|| anyhow!("Google API 请求失败")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_api_path_injection() {
        for value in [
            "",
            ".",
            "..",
            "../permissions",
            "id?deleteContacts=true",
            "a/b",
            "a%2fb",
            "id#x",
            "id\r\n",
        ] {
            assert!(api_path_segment(value).is_err(), "{value:?}");
        }
        assert_eq!(api_path_segment("MDEx-_=").unwrap(), "MDEx-_%3D");
    }

    #[test]
    fn enforce_resource_collection() {
        assert_eq!(
            api_resource_name("contactGroups/abc", "contactGroups").unwrap(),
            "contactGroups/abc"
        );
        for value in [
            "people/abc",
            "/contactGroups/abc",
            "contactGroups/../people/abc",
            "contactGroups/abc?deleteContacts=true",
        ] {
            assert!(api_resource_name(value, "contactGroups").is_err());
        }
    }
}
