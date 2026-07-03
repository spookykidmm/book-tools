use anyhow::{Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookRecord {
    pub content_hash: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub file_path: String,
    pub output_path: Option<String>,
    pub size: u64,
    pub file_type: String,
    pub last_seen: i64,
    pub status: String,
}

pub struct Registry {
    conn: Connection,
    db_path: PathBuf,
}

impl Registry {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let db_path = PathBuf::from(db_path);

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS books (
                content_hash TEXT PRIMARY KEY,
                title TEXT,
                author TEXT,
                file_path TEXT,
                output_path TEXT,
                size INTEGER,
                file_type TEXT,
                last_seen INTEGER,
                status TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_books_last_seen ON books(last_seen);
            CREATE INDEX IF NOT EXISTS idx_books_status ON books(status);
            CREATE INDEX IF NOT EXISTS idx_books_author ON books(author);
            "#,
        )?;

        Ok(Self { conn, db_path })
    }

    pub fn upsert(&self, record: &BookRecord) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO books (
                content_hash, title, author, file_path,
                output_path, size, file_type, last_seen, status
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, strftime('%s','now'), ?8)
            ON CONFLICT(content_hash) DO UPDATE SET
                title = COALESCE(?2, title),
                author = COALESCE(?3, author),
                file_path = ?4,
                output_path = COALESCE(?5, output_path),
                size = ?6,
                file_type = ?7,
                last_seen = strftime('%s','now'),
                status = ?8;
            "#,
            params![
                record.content_hash,
                record.title,
                record.author,
                record.file_path,
                record.output_path,
                record.size as i64,
                record.file_type,
                record.status,
            ],
        )?;
        Ok(())
    }

    pub fn exists(&self, hash: &str) -> Result<bool> {
        let mut stmt = self.conn.prepare("SELECT 1 FROM books WHERE content_hash = ?1 LIMIT 1")?;
        let mut rows = stmt.query(params![hash])?;
        Ok(rows.next()?.is_some())
    }

    pub fn get(&self, hash: &str) -> Result<Option<BookRecord>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT content_hash, title, author, file_path, output_path,
                   size, file_type, last_seen, status
            FROM books WHERE content_hash = ?1
            "#,
        )?;
        let mut rows = stmt.query(params![hash])?;

        if let Some(row) = rows.next()? {
            Ok(Some(BookRecord {
                content_hash: row.get(0)?,
                title: row.get(1)?,
                author: row.get(2)?,
                file_path: row.get(3)?,
                output_path: row.get(4)?,
                size: row.get(5)?,
                file_type: row.get(6)?,
                last_seen: row.get(7)?,
                status: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn mark_processed(&self, hash: &str, output_path: &str) -> Result<()> {
        self.conn.execute(
            r#"
            UPDATE books
            SET output_path = ?1, status = 'processed', last_seen = strftime('%s','now')
            WHERE content_hash = ?2
            "#,
            params![output_path, hash],
        )?;
        Ok(())
    }

    pub fn get_all(&self) -> Result<Vec<BookRecord>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT content_hash, title, author, file_path, output_path,
                   size, file_type, last_seen, status
            FROM books
            ORDER BY last_seen DESC
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(BookRecord {
                content_hash: row.get(0)?,
                title: row.get(1)?,
                author: row.get(2)?,
                file_path: row.get(3)?,
                output_path: row.get(4)?,
                size: row.get(5)?,
                file_type: row.get(6)?,
                last_seen: row.get(7)?,
                status: row.get(8)?,
            })
        })?;

        let mut records = Vec::new();
        for record in rows {
            records.push(record?);
        }
        Ok(records)
    }

    pub fn get_duplicates(&self) -> Result<Vec<(String, Vec<BookRecord>)>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT author, content_hash, title, file_path, output_path,
                   size, file_type, last_seen, status
            FROM books
            WHERE author IS NOT NULL
            ORDER BY author, title
            "#,
        )?;

        let mut groups: std::collections::HashMap<String, Vec<BookRecord>> =
            std::collections::HashMap::new();

        let rows = stmt.query_map([], |row| {
            Ok(BookRecord {
                content_hash: row.get(1)?,
                title: row.get(2)?,
                author: row.get(0)?,
                file_path: row.get(3)?,
                output_path: row.get(4)?,
                size: row.get(5)?,
                file_type: row.get(6)?,
                last_seen: row.get(7)?,
                status: row.get(8)?,
            })
        })?;

        for record in rows {
            let record = record?;
            if let Some(author) = &record.author {
                groups
                    .entry(author.clone())
                    .or_insert_with(Vec::new)
                    .push(record);
            }
        }

        let mut result = Vec::new();
        for (author, records) in groups {
            if records.len() > 1 {
                result.push((author, records));
            }
        }
        Ok(result)
    }

    pub fn count(&self) -> Result<usize> {
        let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM books")?;
        let count: usize = stmt.query_row([], |row| row.get(0))?;
        Ok(count)
    }
}
