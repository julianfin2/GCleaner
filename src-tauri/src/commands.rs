use std::collections::HashMap;
use std::time::Duration;

use futures::future::join_all;
use tauri::{Manager, State};

use crate::contacts::{
    delete_contact_group, delete_contacts_batch, emit_contact_group_status, emit_contact_status,
    scan_contact_groups, scan_contacts,
};
use crate::drive::{
    delete_drive_file, emit_file_status, emit_shared_file_status, remove_shared_permission,
    scan_owned_top_level, scan_shared_account,
};
use crate::gmail::{
    emit_mail_cleanup_status, process_mail_cleanup_candidate, scan_mail_cleanup_candidates,
};
use crate::models::{
    AuthorizedAccount, ContactGroupItem, ContactItem, DriveFile, MailCleanupCandidate,
    SharedDriveFile, TaskCleanupItem, TokenImportReport,
};
use crate::state::AppState;
use crate::tasks::{cleanup_task_list, emit_task_cleanup_status, scan_task_cleanup_items};
use crate::tokens::{
    clear_authorized_account_files, delete_authorized_account_file, get_valid_access_token,
    import_access_tokens, list_authorized_accounts,
};

const WRITE_REQUEST_INTERVAL_MS: u64 = 200;

async fn throttle_write_request() {
    tokio::time::sleep(Duration::from_millis(WRITE_REQUEST_INTERVAL_MS)).await;
}

#[tauri::command]
async fn import_tokens(
    state: State<'_, AppState>,
    tokens: Vec<String>,
) -> std::result::Result<TokenImportReport, String> {
    let report = import_access_tokens(&state.get_tokens_dir(), tokens)
        .await
        .map_err(|e| e.to_string())?;
    for account in &report.imported {
        state.log(&format!(
            "已导入账号 {} 的短期 access token。",
            account.email
        ));
    }
    for failure in &report.failures {
        state.log(&format!(
            "第 {} 行 token {} 导入失败：{}",
            failure.line, failure.preview, failure.error
        ));
    }
    Ok(report)
}

#[tauri::command]
async fn get_authorized_accounts(
    state: State<'_, AppState>,
) -> std::result::Result<Vec<AuthorizedAccount>, String> {
    Ok(list_authorized_accounts(&state.get_tokens_dir()))
}

#[tauri::command]
async fn delete_authorized_account(
    state: State<'_, AppState>,
    account: String,
) -> std::result::Result<(), String> {
    let removed = delete_authorized_account_file(&state.get_tokens_dir(), &account)
        .map_err(|e| e.to_string())?;
    if removed {
        state.log(&format!("已删除账号 {} 的本地 token。", account));
    }
    Ok(())
}

#[tauri::command]
async fn clear_authorized_accounts(
    state: State<'_, AppState>,
) -> std::result::Result<usize, String> {
    let removed =
        clear_authorized_account_files(&state.get_tokens_dir()).map_err(|e| e.to_string())?;
    state.log(&format!("已清空 {} 个本地 token。", removed));
    Ok(removed)
}

#[tauri::command]
async fn scan_drive(
    state: State<'_, AppState>,
    accounts: Vec<String>,
) -> std::result::Result<Vec<DriveFile>, String> {
    state.log(&format!("开始扫描 {} 个账号...", accounts.len()));
    let tokens_dir = state.get_tokens_dir();
    let mut all_files = Vec::new();

    let mut tasks = Vec::new();
    for account in accounts {
        let tokens_dir_clone = tokens_dir.clone();
        tasks.push(tokio::spawn(async move {
            scan_owned_top_level(account, tokens_dir_clone).await
        }));
    }

    let mut failures = Vec::new();
    let results = join_all(tasks).await;
    for res in results {
        match res {
            Ok(Ok(files)) => all_files.extend(files),
            Ok(Err(e)) => failures.push(e.to_string()),
            Err(e) => failures.push(e.to_string()),
        }
    }

    state.log(&format!(
        "扫描完成，共发现 {} 个当前账号拥有的文件或目录。",
        all_files.len()
    ));

    if !failures.is_empty() {
        let message = format!("部分账号扫描失败：{}", failures.join("；"));
        state.log(&message);
        return Err(message);
    }

    Ok(all_files)
}

