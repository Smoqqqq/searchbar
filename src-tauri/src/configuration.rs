use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Read;

pub struct Configuration {
    pub entries: HashMap<String, ConfigurationValue>,
}

#[derive(Debug, Clone)]
pub enum ConfigurationValue {
    String(String),
    Integer(i32),
    Float(f32),
    Boolean(bool),
    HashMap(Vec<ConfigurationValue>),
    Array(Vec<ConfigurationValue>),
}

impl Configuration {
    pub fn new() -> Self {
        Configuration {
            entries: HashMap::new(),
        }
    }

    pub fn init(&mut self) {
        let config = match self.read_config_file_to_json() {
            Ok(config) => config,
            Err(error) => panic!("Error reading config file: {}", error),
        };

        // Put config values in entries
        for (key, value) in config.as_object().unwrap() {
            let entry = self.parse_value(value.clone());

            self.entries.insert(key.to_string(), entry);
        }

        dbg!(&self.entries);
    }

    pub fn read_config_file_to_json(&mut self) -> Result<Value, Box<dyn Error>> {
        let mut config_file = match File::open("./config.json") {
            Ok(file) => file,
            Err(error) => panic!(
                "No config file found, using default configuration : {}",
                error
            ),
        };

        let mut string_buffer = String::new();
        match config_file.read_to_string(&mut string_buffer) {
            Ok(_) => (),
            Err(error) => panic!(
                "Error reading config file, using default configuration : {}",
                error
            ),
        }

        dbg!(&string_buffer);

        let config: Value = serde_json::from_str(&string_buffer)?;

        Ok(config)
    }

    pub fn parse_value(&self, value: Value) -> ConfigurationValue {
        match value {
            Value::String(string) => ConfigurationValue::String(string),
            Value::Number(number) => {
                if number.is_i64() {
                    ConfigurationValue::Integer(number.as_i64().unwrap() as i32)
                } else {
                    ConfigurationValue::Float(number.as_f64().unwrap() as f32)
                }
            }
            Value::Bool(boolean) => ConfigurationValue::Boolean(boolean),
            Value::Array(array) => {
                let mut values = vec![];

                for value in array {
                    values.push(self.parse_value(value));
                }

                ConfigurationValue::Array(values)
            }
            _ => panic!("Unsupported configuration value type"),
        }
    }
}
