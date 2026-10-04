use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use tauri::{AppHandle, Emitter};
use url::Url;

use crate::auth::{api_path_segment, google_client, parse_google_response, send_with_retry};
use crate::models::{DriveFile, FileActionStatusPayload, SharedDriveFile, SharedPermission};
use crate::tokens::get_valid_access_token;

pub async fn scan_owned_top_level(
    account: String,
    tokens_dir: PathBuf,
) -> anyhow::Result<Vec<DriveFile>> {
    let client = google_client()?;
    let token = get_valid_access_token(&tokens_dir, &account)?;
    let mut files_to_delete = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut url = Url::parse("https://www.googleapis.com/drive/v3/files")?;
        url.query_pairs_mut()
            .append_pair(
                "q",
                "'me' in owners and 'root' in parents and trashed=false",
            )
            .append_pair("spaces", "drive")
            .append_pair("pageSize", "100")
            .append_pair(
                "fields",
                "nextPageToken,files(id,name,mimeType,webViewLink)",
            );
        if let Some(ref pt) = page_token {
            url.query_pairs_mut().append_pair("pageToken", pt);
        }

        let body =
            send_with_retry(|| client.get(url.clone()).bearer_auth(&token.access_token)).await?;
        let files = body["files"]
            .as_array()
            .ok_or_else(|| anyhow!("账号 {} 的 Drive 响应中没有 files", account))?;

        for f in files {
            files_to_delete.push(DriveFile {
                id: f["id"].as_str().unwrap_or_default().to_string(),
                name: f["name"].as_str().unwrap_or_default().to_string(),
                mime_type: f["mimeType"].as_str().unwrap_or_default().to_string(),
                web_view_link: f["webViewLink"].as_str().map(|s| s.to_string()),
                account: account.clone(),
            });
        }

        page_token = body["nextPageToken"].as_str().map(|s| s.to_string());
        if page_token.is_none() {
            break;
        }
    }

    Ok(files_to_delete)
}

pub async fn scan_shared_account(
    account: String,
    tokens_dir: PathBuf,
) -> anyhow::Result<Vec<SharedDriveFile>> {
    let client = google_client()?;
    let token = get_valid_access_token(&tokens_dir, &account)?;
    let mut shared_files = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut url = Url::parse("https://www.googleapis.com/drive/v3/files")?;
        url.query_pairs_mut()
            .append_pair(
                "q",
                "sharedWithMe = true and not 'me' in owners and trashed=false",
            )
            .append_pair("spaces", "drive")
            .append_pair("pageSize", "100")
            .append_pair(
                "fields",
                "nextPageToken,files(id,name,mimeType,webViewLink,owners(displayName,emailAddress),permissions(id,type,role,emailAddress))",
            );
        if let Some(ref pt) = page_token {
            url.query_pairs_mut().append_pair("pageToken", pt);
        }

        let body =
            send_with_retry(|| client.get(url.clone()).bearer_auth(&token.access_token)).await?;
        let files = body["files"]
            .as_array()
            .ok_or_else(|| anyhow!("账号 {} 的 Drive 响应中没有 files", account))?;

        for f in files {
            let permissions: Vec<SharedPermission> = if f["permissions"].is_array() {
                serde_json::from_value(f["permissions"].clone())?
            } else {
                Vec::new()
            };

            let Some(permission) = permissions.iter().find(|permission| {
                permission.p_type == "user"
                    && permission.role != "owner"
                    && permission
                        .email_address
                        .as_deref()
                        .map(|email| email.eq_ignore_ascii_case(&account))
                        .unwrap_or(false)
            }) else {
                continue;
            };

            if permission.id.is_empty() {
                continue;
            }

            let owner = f["owners"]
                .as_array()
                .and_then(|owners| owners.first())
                .and_then(|owner| {
                    owner["emailAddress"]
                        .as_str()
                        .or_else(|| owner["displayName"].as_str())
                })
                .unwrap_or("未知所有者")
                .to_string();

            shared_files.push(SharedDriveFile {
                id: f["id"].as_str().unwrap_or_default().to_string(),
                name: f["name"].as_str().unwrap_or_default().to_string(),
                mime_type: f["mimeType"].as_str().unwrap_or_default().to_string(),
                web_view_link: f["webViewLink"].as_str().map(|s| s.to_string()),
                account: account.clone(),
                permission_id: permission.id.clone(),
                owner,
            });
        }

        page_token = body["nextPageToken"].as_str().map(|s| s.to_string());
        if page_token.is_none() {
            break;
        }
    }

    Ok(shared_files)
}

pub async fn delete_drive_file(
    client: &reqwest::Client,
    file_id: &str,
    access_token: &str,
    permanently_delete: bool,
) -> anyhow::Result<()> {
    if permanently_delete {
        permanently_delete_file_with_retry(client, file_id, access_token).await
    } else {
        trash_file_with_retry(client, file_id, access_token).await
    }
}

