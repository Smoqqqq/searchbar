use crate::configuration::{Configuration, ConfigurationValue};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError};
use std::collections::HashMap;
use std::{fs, thread};
use std::cell::OnceCell;
use std::sync::OnceLock;
use std::thread::JoinHandle;
use crate::cache::ResultCache;

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub filename: String,
    pub filepath: String,
    pub filetype: String
}

impl SearchResult {
    pub fn new(filename: String, filepath: String, filetype: String) -> Self {
        SearchResult { filename, filepath, filetype }
    }
}

impl<'de> Deserialize<'de> for SearchResult {
    fn deserialize<D>(deserializer: D) -> Result<SearchResult, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize a map from the deserializer
        let map: HashMap<String, String> = HashMap::deserialize(deserializer)?;

        // Retrieve the "filename" and "filepath" fields
        let filename = map
            .get("filename")
            .ok_or_else(|| DeError::missing_field("filename"))?
            .to_string();
        let filepath = map
            .get("filepath")
            .ok_or_else(|| DeError::missing_field("filepath"))?
            .to_string();

        let filetype = map
            .get("filetype")
            .ok_or_else(|| DeError::missing_field("filetype"))?
            .to_string();

        // Return the deserialized SearchResult instance
        Ok(SearchResult { filename, filepath, filetype })
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
        map.insert("filetype".to_string(), self.filetype.clone());

        map.serialize(serializer)
    }
}

pub struct DiskParser {
    config: Configuration,
    results: Vec<SearchResult>,
    cache: ResultCache
}

static CONFIG: OnceLock<Configuration> = OnceLock::new();
static CACHE: OnceLock<ResultCache> = OnceLock::new();

fn get_config() -> &'static Configuration {
    CONFIG.get_or_init(|| {
        let mut config = Configuration::new();
        config.init();
        config
    })
}

fn get_cache() -> &'static ResultCache {
    CACHE.get_or_init(|| {
        ResultCache::new()
    })
}

impl DiskParser {
    pub fn new() -> Self {
        let config = get_config().clone();
        let cache = get_cache().clone();

        DiskParser {
            config,
            results: vec![],
            cache
        }
    }

    pub fn search(&mut self, filename: String) -> String {
        let include_dirs = match self.config.entries.clone().get("include_dirs") {
            Some(include_dirs) => include_dirs.clone(),
            None => return self.search_all_disks(filename),
        };

        let include_dirs = match include_dirs {
            ConfigurationValue::Array(include_dirs) => include_dirs,
            _ => panic!("Error parsing include_dirs"),
        };

        let paths = include_dirs.iter().map(|x| match x {
            ConfigurationValue::String(include_dir) => include_dir.clone(),
            _ => panic!("Error parsing include_dir"),
        }).collect::<Vec<String>>();

        self.results = DiskParser::search_parallel(paths, filename.clone(), self.config.clone());

        self.cache.set(filename, self.results.clone());
        self.results_to_json()
    }

    pub fn search_all_disks(&mut self, filename: String) -> String {
        println!("No include_dirs found in config, using all disks");

        let disk_list = disk_list::get_disk_list();
        let disk_list = disk_list.iter().map(|x| x[2].clone()).collect::<Vec<String>>();

        DiskParser::search_parallel(disk_list, filename.clone(), self.config.clone());

        self.cache.set(filename.clone(), self.results.clone());
        self.results_to_json()
    }

    pub fn search_path(path: String, query: String, results: &mut Vec<SearchResult>, config: Configuration) -> Vec<SearchResult> {
        // Check that path isn't in exclude_dirs
        let exclude_dirs = match config.entries.clone().get("exclude_dirs") {
            Some(exclude_dirs) => exclude_dirs.clone(),
            None => return vec![],
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
                return vec![];
            }
        }

        let entries = match fs::read_dir(&path) {
            Ok(entries) => entries,
            Err(error) => {
                println!("Error reading path {} : {}", &path, error);
                return vec![];
            },
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

            let filename = entry.file_name();

            let filename = match filename.to_str() {
                Some(filename) => filename.to_string(),
                None => continue,
            };

            let file_matches: bool = filename.to_ascii_lowercase().contains(query.as_str().to_ascii_lowercase().as_str());
            let path_name = path.display().to_string();

            let file_type = metadata.file_type();
            if file_type.is_dir() {
                if file_matches {
                    results.push(SearchResult::new(
                        filename,
                        path.display().to_string(),
                        String::from("folder")
                    ));
                }

                DiskParser::search_path(path_name, query.clone(), results, config.clone());
            } else if file_type.is_file() {
                if file_matches {
                    results.push(SearchResult::new(
                        filename,
                        path_name,
                        String::from("file")
                    ));
                }
            }
        }

        results.to_vec()
    }

    fn search_parallel(paths: Vec<String>, filename: String, config: Configuration) -> Vec<SearchResult> {
        let mut handles: Vec<JoinHandle<Vec<SearchResult>>> = vec![];

        for path in paths {
            let query = filename.clone();
            let config = config.clone();

            let handle: JoinHandle<Vec<SearchResult>> = thread::spawn(move || {
                let mut thread_results: Vec<SearchResult> = vec![];
                DiskParser::search_path(path.clone(), query, &mut thread_results, config);
                thread_results
            });

            handles.push(handle);
        }

        let mut results = vec![];

        for handle in handles {
            if let Ok(thread_results) = handle.join() {
                results.extend(thread_results);
            }
        }

        results
    }

    pub fn results_to_json(&self) -> String {
        serde_json::to_string(&self.results).expect("Error converting results to JSON")
    }
}