#[tauri::command]
async fn delete_files(
    state: State<'_, AppState>,
    files: Vec<DriveFile>,
    permanently_delete: bool,
) -> std::result::Result<(), String> {
    let action_label = if permanently_delete {
        "永久删除"
    } else {
        "移入回收站"
    };
    state.log(&format!(
        "开始{} {} 个文件或目录...",
        action_label,
        files.len()
    ));
    let tokens_dir = state.get_tokens_dir();
    let app_handle = state.app_handle.clone();

    let mut account_groups: HashMap<String, Vec<DriveFile>> = HashMap::new();
    for file in files {
        account_groups
            .entry(file.account.clone())
            .or_default()
            .push(file);
    }

    let mut failures = Vec::new();
    for (account, files_to_clean) in account_groups {
        let client = reqwest::Client::new();
        let token = match get_valid_access_token(&tokens_dir, &account) {
            Ok(token) => token,
            Err(e) => {
                let message = format!("[{}] token 刷新失败：{}", account, e);
                for file in files_to_clean {
                    emit_file_status(&app_handle, &file, "failed");
                }
                failures.push(message);
                continue;
            }
        };

        for file in &files_to_clean {
            emit_file_status(&app_handle, file, "cleaning");
            let result =
                delete_drive_file(&client, &file.id, &token.access_token, permanently_delete).await;

            match result {
                Ok(()) => {
                    state.log(&format!("[{}] 已{}: {}", account, action_label, file.name));
                    emit_file_status(&app_handle, file, "done");
                }
                Err(e) => {
                    let message =
                        format!("[{}] {} {}失败：{}", account, file.name, action_label, e);
                    state.log(&message);
                    failures.push(message);
                    emit_file_status(&app_handle, file, "failed");
                }
            }
            throttle_write_request().await;
        }
    }

    if failures.is_empty() {
        state.log(&format!("所有{}任务已完成。", action_label));
        Ok(())
    } else {
        let message = format!(
            "{}流程完成，但有 {} 个步骤失败。",
            action_label,
            failures.len()
        );
        state.log(&message);
        Err(failures.join("；"))
    }
}

#[tauri::command]
async fn scan_shared_drive(
    state: State<'_, AppState>,
    accounts: Vec<String>,
) -> std::result::Result<Vec<SharedDriveFile>, String> {
    state.log(&format!(
        "开始扫描 {} 个账号的“与我共享”文件...",
        accounts.len()
    ));
    let tokens_dir = state.get_tokens_dir();
    let mut all_files = Vec::new();

    let mut tasks = Vec::new();
    for account in accounts {
        let tokens_dir_clone = tokens_dir.clone();
        tasks.push(tokio::spawn(async move {
            scan_shared_account(account, tokens_dir_clone).await
        }));
    }

    let mut failures = Vec::new();
    let results = join_all(tasks).await;
    for res in results {
        match res {
            Ok(Ok(files)) => all_files.extend(files),
            Ok(Err(e)) => failures.push(e.to_string()),
            Err(e) => failures.push(e.to_string()),
        }
    }

    state.log(&format!(
        "扫描完成，共发现 {} 个可移除当前账号权限的共享文件。",
        all_files.len()
    ));

    if !failures.is_empty() {
        let message = format!("部分账号扫描失败：{}", failures.join("；"));
        state.log(&message);
        return Err(message);
    }

    Ok(all_files)
}

