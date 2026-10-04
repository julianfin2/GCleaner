use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use tauri::{AppHandle, Emitter};
use url::Url;

use crate::auth::{api_path_segment, google_client, parse_google_response, send_with_retry};
use crate::drive::remove_account_permission;
use crate::models::{
    FileActionStatusPayload, MailCleanupCandidate, MailDriveFile, SharedPermission,
};
use crate::tokens::get_valid_access_token;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
struct DriveLink {
    id: String,
    resource_key: Option<String>,
}

pub async fn scan_mail_cleanup_candidates(
    account: String,
    tokens_dir: PathBuf,
    query: String,
) -> anyhow::Result<Vec<MailCleanupCandidate>> {
    let client = google_client()?;
    let token = get_valid_access_token(&tokens_dir, &account)?;
    let mut candidates = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut url = Url::parse("https://gmail.googleapis.com/gmail/v1/users/me/messages")?;
        url.query_pairs_mut()
            .append_pair("q", &query)
            .append_pair("maxResults", "50");
        if let Some(ref pt) = page_token {
            url.query_pairs_mut().append_pair("pageToken", pt);
        }

        let body =
            send_with_retry(|| client.get(url.clone()).bearer_auth(&token.access_token)).await?;
        let messages = body["messages"].as_array().cloned().unwrap_or_default();

        for message in messages {
            let Some(message_id) = message["id"].as_str() else {
                continue;
            };

            let message = get_gmail_message(&client, message_id, &token.access_token).await?;
            let links = extract_drive_links_from_message(&message);
            if links.is_empty() {
                continue;
            }

            let mut drive_files = Vec::new();
            for link in links {
                drive_files
                    .push(inspect_drive_link(&client, &token.access_token, &account, &link).await);
            }

            let removable_permissions = drive_files.iter().filter(|file| file.removable).count();
            let ignored_files = drive_files.len().saturating_sub(removable_permissions);

            candidates.push(MailCleanupCandidate {
                id: message["id"].as_str().unwrap_or_default().to_string(),
                thread_id: message["threadId"].as_str().unwrap_or_default().to_string(),
                account: account.clone(),
                subject: header_value(&message, "Subject")
                    .unwrap_or_else(|| "(无主题)".to_string()),
                from: header_value(&message, "From").unwrap_or_default(),
                date: header_value(&message, "Date").unwrap_or_default(),
                snippet: message["snippet"].as_str().unwrap_or_default().to_string(),
                drive_links: drive_files,
                removable_permissions,
                ignored_files,
            });
        }

        page_token = body["nextPageToken"].as_str().map(|s| s.to_string());
        if page_token.is_none() {
            break;
        }
    }

    Ok(candidates)
}

pub async fn process_mail_cleanup_candidate(
    client: &reqwest::Client,
    candidate: &MailCleanupCandidate,
    access_token: &str,
    permanently_delete: bool,
) -> anyhow::Result<(usize, usize)> {
    let mut removed_permissions = 0;
    let mut failed_permissions = 0;

    for file in &candidate.drive_links {
        if !file.removable {
            continue;
        }
        let Some(permission_id) = file.permission_id.as_deref() else {
            continue;
        };

        match remove_account_permission(
            client,
            &file.id,
            permission_id,
            access_token,
            &candidate.account,
        )
        .await
        {
            Ok(()) => removed_permissions += 1,
            Err(_) => failed_permissions += 1,
        }
    }

    delete_mail(client, &candidate.id, access_token, permanently_delete).await?;
    Ok((removed_permissions, failed_permissions))
}

pub fn emit_mail_cleanup_status(
    app_handle: &AppHandle,
    candidate: &MailCleanupCandidate,
    status: &str,
) {
    app_handle
        .emit(
            "mail-cleanup-status",
            FileActionStatusPayload {
                id: candidate.id.clone(),
                account: candidate.account.clone(),
                status: status.to_string(),
            },
        )
        .ok();
}

