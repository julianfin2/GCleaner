use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};

use crate::auth::parse_google_response;
use crate::models::{AuthToken, AuthorizedAccount, TokenImportFailure, TokenImportReport};

pub fn token_path(tokens_dir: &Path, account: &str) -> PathBuf {
    tokens_dir.join(format!("{}.json", account))
}

pub fn validate_account_name(account: &str) -> anyhow::Result<()> {
    if account.trim().is_empty()
        || account.contains('/')
        || account.contains('\\')
        || account.contains("..")
    {
        return Err(anyhow!("账号名称无效"));
    }
    Ok(())
}

pub fn read_token(tokens_dir: &Path, account: &str) -> anyhow::Result<AuthToken> {
    validate_account_name(account)?;
    let content = fs::read_to_string(token_path(tokens_dir, account))
        .with_context(|| format!("无法读取账号 {} 的 access token", account))?;
    serde_json::from_str(&content)
        .with_context(|| format!("账号 {} 的 access token 文件格式错误", account))
}

pub fn write_token(tokens_dir: &Path, token: &AuthToken) -> anyhow::Result<()> {
    let email = token
        .email
        .as_deref()
        .ok_or_else(|| anyhow!("token 中没有 email，无法保存账号文件"))?;
    fs::create_dir_all(tokens_dir)?;
    fs::write(
        token_path(tokens_dir, email),
        serde_json::to_string_pretty(token)?,
    )?;
    Ok(())
}

pub async fn import_access_tokens(
    tokens_dir: &Path,
    raw_tokens: Vec<String>,
) -> anyhow::Result<TokenImportReport> {
    let client = reqwest::Client::new();
    let mut imported = Vec::new();
    let mut failures = Vec::new();

    for (index, raw_token) in raw_tokens.iter().enumerate() {
        let access_token = raw_token.trim();
        if access_token.is_empty() {
            continue;
        }

        match inspect_access_token(&client, access_token).await {
            Ok(token) => match write_token(tokens_dir, &token) {
                Ok(()) => imported.push(AuthorizedAccount {
                    email: token.email.clone().unwrap_or_default(),
                    expires_at: token.expiry_date,
                    scopes: token.scopes.clone(),
                }),
                Err(e) => failures.push(TokenImportFailure {
                    line: index + 1,
                    preview: token_preview(access_token),
                    error: format!("保存失败：{}", e),
                }),
            },
            Err(e) => failures.push(TokenImportFailure {
                line: index + 1,
                preview: token_preview(access_token),
                error: format!("校验失败，可能已经过期或无效：{}", e),
            }),
        }
    }

    Ok(TokenImportReport { imported, failures })
}

pub fn get_valid_access_token(tokens_dir: &Path, account: &str) -> anyhow::Result<AuthToken> {
    let token = read_token(tokens_dir, account)?;
    if is_token_expired(&token) {
        return Err(anyhow!(
            "账号 {} 的 access token 已过期，请重新导入",
            account
        ));
    }
    Ok(token)
}

pub fn list_authorized_accounts(tokens_dir: &Path) -> Vec<AuthorizedAccount> {
    let mut accounts = Vec::new();
    if let Ok(entries) = fs::read_dir(tokens_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(token) = serde_json::from_str::<AuthToken>(&content) {
                    if let Some(email) = token.email {
                        accounts.push(AuthorizedAccount {
                            email,
                            expires_at: token.expiry_date,
                            scopes: token.scopes,
                        });
                    }
                }
            }
        }
    }
    accounts.sort_by(|a, b| a.email.cmp(&b.email));
    accounts
}

pub fn delete_authorized_account_file(tokens_dir: &Path, account: &str) -> anyhow::Result<bool> {
    validate_account_name(account)?;
    let path = token_path(tokens_dir, account);
    if path.exists() {
        fs::remove_file(path)?;
        return Ok(true);
    }
    Ok(false)
}