pub async fn remove_shared_permission(
    client: &reqwest::Client,
    file: &SharedDriveFile,
    access_token: &str,
) -> anyhow::Result<()> {
    remove_account_permission(
        client,
        &file.id,
        &file.permission_id,
        access_token,
        &file.account,
    )
    .await
}

pub async fn remove_account_permission(
    client: &reqwest::Client,
    file_id: &str,
    permission_id: &str,
    access_token: &str,
    account: &str,
) -> anyhow::Result<()> {
    let url = format!(
        "https://www.googleapis.com/drive/v3/files/{}/permissions/{}",
        api_path_segment(file_id)?,
        api_path_segment(permission_id)?
    );
    let body = send_with_retry(|| {
        client
            .get(&url)
            .bearer_auth(access_token)
            .query(&[("fields", "id,type,role,emailAddress")])
    })
    .await?;
    let permission: SharedPermission = serde_json::from_value(body)?;
    if !is_removable_account_permission(&permission, account, permission_id) {
        return Err(anyhow!("只能移除当前账号的直接非所有者权限"));
    }
    send_delete_with_retry(client, &url, access_token).await
}

fn is_removable_account_permission(permission: &SharedPermission, account: &str, id: &str) -> bool {
    permission.id == id
        && permission.p_type == "user"
        && !permission.role.is_empty()
        && permission.role != "owner"
        && permission
            .email_address
            .as_deref()
            .is_some_and(|email| email.eq_ignore_ascii_case(account))
}

pub fn emit_file_status(app_handle: &AppHandle, file: &DriveFile, status: &str) {
    app_handle
        .emit(
            "file-action-status",
            FileActionStatusPayload {
                id: file.id.clone(),
                account: file.account.clone(),
                status: status.to_string(),
            },
        )
        .ok();
}

pub fn emit_shared_file_status(app_handle: &AppHandle, file: &SharedDriveFile, status: &str) {
    app_handle
        .emit(
            "shared-file-action-status",
            FileActionStatusPayload {
                id: file.id.clone(),
                account: file.account.clone(),
                status: status.to_string(),
            },
        )
        .ok();
}

async fn permanently_delete_file_with_retry(
    client: &reqwest::Client,
    file_id: &str,
    access_token: &str,
) -> anyhow::Result<()> {
    let url = format!(
        "https://www.googleapis.com/drive/v3/files/{}",
        api_path_segment(file_id)?
    );
    send_delete_with_retry(client, &url, access_token).await
}

async fn trash_file_with_retry(
    client: &reqwest::Client,
    file_id: &str,
    access_token: &str,
) -> anyhow::Result<()> {
    let url = format!(
        "https://www.googleapis.com/drive/v3/files/{}",
        api_path_segment(file_id)?
    );
    let mut last_error = None;
    for attempt in 0..3 {
        match client
            .patch(&url)
            .bearer_auth(access_token)
            .json(&serde_json::json!({ "trashed": true }))
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
                        .unwrap_or_else(|| anyhow!("Google API 移入回收站失败 ({})", status)),
                );
                if status.as_u16() != 429 && !status.is_server_error() {
                    break;
                }
            }
            Err(e) => last_error = Some(e.without_url().into()),
        }
        tokio::time::sleep(Duration::from_millis(400 * (attempt + 1) as u64)).await;
    }

    Err(last_error.unwrap_or_else(|| anyhow!("Google API 移入回收站失败")))
}

async fn send_delete_with_retry(
    client: &reqwest::Client,
    url: &str,
    access_token: &str,
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
                        .unwrap_or_else(|| anyhow!("Google API 删除请求失败 ({})", status)),
                );
                if status.as_u16() != 429 && !status.is_server_error() {
                    break;
                }
            }
            Err(e) => last_error = Some(e.without_url().into()),
        }
        tokio::time::sleep(Duration::from_millis(400 * (attempt + 1) as u64)).await;
    }

    Err(last_error.unwrap_or_else(|| anyhow!("Google API 删除请求失败")))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reject_foreign_or_owner_permissions() {
        let mut permission = SharedPermission {
            id: "123".into(),
            p_type: "user".into(),
            role: "reader".into(),
            email_address: Some("me@example.com".into()),
        };
        assert!(is_removable_account_permission(
            &permission,
            "ME@example.com",
            "123"
        ));
        assert!(!is_removable_account_permission(
            &permission,
            "other@example.com",
            "123"
        ));
        assert!(!is_removable_account_permission(
            &permission,
            "me@example.com",
            "456"
        ));
        permission.role = "owner".into();
        assert!(!is_removable_account_permission(
            &permission,
            "me@example.com",
            "123"
        ));
        permission.role = "reader".into();
        permission.p_type = "group".into();
        assert!(!is_removable_account_permission(
            &permission,
            "me@example.com",
            "123"
        ));
    }
}