async fn get_gmail_message(
    client: &reqwest::Client,
    message_id: &str,
    access_token: &str,
) -> anyhow::Result<serde_json::Value> {
    let url = format!(
        "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}",
        api_path_segment(message_id)?
    );
    let body = send_with_retry(|| {
        client
            .get(&url)
            .bearer_auth(access_token)
            .query(&[("format", "full")])
    })
    .await?;
    Ok(body)
}

async fn inspect_drive_link(
    client: &reqwest::Client,
    access_token: &str,
    account: &str,
    link: &DriveLink,
) -> MailDriveFile {
    match get_drive_file(client, access_token, link).await {
        Ok(file) => build_drive_file_result(account, link, &file),
        Err(e) => MailDriveFile {
            id: link.id.clone(),
            name: link.id.clone(),
            mime_type: String::new(),
            permission_id: None,
            owner: String::new(),
            reason: format!("无法访问或权限已失效：{}", e),
            removable: false,
            resource_key: link.resource_key.clone(),
        },
    }
}

async fn get_drive_file(
    client: &reqwest::Client,
    access_token: &str,
    link: &DriveLink,
) -> anyhow::Result<serde_json::Value> {
    let url = format!(
        "https://www.googleapis.com/drive/v3/files/{}",
        api_path_segment(&link.id)?
    );
    let mut request = client
        .get(&url)
        .bearer_auth(access_token)
        .query(&[
            (
                "fields",
                "id,name,mimeType,trashed,owners(displayName,emailAddress),permissions(id,type,role,emailAddress)",
            ),
            ("supportsAllDrives", "true"),
        ]);
    if let Some(resource_key) = link.resource_key.as_deref() {
        request = request.query(&[("resourceKey", resource_key)]);
    }
    let body =
        parse_google_response(request.send().await.map_err(reqwest::Error::without_url)?).await?;
    Ok(body)
}

fn build_drive_file_result(
    account: &str,
    link: &DriveLink,
    file: &serde_json::Value,
) -> MailDriveFile {
    let name = file["name"].as_str().unwrap_or(&link.id).to_string();
    let mime_type = file["mimeType"].as_str().unwrap_or_default().to_string();
    let owner = file["owners"]
        .as_array()
        .and_then(|owners| owners.first())
        .and_then(|owner| {
            owner["emailAddress"]
                .as_str()
                .or_else(|| owner["displayName"].as_str())
        })
        .unwrap_or("未知所有者")
        .to_string();

    if file["trashed"].as_bool().unwrap_or(false) {
        return mail_drive_file(link, name, mime_type, owner, None, "文件已在回收站", false);
    }

    if owner.eq_ignore_ascii_case(account) {
        return mail_drive_file(
            link,
            name,
            mime_type,
            owner,
            None,
            "当前账号是所有者，不移除权限",
            false,
        );
    }

    let permissions: Vec<SharedPermission> = if file["permissions"].is_array() {
        serde_json::from_value(file["permissions"].clone()).unwrap_or_default()
    } else {
        Vec::new()
    };

    if let Some(permission) = permissions.iter().find(|permission| {
        permission.p_type == "user"
            && permission.role != "owner"
            && permission
                .email_address
                .as_deref()
                .map(|email| email.eq_ignore_ascii_case(account))
                .unwrap_or(false)
    }) {
        if !permission.id.is_empty() {
            return mail_drive_file(
                link,
                name,
                mime_type,
                owner,
                Some(permission.id.clone()),
                "发现当前账号的直接权限，可移除",
                true,
            );
        }
    }

    mail_drive_file(
        link,
        name,
        mime_type,
        owner,
        None,
        "未发现当前账号的直接 user 权限，可能来自公开链接、群组、域或继承权限",
        false,
    )
}

fn mail_drive_file(
    link: &DriveLink,
    name: String,
    mime_type: String,
    owner: String,
    permission_id: Option<String>,
    reason: &str,
    removable: bool,
) -> MailDriveFile {
    MailDriveFile {
        id: link.id.clone(),
        name,
        mime_type,
        permission_id,
        owner,
        reason: reason.to_string(),
        removable,
        resource_key: link.resource_key.clone(),
    }
}

