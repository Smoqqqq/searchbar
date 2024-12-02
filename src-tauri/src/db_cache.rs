use std::collections::HashMap;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use crate::ReturnValue;

pub struct DbCache {
    conn: Connection,
    buffer: Vec<FileSystemEntry>, // Buffer for batch inserts
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileSystemEntry {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
}

impl FileSystemEntry {
    pub fn new(path: String, name: String, is_dir: bool) -> FileSystemEntry {
        FileSystemEntry { path, name, is_dir }
    }
}

// Flush db when dropping the DbCache
impl Drop for DbCache {
    fn drop(&mut self) {
        if let Err(e) = self.flush() {
            eprintln!("Failed to flush database buffer: {}", e);
        }
    }
}

impl DbCache {
    const BUFFER_SIZE: usize = 100; // Flush when buffer reaches this size

    pub fn new() -> Self {
        let cache = DbCache {
            conn: Connection::open("cache.db").expect("Can't open cache.db"),
            buffer: Vec::with_capacity(Self::BUFFER_SIZE),
        };

        cache.create_db_if_not_exists();

        cache
    }

    pub fn create_db_if_not_exists(&self) {
        self.conn
            .execute(
                "CREATE TABLE IF NOT EXISTS cache (
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL,
                name TEXT NOT NULL,
                is_dir TINYINT(1) NOT NULL,
                UNIQUE(path, name) ON CONFLICT IGNORE
            )",
                (),
            )
            .expect("Could not create database.");

        // Create an index on the 'name' column to speed up searches
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_name ON cache (name)",
            (),
        ).expect("Failed to create index on name field.");
    }

    pub fn db_exists(&self) -> bool {
        // Check if the `cache` table contains at least one row
        self.conn
            .prepare("SELECT 1 FROM cache LIMIT 1")
            .and_then(|mut stmt| stmt.query_row([], |_| Ok(())))
            .is_ok()
    }

    pub fn store(&mut self, entry: FileSystemEntry) {
        // Check if the entry already exists in the database before adding it
        if !self.entry_exists(&entry) {
            self.buffer.push(entry);
            if self.buffer.len() >= Self::BUFFER_SIZE {
                self.flush().expect("Failed to flush entries to database");
            }
        }
    }

    fn entry_exists(&self, entry: &FileSystemEntry) -> bool {
        let mut stmt = self
            .conn
            .prepare("SELECT 1 FROM cache WHERE path = ?1 AND name = ?2 LIMIT 1")
            .expect("Failed to prepare SELECT statement");

        stmt.query_row(params![&entry.path, &entry.name], |_| Ok(()))
            .is_ok()
    }

    pub fn flush(&mut self) -> rusqlite::Result<()> {
        if self.buffer.is_empty() {
            return Ok(()); // Nothing to flush
        }

        let tx = self.conn.transaction()?; // Begin transaction
        {
            let mut stmt =
                tx.prepare("INSERT INTO cache (path, name, is_dir) VALUES (?1, ?2, ?3)")?;
            for entry in &self.buffer {
                stmt.execute(params![&entry.path, &entry.name, entry.is_dir as i32])?;
            }
        }
        tx.commit()?; // Commit transaction
        self.buffer.clear(); // Clear the buffer
        Ok(())
    }

    pub fn search(&self, name: &str, page: &u32) -> rusqlite::Result<Vec<FileSystemEntry>> {
        let name_arg = format!("%{}%", name);
        println!("Searching {}", name_arg);
        let offset = page * 15;
        let sql = "
            SELECT path, name, is_dir FROM cache
            WHERE name LIKE ?1
            ORDER BY
                CASE
                    WHEN name LIKE '%.exe' THEN 0
                    ELSE 1
                END, name
            LIMIT 15 OFFSET ?2;
        ";
        let mut stmt = self.conn.prepare(sql)?;
        let results = stmt.query_map([name_arg, offset.to_string()], |row| {
            Ok(FileSystemEntry {
                path: row.get(0)?,   // Path
                name: row.get(1)?,   // Name
                is_dir: row.get(2)?, // Is directory
            })
        })?;

        results.collect::<Result<Vec<_>, _>>()
    }

    pub fn count(&self, name: &str) -> u32 {
        let name_arg = format!("%{}%", name);
        let sql = "
            SELECT COUNT(id) FROM cache WHERE name LIKE ?1;
        ";

        let mut stmt = self.conn.prepare(sql).expect("Failed to prepare statement");
        let count: u32 = stmt
            .query_row([name_arg], |row| row.get(0))
            .expect("Failed to count files");

        count
    }

    pub fn to_json(&self, items: HashMap<&str, ReturnValue>) -> String {
        serde_json::to_string(&items).expect("Error converting results to JSON")
    }
}
