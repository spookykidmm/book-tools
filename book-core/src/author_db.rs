use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    pub id: i64,
    pub canonical_name: String,
    pub aliases: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct AuthorDB {
    conn: Connection,
}

impl AuthorDB {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS authors (
                id INTEGER PRIMARY KEY,
                canonical_name TEXT NOT NULL UNIQUE,
                aliases TEXT,
                created_at INTEGER,
                updated_at INTEGER
            );

            CREATE TABLE IF NOT EXISTS series (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                author_id INTEGER,
                created_at INTEGER
            );

            CREATE TABLE IF NOT EXISTS works_authors (
                work_id INTEGER,
                author_id INTEGER,
                PRIMARY KEY (work_id, author_id)
            );

            CREATE INDEX IF NOT EXISTS idx_authors_name ON authors(canonical_name);
            CREATE INDEX IF NOT EXISTS idx_series_name ON series(name);
            "#,
        )?;
        Ok(Self { conn })
    }

    pub fn get_or_create_author(&self, name: &str) -> Result<Author> {
        // Normalize the name first
        let normalized = self.normalize_name(name);
        
        // Check if author exists
        let mut stmt = self.conn.prepare(
            "SELECT id, canonical_name, aliases, created_at, updated_at FROM authors WHERE canonical_name = ?1"
        )?;
        
        if let Ok(mut rows) = stmt.query(params![normalized]) {
            if let Some(row) = rows.next()? {
                let aliases: String = row.get(2)?;
                let aliases: Vec<String> = serde_json::from_str(&aliases).unwrap_or_default();
                return Ok(Author {
                    id: row.get(0)?,
                    canonical_name: row.get(1)?,
                    aliases,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                });
            }
        }

        // Create new author
        let aliases = vec![name.to_string()];
        let aliases_json = serde_json::to_string(&aliases).unwrap_or("[]".to_string());
        let now = chrono::Utc::now().timestamp();

        self.conn.execute(
            r#"
            INSERT INTO authors (canonical_name, aliases, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![normalized, aliases_json, now, now],
        )?;

        // Return the new author
        self.get_or_create_author(name)
    }

    pub fn add_alias(&self, canonical: &str, alias: &str) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "SELECT aliases FROM authors WHERE canonical_name = ?1"
        )?;
        
        let mut aliases: Vec<String> = if let Ok(mut rows) = stmt.query(params![canonical]) {
            if let Some(row) = rows.next()? {
                let aliases_str: String = row.get(0)?;
                serde_json::from_str(&aliases_str).unwrap_or_default()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        if !aliases.contains(&alias.to_string()) {
            aliases.push(alias.to_string());
            let aliases_json = serde_json::to_string(&aliases).unwrap_or("[]".to_string());
            let now = chrono::Utc::now().timestamp();

            self.conn.execute(
                r#"
                UPDATE authors
                SET aliases = ?1, updated_at = ?2
                WHERE canonical_name = ?3
                "#,
                params![aliases_json, now, canonical],
            )?;
        }

        Ok(())
    }

    pub fn get_author_by_alias(&self, alias: &str) -> Result<Option<Author>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, canonical_name, aliases, created_at, updated_at
            FROM authors
            WHERE canonical_name = ?1 OR aliases LIKE '%' || ?1 || '%'
            "#,
        )?;

        let mut rows = stmt.query(params![alias])?;
        if let Some(row) = rows.next()? {
            let aliases: String = row.get(2)?;
            let aliases: Vec<String> = serde_json::from_str(&aliases).unwrap_or_default();
            return Ok(Some(Author {
                id: row.get(0)?,
                canonical_name: row.get(1)?,
                aliases,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            }));
        }

        Ok(None)
    }

    fn normalize_name(&self, name: &str) -> String {
        let mut normalized = name.trim().to_string();

        // Remove common suffixes
        let suffixes = ["(Author)", "(Narrator)", "(editor)", "(illustrator)"];
        for suffix in suffixes {
            if normalized.ends_with(suffix) {
                normalized = normalized[..normalized.len() - suffix.len()].trim().to_string();
            }
        }

        // Fix "Last, First" → "First Last"
        if let Some(pos) = normalized.find(',') {
            let last = normalized[..pos].trim();
            let first = normalized[pos + 1..].trim();
            normalized = format!("{} {}", first, last);
        }

        // Remove extra spaces
        normalized = normalized.split_whitespace().collect::<Vec<&str>>().join(" ");

        normalized
    }
}