fn extract_drive_links_from_message(message: &serde_json::Value) -> Vec<DriveLink> {
    let mut text = String::new();
    collect_mime_text(&message["payload"], &mut text);
    text.push(' ');
    text.push_str(message["snippet"].as_str().unwrap_or_default());

    let mut seen = HashSet::new();
    let mut links = Vec::new();
    for token in split_possible_urls(&text) {
        if let Some(link) = parse_drive_link(&token) {
            if seen.insert((link.id.clone(), link.resource_key.clone())) {
                links.push(link);
            }
        }
    }
    links
}

fn collect_mime_text(part: &serde_json::Value, out: &mut String) {
    if let Some(data) = part["body"]["data"].as_str() {
        if let Ok(decoded) = decode_base64_url(data) {
            out.push(' ');
            out.push_str(&decoded);
        }
    }

    if let Some(parts) = part["parts"].as_array() {
        for child in parts {
            collect_mime_text(child, out);
        }
    }
}

fn split_possible_urls(text: &str) -> Vec<String> {
    text.replace("&amp;", "&")
        .replace("\\u003d", "=")
        .replace("\\u0026", "&")
        .split(|c: char| {
            c.is_whitespace() || ['"', '\'', '<', '>', '(', ')', '[', ']'].contains(&c)
        })
        .filter(|part| part.contains("google.com/") || part.contains("googleusercontent.com/"))
        .map(|part| {
            part.trim_matches(|c| [',', '.', ';'].contains(&c))
                .to_string()
        })
        .collect()
}

fn parse_drive_link(raw: &str) -> Option<DriveLink> {
    parse_drive_link_at_depth(raw, 0)
}

fn parse_drive_link_at_depth(raw: &str, depth: usize) -> Option<DriveLink> {
    if depth > 5 || raw.len() > 8192 {
        return None;
    }
    let url = Url::parse(raw).ok().or_else(|| {
        let decoded = urlencoding::decode(raw).ok()?;
        Url::parse(&decoded).ok()
    })?;
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
        return None;
    }
    let host = url.host_str()?;
    let path_segments: Vec<_> = url.path_segments()?.collect();
    let query: HashMap<_, _> = url.query_pairs().into_owned().collect();
    let resource_key = query.get("resourcekey").cloned();

    if (host == "www.google.com" || host == "google.com") && path_segments.first() == Some(&"url") {
        if let Some(target) = query.get("q").or_else(|| query.get("url")) {
            return parse_drive_link_at_depth(target, depth + 1);
        }
    }

    let id = if host == "drive.google.com" {
        if let Some(id) = query.get("id") {
            id.as_str()
        } else if path_segments.len() >= 3
            && ((path_segments[0] == "file" && path_segments[1] == "d")
                || (path_segments[0] == "drive" && path_segments[1] == "folders"))
        {
            path_segments[2]
        } else {
            return None;
        }
    } else if host == "docs.google.com" && path_segments.len() >= 3 && path_segments[1] == "d" {
        path_segments[2]
    } else {
        return None;
    };
    if id.is_empty()
        || id.len() > 256
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
    {
        return None;
    }
    Some(DriveLink {
        id: id.to_string(),
        resource_key,
    })
}

fn header_value(message: &serde_json::Value, name: &str) -> Option<String> {
    message["payload"]["headers"]
        .as_array()?
        .iter()
        .find(|header| {
            header["name"]
                .as_str()
                .is_some_and(|value| value.eq_ignore_ascii_case(name))
        })
        .and_then(|header| header["value"].as_str())
        .map(|value| value.to_string())
}

fn decode_base64_url(data: &str) -> anyhow::Result<String> {
    let mut bytes = Vec::new();
    let mut buffer: u32 = 0;
    let mut bits = 0;

    for c in data.chars().filter(|c| *c != '=') {
        let value = match c {
            'A'..='Z' => c as u32 - 'A' as u32,
            'a'..='z' => c as u32 - 'a' as u32 + 26,
            '0'..='9' => c as u32 - '0' as u32 + 52,
            '+' | '-' => 62,
            '/' | '_' => 63,
            _ => continue,
        };
        buffer = (buffer << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            bytes.push(((buffer >> bits) & 0xff) as u8);
        }
    }

    String::from_utf8(bytes).map_err(|e| anyhow!("邮件正文编码解析失败：{}", e))
}

