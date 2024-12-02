use std::collections::HashMap;
use crate::configuration::Configuration;
use std::process::Command;
use std::string::ToString;
use enigo::{Button, Coordinate, Direction, Enigo, Mouse, Settings};
use serde::Serialize;
use tauri::Manager;
use crate::db_cache::FileSystemEntry;

mod configuration;
mod disk_mapper;
mod search_result;
mod db_cache;

#[derive(Serialize)]
enum ReturnValue {
    U32(u32),
    Vec(Vec<FileSystemEntry>)
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command(async)]
fn search(filename: &str, page: u32) -> String {
    let mut mapper = disk_mapper::DiskMapper::new();

    if !mapper.is_mapped() {
        mapper.map();
    }

    let cache = db_cache::DbCache::new();

    let results = match cache.search(filename.clone(), &page) {
        Ok(results) => results,
        Err(error) => panic!("{}", error),
    };

    let mut data = HashMap::new();
    data.insert("results", ReturnValue::Vec(results));
    data.insert("count", ReturnValue::U32(cache.count(filename)));

    cache.to_json(data)
}

#[tauri::command(async)]
fn map_filesystem() -> String {
    let mut mapper = disk_mapper::DiskMapper::new();

    if !mapper.is_mapped() {
        mapper.map();
        return String::from("Mapped");
    }

    String::from("Already mapped")
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
        .invoke_handler(tauri::generate_handler![search, launch, click_window, map_filesystem])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Configuration::new().init();
}