#[tauri::command]
async fn remove_shared_permissions(
    state: State<'_, AppState>,
    files: Vec<SharedDriveFile>,
) -> std::result::Result<(), String> {
    state.log(&format!(
        "开始移除 {} 个共享文件中的当前账号权限...",
        files.len()
    ));
    let tokens_dir = state.get_tokens_dir();
    let app_handle = state.app_handle.clone();

    let mut account_groups: HashMap<String, Vec<SharedDriveFile>> = HashMap::new();
    for file in files {
        account_groups
            .entry(file.account.clone())
            .or_default()
            .push(file);
    }

    let mut failures = Vec::new();
    for (account, files_to_clean) in account_groups {
        let client = reqwest::Client::new();
        let token = match get_valid_access_token(&tokens_dir, &account) {
            Ok(token) => token,
            Err(e) => {
                let message = format!("[{}] token 刷新失败：{}", account, e);
                for file in files_to_clean {
                    emit_shared_file_status(&app_handle, &file, "failed");
                }
                failures.push(message);
                continue;
            }
        };

        for file in &files_to_clean {
            emit_shared_file_status(&app_handle, file, "cleaning");
            match remove_shared_permission(&client, file, &token.access_token).await {
                Ok(()) => {
                    state.log(&format!("[{}] 已移除共享权限: {}", account, file.name));
                    emit_shared_file_status(&app_handle, file, "done");
                }
                Err(e) => {
                    let message = format!("[{}] {} 移除共享权限失败：{}", account, file.name, e);
                    state.log(&message);
                    failures.push(message);
                    emit_shared_file_status(&app_handle, file, "failed");
                }
            }
            throttle_write_request().await;
        }
    }

    if failures.is_empty() {
        state.log("所有共享权限移除任务已完成。");
        Ok(())
    } else {
        let message = format!("共享权限移除流程完成，但有 {} 个文件失败。", failures.len());
        state.log(&message);
        Err(failures.join("；"))
    }
}

#[tauri::command]
async fn scan_mail_cleanup(
    state: State<'_, AppState>,
    accounts: Vec<String>,
    query: String,
) -> std::result::Result<Vec<MailCleanupCandidate>, String> {
    let query = if query.trim().is_empty() {
        "has:drive -from:me".to_string()
    } else {
        query.trim().to_string()
    };
    state.log(&format!(
        "开始按条件“{}”扫描 {} 个账号的 Gmail 文件线索...",
        query,
        accounts.len()
    ));
    let tokens_dir = state.get_tokens_dir();
    let mut all_candidates = Vec::new();

    let mut tasks = Vec::new();
    for account in accounts {
        let tokens_dir_clone = tokens_dir.clone();
        let query_clone = query.clone();
        tasks.push(tokio::spawn(async move {
            scan_mail_cleanup_candidates(account, tokens_dir_clone, query_clone).await
        }));
    }

    let mut failures = Vec::new();
    let results = join_all(tasks).await;
    for res in results {
        match res {
            Ok(Ok(candidates)) => all_candidates.extend(candidates),
            Ok(Err(e)) => failures.push(e.to_string()),
            Err(e) => failures.push(e.to_string()),
        }
    }

    let removable_count: usize = all_candidates
        .iter()
        .map(|candidate| candidate.removable_permissions)
        .sum();
    state.log(&format!(
        "邮件扫描完成，共发现 {} 封带 Drive 链接的邮件，其中 {} 个权限可明确移除。",
        all_candidates.len(),
        removable_count
    ));

    if !failures.is_empty() {
        let message = format!("部分账号邮件扫描失败：{}", failures.join("；"));
        state.log(&message);
        return Err(message);
    }

    Ok(all_candidates)
}

