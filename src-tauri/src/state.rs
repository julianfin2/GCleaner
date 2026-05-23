use std::fs;
use std::path::PathBuf;

use tauri::{AppHandle, Emitter, Manager};

pub struct AppState {
    pub app_handle: AppHandle,
}

impl AppState {
    pub fn get_tokens_dir(&self) -> PathBuf {
        let dir = self
            .app_handle
            .path()
            .app_data_dir()
            .expect("failed to get app data dir")
            .join("tokens");
        if !dir.exists() {
            fs::create_dir_all(&dir).ok();
        }
        dir
    }

    pub fn log(&self, message: &str) {
        println!("{}", message);
        self.app_handle.emit("log", message).ok();
    }
}
