fn main() {
    let manifest = tauri_build::AppManifest::new().commands(&[
        "import_tokens",
        "get_authorized_accounts",
        "delete_authorized_account",
        "clear_authorized_accounts",
        "scan_drive",
        "delete_files",
        "scan_shared_drive",
        "remove_shared_permissions",
        "scan_mail_cleanup",
        "cleanup_mail_candidates",
        "scan_contacts_cleanup",
        "scan_contact_groups_cleanup",
        "delete_contacts_cleanup",
        "delete_contact_groups_cleanup",
        "scan_tasks_cleanup",
        "cleanup_tasks",
    ]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
        .expect("failed to build Tauri application");
}
