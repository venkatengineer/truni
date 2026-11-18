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
pub struct UserFact {
    pub key: String,
    pub category: String,
    pub fact: String,
    pub confidence: f32,
    pub access_count: u32,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceEpisode {
    pub id: i64,
    pub task_intent: String,
    pub user_query: String,
    pub action_taken: String,
    pub outcome: String,
    pub insight: String,
    pub timestamp: DateTime<Utc>,
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

        // High efficiency pragmas: WAL mode, incremental vacuum, journal size ceiling
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA auto_vacuum = INCREMENTAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA journal_size_limit = 4194304;

            -- 1. Core Key-Value Memory
            CREATE TABLE IF NOT EXISTS memories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                key TEXT UNIQUE NOT NULL,
                category TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- 2. Learned User Knowledge & Persistent Preferences
            CREATE TABLE IF NOT EXISTS user_profile (
                key TEXT PRIMARY KEY NOT NULL,
                category TEXT NOT NULL,
                fact TEXT NOT NULL,
                confidence REAL NOT NULL DEFAULT 1.0,
                access_count INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- 3. Experiential Episodic Memory (What worked, what failed)
            CREATE TABLE IF NOT EXISTS experience_episodes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_intent TEXT NOT NULL,
                user_query TEXT NOT NULL,
                action_taken TEXT NOT NULL,
                outcome TEXT NOT NULL,
                insight TEXT NOT NULL,
                timestamp TEXT NOT NULL
            );

            -- 4. Rolling Short-Term Dialogue History (Sliding Window)
            CREATE TABLE IF NOT EXISTS interaction_turns (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TEXT NOT NULL
            );

            -- 5. Task Registry
            CREATE TABLE IF NOT EXISTS tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id TEXT UNIQUE NOT NULL,
                prompt TEXT NOT NULL,
                status TEXT NOT NULL,
                summary TEXT,
                created_at TEXT NOT NULL,
                completed_at TEXT
            );

            -- 6. Audit Trail
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