#[tauri::command]
async fn cleanup_mail_candidates(
    state: State<'_, AppState>,
    candidates: Vec<MailCleanupCandidate>,
    permanently_delete: bool,
) -> std::result::Result<(), String> {
    let action_label = if permanently_delete {
        "永久删除"
    } else {
        "移入回收站"
    };
    state.log(&format!(
        "开始处理 {} 封邮件：移除可明确处理的 Drive 权限，并将邮件{}...",
        candidates.len(),
        action_label
    ));
    let tokens_dir = state.get_tokens_dir();
    let app_handle = state.app_handle.clone();

    let mut account_groups: HashMap<String, Vec<MailCleanupCandidate>> = HashMap::new();
    for candidate in candidates {
        account_groups
            .entry(candidate.account.clone())
            .or_default()
            .push(candidate);
    }

    let mut failures = Vec::new();
    let mut removed_permissions = 0usize;
    let mut failed_permissions = 0usize;
    let mut trashed_messages = 0usize;

    for (account, candidates_to_clean) in account_groups {
        let client = reqwest::Client::new();
        let token = match get_valid_access_token(&tokens_dir, &account) {
            Ok(token) => token,
            Err(e) => {
                let message = format!("[{}] token 刷新失败：{}", account, e);
                for candidate in candidates_to_clean {
                    emit_mail_cleanup_status(&app_handle, &candidate, "failed");
                }
                failures.push(message);
                continue;
            }
        };

        for candidate in &candidates_to_clean {
            emit_mail_cleanup_status(&app_handle, candidate, "cleaning");
            match process_mail_cleanup_candidate(
                &client,
                candidate,
                &token.access_token,
                permanently_delete,
            )
            .await
            {
                Ok((removed, failed)) => {
                    removed_permissions += removed;
                    failed_permissions += failed;
                    trashed_messages += 1;
                    state.log(&format!(
                        "[{}] 已处理邮件: {}，移除权限 {} 个，权限失败 {} 个，邮件已{}。",
                        account, candidate.subject, removed, failed, action_label
                    ));
                    emit_mail_cleanup_status(&app_handle, candidate, "done");
                }
                Err(e) => {
                    let message =
                        format!("[{}] 邮件 {} 处理失败：{}", account, candidate.subject, e);
                    state.log(&message);
                    failures.push(message);
                    emit_mail_cleanup_status(&app_handle, candidate, "failed");
                }
            }
            throttle_write_request().await;
        }
    }

    state.log(&format!(
        "清理邮件完成：已移除 {} 个权限，{} 个权限移除失败，{} 封邮件已{}。",
        removed_permissions, failed_permissions, trashed_messages, action_label
    ));

    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("；"))
    }
}

#[tauri::command]
async fn scan_contacts_cleanup(
    state: State<'_, AppState>,
    accounts: Vec<String>,
) -> std::result::Result<Vec<ContactItem>, String> {
    state.log(&format!(
        "开始扫描 {} 个账号的通讯录联系人...",
        accounts.len()
    ));
    let tokens_dir = state.get_tokens_dir();
    let mut all_contacts = Vec::new();

    let mut tasks = Vec::new();
    for account in accounts {
        let tokens_dir_clone = tokens_dir.clone();
        tasks.push(tokio::spawn(async move {
            scan_contacts(account, tokens_dir_clone).await
        }));
    }

    let mut failures = Vec::new();
    let results = join_all(tasks).await;
    for res in results {
        match res {
            Ok(Ok(contacts)) => all_contacts.extend(contacts),
            Ok(Err(e)) => failures.push(e.to_string()),
            Err(e) => failures.push(e.to_string()),
        }
    }

    state.log(&format!(
        "通讯录扫描完成，共发现 {} 个联系人。",
        all_contacts.len()
    ));

    if !failures.is_empty() {
        let message = format!("部分账号通讯录扫描失败：{}", failures.join("；"));
        state.log(&message);
        return Err(message);
    }

    Ok(all_contacts)
}

#[tauri::command]
async fn scan_contact_groups_cleanup(
    state: State<'_, AppState>,
    accounts: Vec<String>,
) -> std::result::Result<Vec<ContactGroupItem>, String> {
    state.log(&format!(
        "开始扫描 {} 个账号的通讯录自定义标签...",
        accounts.len()
    ));
    let tokens_dir = state.get_tokens_dir();
    let mut all_groups = Vec::new();

    let mut tasks = Vec::new();
    for account in accounts {
        let tokens_dir_clone = tokens_dir.clone();
        tasks.push(tokio::spawn(async move {
            scan_contact_groups(account, tokens_dir_clone).await
        }));
    }

    let mut failures = Vec::new();
    let results = join_all(tasks).await;
    for res in results {
        match res {
            Ok(Ok(groups)) => all_groups.extend(groups),
            Ok(Err(e)) => failures.push(e.to_string()),
            Err(e) => failures.push(e.to_string()),
        }
    }

    state.log(&format!(
        "通讯录标签扫描完成，共发现 {} 个自定义标签。",
        all_groups.len()
    ));

    if !failures.is_empty() {
        let message = format!("部分账号通讯录标签扫描失败：{}", failures.join("；"));
        state.log(&message);
        return Err(message);
    }

    Ok(all_groups)
}

