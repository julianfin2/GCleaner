use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use tauri::{AppHandle, Emitter};
use url::Url;

use crate::auth::{api_path_segment, google_client, parse_google_response, send_with_retry};
use crate::models::{FileActionStatusPayload, TaskCleanupItem};
use crate::tokens::get_valid_access_token;

#[derive(Debug, Clone)]
struct TaskList {
    id: String,
    title: String,
}

pub async fn scan_task_cleanup_items(
    account: String,
    tokens_dir: PathBuf,
) -> anyhow::Result<Vec<TaskCleanupItem>> {
    let client = google_client()?;
    let token = get_valid_access_token(&tokens_dir, &account)?;
    let default_list = get_default_task_list(&client, &token.access_token).await?;
    let task_lists = list_task_lists(&client, &token.access_token).await?;
    let mut items = Vec::new();

    for list in task_lists {
        let is_default = list.id == default_list.id;
        let task_count = count_tasks_in_list(&client, &token.access_token, &list.id).await?;
        if !is_default || task_count > 0 {
            items.push(TaskCleanupItem {
                id: format!("{}:{}", account, list.id),
                task_list_id: list.id,
                title: list.title,
                account: account.clone(),
                is_default,
                task_count,
            });
        }
    }

    Ok(items)
}

pub async fn cleanup_task_list(
    client: &reqwest::Client,
    item: &TaskCleanupItem,
    access_token: &str,
) -> anyhow::Result<()> {
    if item.is_default {
        delete_all_tasks_in_list(client, access_token, &item.task_list_id).await
    } else {
        delete_task_list(client, access_token, &item.task_list_id).await
    }
}

pub fn emit_task_cleanup_status(app_handle: &AppHandle, item: &TaskCleanupItem, status: &str) {
    app_handle
        .emit(
            "task-cleanup-status",
            FileActionStatusPayload {
                id: item.id.clone(),
                account: item.account.clone(),
                status: status.to_string(),
            },
        )
        .ok();
}

async fn get_default_task_list(
    client: &reqwest::Client,
    access_token: &str,
) -> anyhow::Result<TaskList> {
    let body = send_with_retry(|| {
        client
            .get("https://tasks.googleapis.com/tasks/v1/users/@me/lists/@default")
            .bearer_auth(access_token)
    })
    .await?;
    parse_task_list(&body).ok_or_else(|| anyhow!("无法识别默认任务列表"))
}

async fn list_task_lists(
    client: &reqwest::Client,
    access_token: &str,
) -> anyhow::Result<Vec<TaskList>> {
    let mut lists = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut url = Url::parse("https://tasks.googleapis.com/tasks/v1/users/@me/lists")?;
        url.query_pairs_mut().append_pair("maxResults", "1000");
        if let Some(ref token) = page_token {
            url.query_pairs_mut().append_pair("pageToken", token);
        }

        let body = send_with_retry(|| client.get(url.clone()).bearer_auth(access_token)).await?;
        for item in body["items"].as_array().cloned().unwrap_or_default() {
            if let Some(list) = parse_task_list(&item) {
                lists.push(list);
            }
        }

        page_token = body["nextPageToken"].as_str().map(|s| s.to_string());
        if page_token.is_none() {
            break;
        }
    }

    Ok(lists)
}

async fn count_tasks_in_list(
    client: &reqwest::Client,
    access_token: &str,
    task_list_id: &str,
) -> anyhow::Result<usize> {
    let mut count = 0usize;
    let mut page_token: Option<String> = None;

    loop {
        let body =
            list_tasks_page(client, access_token, task_list_id, page_token.as_deref()).await?;
        count += body["items"]
            .as_array()
            .map(|items| items.len())
            .unwrap_or(0);
        page_token = body["nextPageToken"].as_str().map(|s| s.to_string());
        if page_token.is_none() {
            break;
        }
    }

    Ok(count)
}

async fn delete_all_tasks_in_list(
    client: &reqwest::Client,
    access_token: &str,
    task_list_id: &str,
) -> anyhow::Result<()> {
    let mut page_token: Option<String> = None;

    loop {
        let body =
            list_tasks_page(client, access_token, task_list_id, page_token.as_deref()).await?;
        for task in body["items"].as_array().cloned().unwrap_or_default() {
            let Some(task_id) = task["id"].as_str() else {
                continue;
            };
            delete_task(client, access_token, task_list_id, task_id).await?;
        }

        page_token = body["nextPageToken"].as_str().map(|s| s.to_string());
        if page_token.is_none() {
            break;
        }
    }

    Ok(())
}

async fn list_tasks_page(
    client: &reqwest::Client,
    access_token: &str,
    task_list_id: &str,
    page_token: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    let mut url = Url::parse(&format!(
        "https://tasks.googleapis.com/tasks/v1/lists/{}/tasks",
        api_path_segment(task_list_id)?
    ))?;
    url.query_pairs_mut()
        .append_pair("maxResults", "100")
        .append_pair("showCompleted", "true")
        .append_pair("showHidden", "true")
        .append_pair("showDeleted", "false");
    if let Some(token) = page_token {
        url.query_pairs_mut().append_pair("pageToken", token);
    }

    send_with_retry(|| client.get(url.clone()).bearer_auth(access_token)).await
}

async fn delete_task_list(
    client: &reqwest::Client,
    access_token: &str,
    task_list_id: &str,
) -> anyhow::Result<()> {
    let url = format!(
        "https://tasks.googleapis.com/tasks/v1/users/@me/lists/{}",
        api_path_segment(task_list_id)?
    );
    send_delete_with_retry(client, &url, access_token, "Tasks API 删除任务列表失败").await
}

async fn delete_task(
    client: &reqwest::Client,
    access_token: &str,
    task_list_id: &str,
    task_id: &str,
) -> anyhow::Result<()> {
    let url = format!(
        "https://tasks.googleapis.com/tasks/v1/lists/{}/tasks/{}",
        api_path_segment(task_list_id)?,
        api_path_segment(task_id)?
    );
    send_delete_with_retry(client, &url, access_token, "Tasks API 删除任务失败").await
}

async fn send_delete_with_retry(
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

fn parse_task_list(value: &serde_json::Value) -> Option<TaskList> {
    let id = value["id"].as_str()?.to_string();
    let title = value["title"]
        .as_str()
        .unwrap_or("(无标题任务列表)")
        .to_string();
    Some(TaskList { id, title })
}
