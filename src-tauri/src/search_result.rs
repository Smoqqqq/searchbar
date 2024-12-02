use serde::{Deserializer, Serializer, de::Error as DeError};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    pub filepath: String,
    pub filetype: String,
}

impl SearchResult {
    pub fn new(filepath: String, filetype: String) -> Self {
        SearchResult {
            filepath,
            filetype,
        }
    }

    pub fn get_file_name(&self) -> String {
        self.filepath.split("\\").last().unwrap().to_owned()
    }
}