use crate::configuration::{Configuration, ConfigurationValue};
use serde::{Serialize, Serializer};
use std::collections::HashMap;
use std::fs;

pub struct DiskParser {
    config: Configuration,
    results: Vec<SearchResult>,
}

#[derive(Debug)]
pub struct SearchResult {
    pub filename: String,
    pub filepath: String,
}

impl SearchResult {
    pub fn new(filename: String, filepath: String) -> Self {
        SearchResult { filename, filepath }
    }
}

impl Serialize for SearchResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map: HashMap<String, String> = HashMap::new();

        map.insert("filepath".to_string(), self.filepath.clone());
        map.insert("filename".to_string(), self.filename.clone());

        map.serialize(serializer)
    }
}

impl DiskParser {
    pub fn new() -> Self {
        let mut config = Configuration::new();
        config.init();

        DiskParser {
            config,
            results: vec![],
        }
    }

    pub fn search(&mut self, filename: String) -> String {
        let include_dirs = match self.config.entries.clone().get("include_dirs") {
            Some(include_dirs) => include_dirs.clone(),
            None => return self.search_all_disks(filename),
        };

        let include_dirs = match include_dirs {
            ConfigurationValue::Array(include_dirs) => include_dirs,
            _ => return panic!("Error parsing include_dirs"),
        };

        for include_dir in include_dirs.clone() {
            let include_dir = match include_dir.clone() {
                ConfigurationValue::String(include_dir) => include_dir,
                _ => return panic!("Error parsing include_dir"),
            };

            self.search_path(include_dir.clone(), filename.clone());
        }

        println!("Results: {:?}", self.results);

        self.results_to_json()
    }

    pub fn search_all_disks(&mut self, filename: String) -> String {
        println!("No include_dirs found in config, using all disks");

        let disk_list = disk_list::get_disk_list();

        // Loop over the disk list
        for disk in disk_list {
            let disk_letter = &disk[2];

            self.search_path(disk_letter.to_string(), filename.clone());
        }

        println!("Results: {:?}", self.results);

        self.results_to_json()
    }

    pub fn search_path(&mut self, path: String, query: String) {
        // Check that path isn't in exclude_dirs
        let exclude_dirs = match self.config.entries.clone().get("exclude_dirs") {
            Some(exclude_dirs) => exclude_dirs.clone(),
            None => return,
        };

        let exclude_dirs = match exclude_dirs {
            ConfigurationValue::Array(exclude_dirs) => exclude_dirs,
            _ => panic!("Error parsing exclude_dirs"),
        };

        for exclude_dir in exclude_dirs {
            let exclude_dir = match exclude_dir.clone() {
                ConfigurationValue::String(exclude_dir) => exclude_dir,
                _ => panic!("Error parsing exclude_dir"),
            };

            if path.contains(exclude_dir.as_str()) {
                println!("Excluding {}", path);
                return;
            }
        }

        println!("Searching in {}", path);

        let entries = match fs::read_dir(&path) {
            Ok(entries) => entries,
            Err(error) => return println!("Error reading path {} : {}", &path, error),
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };

            let path = entry.path();
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };

            let file_type = metadata.file_type();
            if file_type.is_dir() {
                self.search_path(path.display().to_string(), query.clone());
            } else if file_type.is_file() {
                let filename = entry.file_name();

                let filename = match filename.to_str() {
                    Some(filename) => filename,
                    None => continue,
                };

                if filename.contains(query.as_str()) {
                    self.results.push(SearchResult::new(
                        filename.to_string(),
                        path.display().to_string(),
                    ));
                }
            }
        }
    }

    pub fn results_to_json(&self) -> String {
        serde_json::to_string(&self.results).expect("Error converting results to JSON")
    }
}
