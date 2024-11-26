use crate::configuration::Configuration;
use std::process::Command;
use tauri::Manager;

mod configuration;
mod disk_parser;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn search(filename: &str) -> String {
    disk_parser::DiskParser::new().search(filename.to_string())
}

#[tauri::command]
fn launch(filepath: &str) -> String {
    let result = match Command::new("cmd")
        .args(["/C", "start", "", filepath])
        .spawn()
    {
        Ok(result) => result,
        Err(error) => return format!("Error launching file: {}", error),
    };

    format!("Launched file: {:?}", result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![search, launch])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Configuration::new().init();
}