pub fn clear_authorized_account_files(tokens_dir: &Path) -> anyhow::Result<usize> {
    let mut removed = 0;
    if let Ok(entries) = fs::read_dir(tokens_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                fs::remove_file(&path)?;
                removed += 1;
            }
        }
    }
    Ok(removed)
}

async fn inspect_access_token(
    client: &reqwest::Client,
    access_token: &str,
) -> anyhow::Result<AuthToken> {
    let token_info = parse_google_response(
        client
            .get("https://oauth2.googleapis.com/tokeninfo")
            .query(&[("access_token", access_token)])
            .send()
            .await?,
    )
    .await?;

    let expires_in = token_info["expires_in"]
        .as_str()
        .and_then(|value| value.parse::<u64>().ok())
        .or_else(|| token_info["expires_in"].as_u64());
    let scopes = token_info["scope"]
        .as_str()
        .map(|scope| {
            scope
                .split_whitespace()
                .map(|item| item.to_string())
                .collect()
        })
        .unwrap_or_else(Vec::new);

    let email = if let Some(email) = token_info["email"].as_str() {
        email.to_string()
    } else if let Ok(email) = get_userinfo_email(client, access_token).await {
        email
    } else if let Ok(email) = get_gmail_profile_email(client, access_token).await {
        email
    } else if let Ok(email) = get_drive_about_email(client, access_token).await {
        email
    } else {
        return Err(anyhow!(
            "无法从 access token 识别账号邮箱，请确认 token 包含 userinfo.email、Gmail 或 Drive 权限"
        ));
    };

    Ok(AuthToken {
        access_token: access_token.to_string(),
        token_type: "Bearer".to_string(),
        expires_in,
        email: Some(email),
        expiry_date: expires_in.map(|seconds| chrono::Utc::now().timestamp() + seconds as i64),
        scopes,
    })
}

fn token_preview(access_token: &str) -> String {
    let token = access_token.trim();
    let length = token.chars().count();
    let start: String = token.chars().take(8).collect();
    let end: String = token
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if length > 16 {
        format!("{}...{}（{}字符）", start, end, length)
    } else {
        format!("{}（{}字符）", token, length)
    }
}

async fn get_userinfo_email(
    client: &reqwest::Client,
    access_token: &str,
) -> anyhow::Result<String> {
    let body = parse_google_response(
        client
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
            .bearer_auth(access_token)
            .send()
            .await?,
    )
    .await?;
    body["email"]
        .as_str()
        .map(|value| value.to_string())
        .ok_or_else(|| anyhow!("userinfo 响应中没有 email"))
}

async fn get_gmail_profile_email(
    client: &reqwest::Client,
    access_token: &str,
) -> anyhow::Result<String> {
    let body = parse_google_response(
        client
            .get("https://gmail.googleapis.com/gmail/v1/users/me/profile")
            .bearer_auth(access_token)
            .send()
            .await?,
    )
    .await?;
    body["emailAddress"]
        .as_str()
        .map(|value| value.to_string())
        .ok_or_else(|| anyhow!("Gmail profile 响应中没有 emailAddress"))
}

async fn get_drive_about_email(
    client: &reqwest::Client,
    access_token: &str,
) -> anyhow::Result<String> {
    let body = parse_google_response(
        client
            .get("https://www.googleapis.com/drive/v3/about")
            .bearer_auth(access_token)
            .query(&[("fields", "user(emailAddress)")])
            .send()
            .await?,
    )
    .await?;
    body["user"]["emailAddress"]
        .as_str()
        .map(|value| value.to_string())
        .ok_or_else(|| anyhow!("Drive about 响应中没有 emailAddress"))
}

fn is_token_expired(token: &AuthToken) -> bool {
    token
        .expiry_date
        .map(|expiry| expiry <= chrono::Utc::now().timestamp() + 30)
        .unwrap_or(false)
}
