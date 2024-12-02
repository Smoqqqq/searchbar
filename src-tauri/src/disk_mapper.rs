use crate::configuration::{Configuration, ConfigurationValue};
use std::fs;
use std::sync::{Arc, OnceLock, Mutex};
use rayon::prelude::*;
use crossbeam_channel::{unbounded, Sender};
use crate::db_cache::{DbCache, FileSystemEntry};

pub struct DiskMapper {
    config: Configuration,
    db_cache: Arc<Mutex<DbCache>>, // Wrap DbCache in a Mutex
}

impl DiskMapper {
    pub fn new() -> Self {
        let config = get_config().clone();
        let db_cache = Arc::new(Mutex::new(DbCache::new()));

        DiskMapper { config, db_cache }
    }

    pub fn is_mapped(&self) -> bool {
        self.db_cache.try_lock().unwrap().db_exists()
    }

    pub fn map(&mut self) {
        println!("Starting to map filesystem...");

        // Create a channel for communication
        let (sender, receiver) = unbounded();

        // Spawn a thread for database writes
        let db_cache = self.db_cache.clone();
        std::thread::spawn(move || {
            let mut db_cache = db_cache.lock().unwrap();
            for entry in receiver {
                db_cache.store(entry);
            }
            db_cache.flush().expect("Failed to flush DbCache");
        });

        // Get the list of disk paths
        let disk_list = disk_list::get_disk_list();
        let disk_list = disk_list.iter().map(|x| x[2].clone()).collect::<Vec<String>>();

        // Parallel processing of disk paths
        disk_list.into_par_iter().for_each(|disk| {
            if let Ok(entries) = fs::read_dir(&disk) {
                let subdirs: Vec<_> = entries
                    .filter_map(Result::ok)
                    .filter(|entry| entry.file_type().map_or(false, |ft| ft.is_dir()))
                    .map(|entry| FileSystemEntry::new(
                        entry.path().display().to_string(),
                        entry.file_name().to_str().expect("Could not get file_name").to_string(),
                        true,
                    ))
                    .collect();

                subdirs.into_par_iter().for_each(|subdir| {
                    self.map_path_with_channel(subdir, &sender);
                });
            }
        });

        // Drop the sender to close the channel
        drop(sender);

        println!("Finished mapping filesystem.");
    }

    fn map_path_with_channel(&self, path: FileSystemEntry, sender: &Sender<FileSystemEntry>) {
        let mut stack = vec![path];

        while let Some(current_path) = stack.pop() {
            if self.is_excluded(&current_path.path) {
                println!("Excluding {}", current_path.path);
                continue;
            }

            let entries = match fs::read_dir(&current_path.path) {
                Ok(entries) => entries,
                Err(err) => {
                    println!("Error reading {}: {}", current_path.path, err);
                    continue;
                }
            };

            let mut subdirs = Vec::new();
            for entry in entries.filter_map(Result::ok) {
                let path_name = entry.path().display().to_string();

                let result = FileSystemEntry::new(
                    path_name,
                    entry.file_name().to_str().expect("Could not get file name").to_string(),
                    entry.file_type().unwrap().is_dir(),
                );

                sender.send(result.clone()).expect("Failed to send result");

                if entry.file_type().map_or(false, |ft| ft.is_dir()) {
                    subdirs.push(result);
                }
            }

            stack.extend(subdirs);
        }
    }

    fn is_excluded(&self, path: &str) -> bool {
        if let Some(ConfigurationValue::Array(exclude_dirs)) = self.config.entries.get("exclude_dirs") {
            for exclude_dir in exclude_dirs {
                if let ConfigurationValue::String(exclude_dir) = exclude_dir {
                    if path.contains(exclude_dir) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

fn get_config() -> &'static Configuration {
    static CONFIG: OnceLock<Configuration> = OnceLock::new();
    CONFIG.get_or_init(|| {
        let mut config = Configuration::new();
        config.init();
        config
    })
}
