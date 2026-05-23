use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use tauri::{AppHandle, Emitter};
use url::Url;

use crate::auth::{parse_google_response, send_with_retry};
use crate::models::{ContactGroupItem, ContactItem, FileActionStatusPayload};
use crate::tokens::get_valid_access_token;

pub async fn scan_contacts(
    account: String,
    tokens_dir: PathBuf,
) -> anyhow::Result<Vec<ContactItem>> {
    let client = reqwest::Client::new();
    let token = get_valid_access_token(&tokens_dir, &account)?;
    let mut contacts = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut url = Url::parse("https://people.googleapis.com/v1/people/me/connections")?;
        url.query_pairs_mut()
            .append_pair("pageSize", "1000")
            .append_pair("personFields", "names,emailAddresses,phoneNumbers");
        if let Some(ref token) = page_token {
            url.query_pairs_mut().append_pair("pageToken", token);
        }

        let body =
            send_with_retry(|| client.get(url.clone()).bearer_auth(&token.access_token)).await?;
        let connections = body["connections"].as_array().cloned().unwrap_or_default();

        for contact in connections {
            let resource_name = contact["resourceName"].as_str().unwrap_or_default();
            if resource_name.is_empty() {
                continue;
            }

            contacts.push(ContactItem {
                id: resource_name.to_string(),
                resource_name: resource_name.to_string(),
                display_name: first_value(&contact, "names", "displayName")
                    .unwrap_or_else(|| "(无姓名)".to_string()),
                email: first_value(&contact, "emailAddresses", "value").unwrap_or_default(),
                phone: first_value(&contact, "phoneNumbers", "value").unwrap_or_default(),
                account: account.clone(),
            });
        }

        page_token = body["nextPageToken"].as_str().map(|s| s.to_string());
        if page_token.is_none() {
            break;
        }
    }

    Ok(contacts)
}

pub async fn scan_contact_groups(
    account: String,
    tokens_dir: PathBuf,
) -> anyhow::Result<Vec<ContactGroupItem>> {
    let client = reqwest::Client::new();
    let token = get_valid_access_token(&tokens_dir, &account)?;
    let mut groups = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut url = Url::parse("https://people.googleapis.com/v1/contactGroups")?;
        url.query_pairs_mut()
            .append_pair("pageSize", "1000")
            .append_pair("groupFields", "metadata,name,groupType,memberCount");
        if let Some(ref token) = page_token {
            url.query_pairs_mut().append_pair("pageToken", token);
        }

        let body =
            send_with_retry(|| client.get(url.clone()).bearer_auth(&token.access_token)).await?;
        let contact_groups = body["contactGroups"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        for group in contact_groups {
            if group["groupType"].as_str() != Some("USER_CONTACT_GROUP") {
                continue;
            }

            let resource_name = group["resourceName"].as_str().unwrap_or_default();
            if resource_name.is_empty() {
                continue;
            }

            groups.push(ContactGroupItem {
                id: resource_name.to_string(),
                resource_name: resource_name.to_string(),
                name: group["formattedName"]
                    .as_str()
                    .or_else(|| group["name"].as_str())
                    .unwrap_or("(未命名标签)")
                    .to_string(),
                member_count: group["memberCount"].as_u64().unwrap_or(0) as usize,
                account: account.clone(),
            });
        }

        page_token = body["nextPageToken"].as_str().map(|s| s.to_string());
        if page_token.is_none() {
            break;
        }
    }

    Ok(groups)
}

pub async fn delete_contacts_batch(
    client: &reqwest::Client,
    resource_names: &[String],
    access_token: &str,
) -> anyhow::Result<()> {
    let url = "https://people.googleapis.com/v1/people:batchDeleteContacts";
    let mut last_error = None;
    for attempt in 0..3 {
        match client
            .post(url)
            .bearer_auth(access_token)
            .json(&serde_json::json!({ "resourceNames": resource_names }))
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
                        .unwrap_or_else(|| anyhow!("People API 删除联系人失败 ({})", status)),
                );
                if status.as_u16() != 429 && !status.is_server_error() {
                    break;
                }
            }
            Err(e) => last_error = Some(e.into()),
        }
        tokio::time::sleep(Duration::from_millis(400 * (attempt + 1) as u64)).await;
    }

    Err(last_error.unwrap_or_else(|| anyhow!("People API 删除联系人失败")))
}

pub async fn delete_contact_group(
    client: &reqwest::Client,
    group: &ContactGroupItem,
    access_token: &str,
) -> anyhow::Result<()> {
    let url = format!(
        "https://people.googleapis.com/v1/{}",
        group.resource_name.trim_start_matches('/')
    );
    let mut last_error = None;
    for attempt in 0..3 {
        match client
            .delete(&url)
            .bearer_auth(access_token)
            .query(&[("deleteContacts", "false")])
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
                        .unwrap_or_else(|| anyhow!("People API 删除标签失败 ({})", status)),
                );
                if status.as_u16() != 429 && !status.is_server_error() {
                    break;
                }
            }
            Err(e) => last_error = Some(e.into()),
        }
        tokio::time::sleep(Duration::from_millis(400 * (attempt + 1) as u64)).await;
    }

    Err(last_error.unwrap_or_else(|| anyhow!("People API 删除标签失败")))
}

pub fn emit_contact_status(app_handle: &AppHandle, contact: &ContactItem, status: &str) {
    app_handle
        .emit(
            "contact-action-status",
            FileActionStatusPayload {
                id: contact.id.clone(),
                account: contact.account.clone(),
                status: status.to_string(),
            },
        )
        .ok();
}

pub fn emit_contact_group_status(app_handle: &AppHandle, group: &ContactGroupItem, status: &str) {
    app_handle
        .emit(
            "contact-group-action-status",
            FileActionStatusPayload {
                id: group.id.clone(),
                account: group.account.clone(),
                status: status.to_string(),
            },
        )
        .ok();
}

fn first_value(person: &serde_json::Value, list_field: &str, value_field: &str) -> Option<String> {
    person[list_field]
        .as_array()
        .and_then(|items| items.first())
        .and_then(|item| item[value_field].as_str())
        .map(|value| value.to_string())
}