            CREATE INDEX IF NOT EXISTS idx_memories_cat ON memories(category);
            CREATE INDEX IF NOT EXISTS idx_profile_cat ON user_profile(category);
            CREATE INDEX IF NOT EXISTS idx_exp_intent ON experience_episodes(task_intent);
            CREATE INDEX IF NOT EXISTS idx_turns_time ON interaction_turns(timestamp);
            CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
            CREATE INDEX IF NOT EXISTS idx_audit_time ON audit_logs(timestamp);
            "#,
        )?;
        Ok(())
    }

    // =========================================================================
    // 1. LEARNED USER PROFILE & PREFERENCES
    // =========================================================================

    pub fn learn_fact(&self, key: &str, category: &str, fact: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            r#"
            INSERT INTO user_profile (key, category, fact, confidence, access_count, created_at, updated_at)
            VALUES (?1, ?2, ?3, 1.0, 1, ?4, ?4)
            ON CONFLICT(key) DO UPDATE SET
                category = excluded.category,
                fact = excluded.fact,
                confidence = MIN(user_profile.confidence + 0.1, 2.0),
                access_count = user_profile.access_count + 1,
                updated_at = excluded.updated_at
            "#,
            params![key, category, fact, now],
        )?;
        Ok(())
    }

    pub fn get_user_facts(&self) -> Result<Vec<UserFact>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT key, category, fact, confidence, access_count, updated_at FROM user_profile ORDER BY access_count DESC, updated_at DESC LIMIT 30")?;
        let rows = stmt.query_map([], |row| {
            let u_str: String = row.get(5)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, f32>(3)?,
                row.get::<_, u32>(4)?,
                u_str,
            ))
        })?;

        let mut facts = Vec::new();
        for r in rows {
            let (key, category, fact, confidence, access_count, u_str) = r?;
            if let Ok(u_dt) = DateTime::parse_from_rfc3339(&u_str) {
                facts.push(UserFact {
                    key,
                    category,
                    fact,
                    confidence,
                    access_count,
                    updated_at: u_dt.with_timezone(&Utc),
                });
            }
        }
        Ok(facts)
    }

    // =========================================================================
    // 2. EXPERIENTIAL MEMORY (LEARNING FROM PAST OUTCOMES)
    // =========================================================================

    pub fn record_experience(
        &self,
        task_intent: &str,
        user_query: &str,
        action_taken: &str,
        outcome: &str,
        insight: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            r#"
            INSERT INTO experience_episodes (task_intent, user_query, action_taken, outcome, insight, timestamp)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![task_intent, user_query, action_taken, outcome, insight, now],
        )?;
        Ok(())
    }

    pub fn query_relevant_experience(&self, prompt: &str) -> Result<Vec<ExperienceEpisode>> {
        let conn = self.conn.lock().unwrap();
        let pattern = format!("%{}%", prompt.split_whitespace().next().unwrap_or(""));
        let mut stmt = conn.prepare(
            r#"
            SELECT id, task_intent, user_query, action_taken, outcome, insight, timestamp
            FROM experience_episodes
            WHERE user_query LIKE ?1 OR task_intent LIKE ?1 OR insight LIKE ?1
            ORDER BY id DESC
            LIMIT 5
            "#,
        )?;
        let rows = stmt.query_map(params![pattern], |row| {
            let ts: String = row.get(6)?;
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                ts,
            ))
        })?;

        let mut eps = Vec::new();
        for r in rows {
            let (id, task_intent, user_query, action_taken, outcome, insight, ts) = r?;
            if let Ok(dt) = DateTime::parse_from_rfc3339(&ts) {
                eps.push(ExperienceEpisode {
                    id,
                    task_intent,
                    user_query,
                    action_taken,
                    outcome,
                    insight,
                    timestamp: dt.with_timezone(&Utc),
                });
            }
        }
        Ok(eps)
    }

    // =========================================================================
    // 3. ROLLING CONVERSATION TURNS
    // =========================================================================

    pub fn record_turn(&self, role: &str, content: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO interaction_turns (role, content, timestamp) VALUES (?1, ?2, ?3)",
            params![role, content, now],
        )?;
        Ok(())
    }

    // =========================================================================
    // 4. MEMORY SYNTHESIS FOR LLM CONTEXT
    // =========================================================================

    pub fn get_system_memory_context(&self, current_prompt: &str) -> String {
        let mut parts = Vec::new();

        // 1. Ingest Learned User Facts
        if let Ok(facts) = self.get_user_facts() {
            if !facts.is_empty() {
                let mut fact_lines = Vec::new();
                for f in facts.iter().take(8) {
                    fact_lines.push(format!("• {}: {}", f.category, f.fact));
                }
                parts.push(format!("User Profile & Learned Preferences:\n{}", fact_lines.join("\n")));
            }
        }

        // 2. Ingest Relevant Past Experience
        if let Ok(exps) = self.query_relevant_experience(current_prompt) {
            if !exps.is_empty() {
                let mut exp_lines = Vec::new();
                for e in exps.iter().take(3) {
                    exp_lines.push(format!("• When asked '{}': {} (Lesson: {})", e.user_query, e.outcome, e.insight));
                }
                parts.push(format!("Relevant Past Experiences:\n{}", exp_lines.join("\n")));
            }
        }

        parts.join("\n\n")
    }

    // =========================================================================
    // 5. AUTO-PRUNING & COMPACTION (MAINTAINS TINY DISK FOOTPRINT)
    // =========================================================================

    pub fn compact_and_vacuum(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // 1. Keep only top 100 recent experiences
        conn.execute(
            "DELETE FROM experience_episodes WHERE id NOT IN (SELECT id FROM experience_episodes ORDER BY id DESC LIMIT 100)",
            [],
        )?;

        // 2. Keep only top 50 recent interaction turns
        conn.execute(
            "DELETE FROM interaction_turns WHERE id NOT IN (SELECT id FROM interaction_turns ORDER BY id DESC LIMIT 50)",
            [],
        )?;

        // 3. Keep only top 50 recent tasks
        conn.execute(
            "DELETE FROM tasks WHERE id NOT IN (SELECT id FROM tasks ORDER BY id DESC LIMIT 50)",
            [],
        )?;

        // 4. Delete audit logs older than 7 days
        conn.execute(
            "DELETE FROM audit_logs WHERE datetime(timestamp) < datetime('now', '-7 days')",
            [],
        )?;

        // 5. Vacuum unallocated pages
        conn.execute_batch("PRAGMA incremental_vacuum;")?;

        Ok(())
    }

    // =========================================================================
    // 6. LEGACY COMPATIBILITY METHODS
    // =========================================================================

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

        // Test learning facts
        store.learn_fact("pref_terminal", "preference", "User prefers Kitty terminal")?;
        store.learn_fact("user_name", "identity", "User name is Venkat")?;
        let facts = store.get_user_facts()?;
        assert_eq!(facts.len(), 2);

        // Test experience recording
        store.record_experience("AppLaunch", "open terminal", "kitty", "success", "Launched Kitty terminal on desktop")?;
        let exps = store.query_relevant_experience("terminal")?;
        assert_eq!(exps.len(), 1);

        // Test memory context generation
        let ctx = store.get_system_memory_context("terminal");
        assert!(ctx.contains("Kitty terminal"));

        // Test compacting
        store.compact_and_vacuum()?;

        Ok(())
    }
}
