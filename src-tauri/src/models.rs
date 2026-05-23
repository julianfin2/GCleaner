use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<u64>,
    pub email: Option<String>,
    pub expiry_date: Option<i64>,
    #[serde(default)]
    pub scopes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileActionStatusPayload {
    pub id: String,
    pub account: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DriveFile {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub web_view_link: Option<String>,
    pub account: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SharedDriveFile {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub web_view_link: Option<String>,
    pub account: String,
    pub permission_id: String,
    pub owner: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthorizedAccount {
    pub email: String,
    pub expires_at: Option<i64>,
    pub scopes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenImportFailure {
    pub line: usize,
    pub preview: String,
    pub error: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenImportReport {
    pub imported: Vec<AuthorizedAccount>,
    pub failures: Vec<TokenImportFailure>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MailCleanupCandidate {
    pub id: String,
    pub thread_id: String,
    pub account: String,
    pub subject: String,
    pub from: String,
    pub date: String,
    pub snippet: String,
    pub drive_links: Vec<MailDriveFile>,
    pub removable_permissions: usize,
    pub ignored_files: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MailDriveFile {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub permission_id: Option<String>,
    pub owner: String,
    pub reason: String,
    pub removable: bool,
    pub resource_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContactItem {
    pub id: String,
    pub resource_name: String,
    pub display_name: String,
    pub email: String,
    pub phone: String,
    pub account: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContactGroupItem {
    pub id: String,
    pub resource_name: String,
    pub name: String,
    pub member_count: usize,
    pub account: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskCleanupItem {
    pub id: String,
    pub task_list_id: String,
    pub title: String,
    pub account: String,
    pub is_default: bool,
    pub task_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SharedPermission {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub role: String,
    #[serde(default, rename = "type")]
    pub p_type: String,
    #[serde(default, rename = "emailAddress")]
    pub email_address: Option<String>,
}