#[tauri::command]
async fn delete_contacts_cleanup(
    state: State<'_, AppState>,
    contacts: Vec<ContactItem>,
) -> std::result::Result<(), String> {
    state.log(&format!("开始删除 {} 个通讯录联系人...", contacts.len()));
    let tokens_dir = state.get_tokens_dir();
    let app_handle = state.app_handle.clone();

    let mut account_groups: HashMap<String, Vec<ContactItem>> = HashMap::new();
    for contact in contacts {
        account_groups
            .entry(contact.account.clone())
            .or_default()
            .push(contact);
    }

    let mut failures = Vec::new();
    let mut deleted_count = 0usize;
    for (account, contacts_to_delete) in account_groups {
        let client = reqwest::Client::new();
        let token = match get_valid_access_token(&tokens_dir, &account) {
            Ok(token) => token,
            Err(e) => {
                let message = format!("[{}] token 刷新失败：{}", account, e);
                for contact in contacts_to_delete {
                    emit_contact_status(&app_handle, &contact, "failed");
                }
                failures.push(message);
                continue;
            }
        };

        for chunk in contacts_to_delete.chunks(200) {
            for contact in chunk {
                emit_contact_status(&app_handle, contact, "cleaning");
            }

            let resource_names: Vec<String> = chunk
                .iter()
                .map(|contact| contact.resource_name.clone())
                .collect();
            match delete_contacts_batch(&client, &resource_names, &token.access_token).await {
                Ok(()) => {
                    deleted_count += chunk.len();
                    for contact in chunk {
                        emit_contact_status(&app_handle, contact, "done");
                    }
                    state.log(&format!(
                        "[{}] 已删除 {} 个通讯录联系人。",
                        account,
                        chunk.len()
                    ));
                }
                Err(e) => {
                    let message = format!(
                        "[{}] 删除 {} 个通讯录联系人失败：{}",
                        account,
                        chunk.len(),
                        e
                    );
                    state.log(&message);
                    failures.push(message);
                    for contact in chunk {
                        emit_contact_status(&app_handle, contact, "failed");
                    }
                }
            }
            throttle_write_request().await;
        }
    }

    state.log(&format!(
        "通讯录清理完成：已删除 {} 个联系人。",
        deleted_count
    ));

    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("；"))
    }
}

#[tauri::command]
async fn delete_contact_groups_cleanup(
    state: State<'_, AppState>,
    groups: Vec<ContactGroupItem>,
) -> std::result::Result<(), String> {
    state.log(&format!("开始删除 {} 个通讯录自定义标签...", groups.len()));
    let tokens_dir = state.get_tokens_dir();
    let app_handle = state.app_handle.clone();

    let mut account_groups: HashMap<String, Vec<ContactGroupItem>> = HashMap::new();
    for group in groups {
        account_groups
            .entry(group.account.clone())
            .or_default()
            .push(group);
    }

    let mut failures = Vec::new();
    let mut deleted_count = 0usize;
    for (account, groups_to_delete) in account_groups {
        let client = reqwest::Client::new();
        let token = match get_valid_access_token(&tokens_dir, &account) {
            Ok(token) => token,
            Err(e) => {
                let message = format!("[{}] token 刷新失败：{}", account, e);
                for group in groups_to_delete {
                    emit_contact_group_status(&app_handle, &group, "failed");
                }
                failures.push(message);
                continue;
            }
        };

        for group in &groups_to_delete {
            emit_contact_group_status(&app_handle, group, "cleaning");
            match delete_contact_group(&client, group, &token.access_token).await {
                Ok(()) => {
                    deleted_count += 1;
                    state.log(&format!("[{}] 已删除通讯录标签: {}", account, group.name));
                    emit_contact_group_status(&app_handle, group, "done");
                }
                Err(e) => {
                    let message =
                        format!("[{}] 删除通讯录标签 {} 失败：{}", account, group.name, e);
                    state.log(&message);
                    failures.push(message);
                    emit_contact_group_status(&app_handle, group, "failed");
                }
            }
            throttle_write_request().await;
        }
    }

    state.log(&format!(
        "通讯录标签清理完成：已删除 {} 个自定义标签。",
        deleted_count
    ));

    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("；"))
    }
}

