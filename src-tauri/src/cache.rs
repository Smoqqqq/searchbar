use std::collections::HashMap;
use std::fs;
use crate::disk_parser::SearchResult;

#[derive(Clone)]
pub struct ResultCache {
    items: HashMap<String, Vec<SearchResult>>,
}

impl ResultCache {
    pub fn new() -> Self {
        let mut cache = ResultCache {
            items: HashMap::new(),
        };

        cache.unserialize();
        cache
    }

    pub fn set(&mut self, key: String, results: Vec<SearchResult>) {
        self.items.insert(key, results);
        self.serialize();
    }

    pub fn get(&self, key: String) -> Option<&Vec<SearchResult>> {
        self.items.get(&key)
    }

    pub fn serialize(&self) {
        let serialized = match serde_json::to_string(&self.items) {
            Ok(serialized) => serialized,
            Err(error) => return println!("Error serializing cache: {}", error)
        };
        fs::write("./cache.json", serialized).expect("Error writing cache to disk");
    }

    pub fn unserialize(&mut self) {
        let serialized = match fs::read_to_string("./cache.json") {
            Ok(serialized) => serialized,
            Err(_) => return
        };
        self.items = serde_json::from_str(&serialized).expect("Error deserializing cache");
    }
}