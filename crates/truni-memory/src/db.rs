use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: i64,
    pub key: String,
    pub category: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub id: i64,
    pub task_id: String,
    pub prompt: String,
    pub status: String,
    pub summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: i64,
    pub task_id: Option<String>,
    pub tool_name: String,
    pub action: String,
    pub is_destructive: bool,
    pub approved: bool,
    pub output_snippet: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone)]
pub struct MemoryStore {
    conn: Arc<Mutex<Connection>>,
}

impl MemoryStore {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create memory directory {:?}", parent))?;
        }

        let conn = Connection::open(&path)
            .with_context(|| format!("Failed to open SQLite database at {:?}", path.as_ref()))?;

        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.init_schema()?;
        Ok(store)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS memories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                key TEXT UNIQUE NOT NULL,
                category TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id TEXT UNIQUE NOT NULL,
                prompt TEXT NOT NULL,
                status TEXT NOT NULL,
                summary TEXT,
                created_at TEXT NOT NULL,
                completed_at TEXT
            );

            CREATE TABLE IF NOT EXISTS audit_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id TEXT,
                tool_name TEXT NOT NULL,
                action TEXT NOT NULL,
                is_destructive INTEGER NOT NULL,
                approved INTEGER NOT NULL,
                output_snippet TEXT,
                timestamp TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_memories_category ON memories(category);
            CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
            CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_logs(timestamp);
            "#,
        )?;
        Ok(())
    }

    pub fn set_memory(&self, key: &str, category: &str, content: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            r#"
            INSERT INTO memories (key, category, content, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?4)
            ON CONFLICT(key) DO UPDATE SET
                category = excluded.category,
                content = excluded.content,
                updated_at = excluded.updated_at
            "#,
            params![key, category, content, now],
        )?;
        Ok(())
    }

    pub fn get_memory(&self, key: &str) -> Result<Option<MemoryItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, key, category, content, created_at, updated_at FROM memories WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;

        if let Some(row) = rows.next()? {
            let created_at_str: String = row.get(4)?;
            let updated_at_str: String = row.get(5)?;
            Ok(Some(MemoryItem {
                id: row.get(0)?,
                key: row.get(1)?,
                category: row.get(2)?,
                content: row.get(3)?,
                created_at: DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&updated_at_str)?.with_timezone(&Utc),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn search_memories(&self, query: &str) -> Result<Vec<MemoryItem>> {
        let conn = self.conn.lock().unwrap();
        let pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(
            "SELECT id, key, category, content, created_at, updated_at FROM memories WHERE key LIKE ?1 OR content LIKE ?1 ORDER BY updated_at DESC LIMIT 20",
        )?;
        let rows = stmt.query_map(params![pattern], |row| {
            let created_at_str: String = row.get(4)?;
            let updated_at_str: String = row.get(5)?;
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                created_at_str,
                updated_at_str,
            ))
        })?;

        let mut items = Vec::new();
        for r in rows {
            let (id, key, category, content, c_str, u_str) = r?;
            if let (Ok(c_dt), Ok(u_dt)) = (DateTime::parse_from_rfc3339(&c_str), DateTime::parse_from_rfc3339(&u_str)) {
                items.push(MemoryItem {
                    id,
                    key,
                    category,
                    content,
                    created_at: c_dt.with_timezone(&Utc),
                    updated_at: u_dt.with_timezone(&Utc),
                });
            }
        }
        Ok(items)
    }

    pub fn delete_memory(&self, key: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM memories WHERE key = ?1", params![key])?;
        Ok(affected > 0)
    }

    pub fn record_task(&self, task_id: &str, prompt: &str, status: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            r#"
            INSERT INTO tasks (task_id, prompt, status, created_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(task_id) DO UPDATE SET
                status = excluded.status
            "#,
            params![task_id, prompt, status, now],
        )?;
        Ok(())
    }

    pub fn finish_task(&self, task_id: &str, status: &str, summary: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            r#"
            UPDATE tasks SET status = ?1, summary = ?2, completed_at = ?3 WHERE task_id = ?4
            "#,
            params![status, summary, now, task_id],
        )?;
        Ok(())
    }

    pub fn log_audit(
        &self,
        task_id: Option<&str>,
        tool_name: &str,
        action: &str,
        is_destructive: bool,
        approved: bool,
        output_snippet: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            r#"
            INSERT INTO audit_logs (task_id, tool_name, action, is_destructive, approved, output_snippet, timestamp)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                task_id,
                tool_name,
                action,
                if is_destructive { 1 } else { 0 },
                if approved { 1 } else { 0 },
                output_snippet,
                now
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_store_operations() -> Result<()> {
        let store = MemoryStore::open_in_memory()?;
        store.set_memory("project_solaris", "context", "Solaris aerospace command deck")?;

        let item = store.get_memory("project_solaris")?.expect("Item should exist");
        assert_eq!(item.category, "context");
        assert_eq!(item.content, "Solaris aerospace command deck");

        let search = store.search_memories("aerospace")?;
        assert_eq!(search.len(), 1);

        store.log_audit(
            Some("task_1"),
            "filesystem",
            "read_file /etc/os-release",
            false,
            true,
            Some("Arch Linux"),
        )?;

        Ok(())
    }
}