#[tauri::command]
async fn scan_tasks_cleanup(
    state: State<'_, AppState>,
    accounts: Vec<String>,
) -> std::result::Result<Vec<TaskCleanupItem>, String> {
    state.log(&format!("开始扫描 {} 个账号的任务列表...", accounts.len()));
    let tokens_dir = state.get_tokens_dir();
    let mut all_items = Vec::new();

    let mut tasks = Vec::new();
    for account in accounts {
        let tokens_dir_clone = tokens_dir.clone();
        tasks.push(tokio::spawn(async move {
            scan_task_cleanup_items(account, tokens_dir_clone).await
        }));
    }

    let mut failures = Vec::new();
    let results = join_all(tasks).await;
    for res in results {
        match res {
            Ok(Ok(items)) => all_items.extend(items),
            Ok(Err(e)) => failures.push(e.to_string()),
            Err(e) => failures.push(e.to_string()),
        }
    }

    let list_count = all_items.iter().filter(|item| !item.is_default).count();
    let default_task_count: usize = all_items
        .iter()
        .filter(|item| item.is_default)
        .map(|item| item.task_count)
        .sum();
    state.log(&format!(
        "任务扫描完成，共发现 {} 个可删除任务列表，默认任务列表中 {} 个任务可删除。",
        list_count, default_task_count
    ));

    if !failures.is_empty() {
        let message = format!("部分账号任务扫描失败：{}", failures.join("；"));
        state.log(&message);
        return Err(message);
    }

    Ok(all_items)
}

#[tauri::command]
async fn cleanup_tasks(
    state: State<'_, AppState>,
    items: Vec<TaskCleanupItem>,
) -> std::result::Result<(), String> {
    state.log(&format!("开始清理 {} 个任务列表项目...", items.len()));
    let tokens_dir = state.get_tokens_dir();
    let app_handle = state.app_handle.clone();

    let mut account_groups: HashMap<String, Vec<TaskCleanupItem>> = HashMap::new();
    for item in items {
        account_groups
            .entry(item.account.clone())
            .or_default()
            .push(item);
    }

    let mut failures = Vec::new();
    for (account, items_to_clean) in account_groups {
        let client = reqwest::Client::new();
        let token = match get_valid_access_token(&tokens_dir, &account) {
            Ok(token) => token,
            Err(e) => {
                let message = format!("[{}] token 刷新失败：{}", account, e);
                for item in items_to_clean {
                    emit_task_cleanup_status(&app_handle, &item, "failed");
                }
                failures.push(message);
                continue;
            }
        };

        for item in &items_to_clean {
            emit_task_cleanup_status(&app_handle, item, "cleaning");
            match cleanup_task_list(&client, item, &token.access_token).await {
                Ok(()) => {
                    if item.is_default {
                        state.log(&format!(
                            "[{}] 已清空默认任务列表 {} 中的 {} 个任务。",
                            account, item.title, item.task_count
                        ));
                    } else {
                        state.log(&format!("[{}] 已删除任务列表: {}", account, item.title));
                    }
                    emit_task_cleanup_status(&app_handle, item, "done");
                }
                Err(e) => {
                    let action = if item.is_default {
                        "清空默认任务列表失败"
                    } else {
                        "删除任务列表失败"
                    };
                    let message = format!("[{}] {} {}：{}", account, item.title, action, e);
                    state.log(&message);
                    failures.push(message);
                    emit_task_cleanup_status(&app_handle, item, "failed");
                }
            }
            throttle_write_request().await;
        }
    }

    if failures.is_empty() {
        state.log("任务清理流程已完成。");
        Ok(())
    } else {
        Err(failures.join("；"))
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(AppState {
                app_handle: app.handle().clone(),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            import_tokens,
            get_authorized_accounts,
            delete_authorized_account,
            clear_authorized_accounts,
            scan_drive,
            delete_files,
            scan_shared_drive,
            remove_shared_permissions,
            scan_mail_cleanup,
            cleanup_mail_candidates,
            scan_contacts_cleanup,
            scan_contact_groups_cleanup,
            delete_contacts_cleanup,
            delete_contact_groups_cleanup,
            scan_tasks_cleanup,
            cleanup_tasks
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