async fn delete_mail(
    client: &reqwest::Client,
    message_id: &str,
    access_token: &str,
    permanently_delete: bool,
) -> anyhow::Result<()> {
    if permanently_delete {
        let url = format!(
            "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}",
            api_path_segment(message_id)?
        );
        delete_with_retry(client, &url, access_token, "Gmail 永久删除失败").await
    } else {
        let url = format!(
            "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}/trash",
            api_path_segment(message_id)?
        );
        move_mail_to_trash(client, &url, access_token).await
    }
}

async fn move_mail_to_trash(
    client: &reqwest::Client,
    url: &str,
    access_token: &str,
) -> anyhow::Result<()> {
    let mut last_error = None;
    for attempt in 0..3 {
        match client
            .post(url)
            .bearer_auth(access_token)
            .header(reqwest::header::CONTENT_LENGTH, "0")
            .body("")
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    return Ok(());
                }
                last_error = Some(
                    parse_google_response(resp)
                        .await
                        .err()
                        .unwrap_or_else(|| anyhow!("Gmail 移入回收站失败 ({})", status)),
                );
                if status.as_u16() != 429 && !status.is_server_error() {
                    break;
                }
            }
            Err(e) => last_error = Some(e.without_url().into()),
        }
        tokio::time::sleep(Duration::from_millis(400 * (attempt + 1) as u64)).await;
    }

    Err(last_error.unwrap_or_else(|| anyhow!("Gmail 移入回收站失败")))
}

async fn delete_with_retry(
    client: &reqwest::Client,
    url: &str,
    access_token: &str,
    label: &str,
) -> anyhow::Result<()> {
    let mut last_error = None;
    for attempt in 0..3 {
        match client.delete(url).bearer_auth(access_token).send().await {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    return Ok(());
                }
                last_error = Some(
                    parse_google_response(resp)
                        .await
                        .err()
                        .unwrap_or_else(|| anyhow!("{} ({})", label, status)),
                );
                if status.as_u16() != 429 && !status.is_server_error() {
                    break;
                }
            }
            Err(e) => last_error = Some(e.without_url().into()),
        }
        tokio::time::sleep(Duration::from_millis(400 * (attempt + 1) as u64)).await;
    }

    Err(last_error.unwrap_or_else(|| anyhow!("{}", label)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_untrusted_drive_links() {
        for link in [
            "https://evildocs.google.com/document/d/abc/edit",
            "https://docs.google.com.evil.example/document/d/abc/edit",
            "https://drive.google.com/open?id=a%2Fb",
            "https://drive.google.com/open?id=a%3Ffields%3Dpermissions",
            "https://drive.google.com/open?id=",
            "https://drive.google.com/file/d/a%2Fb/view",
            "https://user@drive.google.com/file/d/abc/view",
        ] {
            assert!(parse_drive_link(link).is_none(), "{link}");
        }
    }

    #[test]
    fn accept_official_drive_links_and_limit_redirect_depth() {
        for link in [
            "https://docs.google.com/document/d/abc-_123/edit",
            "https://drive.google.com/file/d/abc-_123/view",
            "https://drive.google.com/drive/folders/abc-_123",
            "https://drive.google.com/open?id=abc-_123",
        ] {
            assert_eq!(parse_drive_link(link).unwrap().id, "abc-_123");
        }
        let mut link = "https://drive.google.com/open?id=abc-_123&resourcekey=key".to_string();
        link = format!(
            "https://www.google.com/url?q={}",
            urlencoding::encode(&link)
        );
        let parsed = parse_drive_link(&link).unwrap();
        assert_eq!(parsed.id, "abc-_123");
        assert_eq!(parsed.resource_key.as_deref(), Some("key"));
        for _ in 0..6 {
            link = format!(
                "https://www.google.com/url?q={}",
                urlencoding::encode(&link)
            );
        }
        assert!(parse_drive_link(&link).is_none());
    }
}
