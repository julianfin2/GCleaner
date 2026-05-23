mod auth;
mod commands;
mod contacts;
mod drive;
mod gmail;
mod models;
mod state;
mod tasks;
mod tokens;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    commands::run();
}
