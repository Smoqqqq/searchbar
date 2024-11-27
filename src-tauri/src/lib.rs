use crate::configuration::Configuration;
use std::process::Command;
use std::string::ToString;
use enigo::{Button, Coordinate, Direction, Enigo, Mouse, Settings};
use tauri::Manager;

mod configuration;
mod disk_parser;
mod cache;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command(async)]
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

#[tauri::command]
fn search_from_cache(filename: &str) -> String {
    let cache = cache::ResultCache::new();
    let results = cache.get(filename.to_string());

    match results {
        Some(results) => serde_json::to_string(results).unwrap(),
        None => "[]".parse().unwrap(),
    }
}

#[tauri::command]
async fn click_window(app_handle: tauri::AppHandle) {
    let mut enigo = Enigo::new(&Settings::default()).unwrap();

    let window = app_handle.get_window("main").unwrap();
    let position = window.inner_position().unwrap();
    let size = window.inner_size().unwrap();

    let x: i32 = position.x;
    let y: i32 = position.y;
    let width: i32 = size.width as i32;

    enigo.move_mouse(x + width / 2, y + 25, Coordinate::Abs).unwrap();
    enigo.button(Button::Left, Direction::Click).unwrap();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![search, search_from_cache, launch, click_window])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Configuration::new().init();
}
