use std::path::Path;

use rusqlite::{params, Connection};

use super::store::TaskGraphStore;
use super::types::*;

const SCHEMA_VERSION: u32 = 1;

pub(crate) fn init_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS meta (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS tasks (
            task_id                 TEXT PRIMARY KEY,
            project_root            TEXT NOT NULL,
            task_worktree_root      TEXT NOT NULL,
            title                   TEXT NOT NULL,
            status                  TEXT NOT NULL,
            lead_session_id         TEXT,
            current_coder_session_id TEXT,
            lead_provider           TEXT NOT NULL,
            coder_provider          TEXT NOT NULL,
            created_at              INTEGER NOT NULL,
            updated_at              INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sessions (
            session_id          TEXT PRIMARY KEY,
            task_id             TEXT NOT NULL,
            parent_session_id   TEXT,
            provider            TEXT NOT NULL,
            role                TEXT NOT NULL,
            external_session_id TEXT,
            transcript_path     TEXT,
            agent_id            TEXT,
            status              TEXT NOT NULL,
            cwd                 TEXT NOT NULL,
            title               TEXT NOT NULL,
            created_at          INTEGER NOT NULL,
            updated_at          INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS artifacts (
            artifact_id TEXT PRIMARY KEY,
            task_id     TEXT NOT NULL,
            session_id  TEXT NOT NULL,
            kind        TEXT NOT NULL,
            title       TEXT NOT NULL,
            content_ref TEXT NOT NULL,
            created_at  INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS task_agents (
            agent_id     TEXT PRIMARY KEY,
            task_id      TEXT NOT NULL,
            provider     TEXT NOT NULL,
            role         TEXT NOT NULL,
            display_name TEXT,
            sort_order   INTEGER NOT NULL,
            created_at   INTEGER NOT NULL,
            updated_at   INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS buffered_messages (
            id      INTEGER PRIMARY KEY AUTOINCREMENT,
            payload TEXT NOT NULL
        );",
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_version', ?1)",
        params![SCHEMA_VERSION.to_string()],
    )?;
    Ok(())
}

fn status_str(s: TaskStatus) -> &'static str {
    match s {
        TaskStatus::Draft => "draft",
        TaskStatus::Planning => "planning",
        TaskStatus::Implementing => "implementing",
        TaskStatus::Reviewing => "reviewing",
        TaskStatus::Done => "done",
        TaskStatus::Error => "error",
    }
}

fn parse_status(s: &str) -> TaskStatus {
    match s {
        "planning" => TaskStatus::Planning,
        "implementing" => TaskStatus::Implementing,
        "reviewing" => TaskStatus::Reviewing,
        "done" => TaskStatus::Done,
        "error" => TaskStatus::Error,
        _ => TaskStatus::Draft,
    }
}

fn provider_str(p: Provider) -> &'static str {
    match p {
        Provider::Claude => "claude",
        Provider::Codex => "codex",
    }
}

fn parse_provider(s: &str) -> Provider {
    if s == "codex" { Provider::Codex } else { Provider::Claude }
}

fn session_status_str(s: SessionStatus) -> &'static str {
    match s {
        SessionStatus::Active => "active",
        SessionStatus::Paused => "paused",
        SessionStatus::Completed => "completed",
        SessionStatus::Error => "error",
    }
}

fn parse_session_status(s: &str) -> SessionStatus {
    match s {
        "paused" => SessionStatus::Paused,
        "completed" => SessionStatus::Completed,
        "error" => SessionStatus::Error,
        _ => SessionStatus::Active,
    }
}

fn session_role_str(r: SessionRole) -> &'static str {
    match r {
        SessionRole::Lead => "lead",
        SessionRole::Coder => "coder",
    }
}

fn parse_session_role(s: &str) -> SessionRole {
    if s == "coder" { SessionRole::Coder } else { SessionRole::Lead }
}

fn artifact_kind_str(k: ArtifactKind) -> &'static str {
    match k {
        ArtifactKind::Research => "research",
        ArtifactKind::Plan => "plan",
        ArtifactKind::Review => "review",
        ArtifactKind::Diff => "diff",
        ArtifactKind::Verification => "verification",
        ArtifactKind::Summary => "summary",
    }
}

fn parse_artifact_kind(s: &str) -> ArtifactKind {
    match s {
        "plan" => ArtifactKind::Plan,
        "review" => ArtifactKind::Review,
        "diff" => ArtifactKind::Diff,
        "verification" => ArtifactKind::Verification,
        "summary" => ArtifactKind::Summary,
        _ => ArtifactKind::Research,
    }
}

impl TaskGraphStore {
    /// Open or create a SQLite-backed store at the given path.
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")?;
        init_schema(&conn)?;
        let mut store = Self::load_from_db(&conn)?;
        store.db_path = Some(path.to_path_buf());
        store.db = Some(std::sync::Mutex::new(conn));
        store.migrate_legacy_agents();
        Ok(store)
    }

    /// Persist all in-memory state to the SQLite database.
    /// No-op if no database connection is configured.
    pub fn save(&self) -> anyhow::Result<()> {
        let Some(db) = &self.db else { return Ok(()) };
        let conn = db.lock().map_err(|e| anyhow::anyhow!("db mutex poisoned: {e}"))?;
        self.save_to_db(&conn)
    }

    fn save_to_db(&self, conn: &Connection) -> anyhow::Result<()> {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch("DELETE FROM tasks; DELETE FROM sessions; DELETE FROM artifacts; DELETE FROM task_agents;")?;
        for t in self.tasks.values() {
            tx.execute(
                "INSERT INTO tasks VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![
                    t.task_id, t.project_root, t.task_worktree_root, t.title,
                    status_str(t.status), t.lead_session_id, t.current_coder_session_id,
                    provider_str(t.lead_provider), provider_str(t.coder_provider),
                    t.created_at as i64, t.updated_at as i64,
                ],
            )?;
        }
        for s in self.sessions.values() {
            tx.execute(
                "INSERT INTO sessions VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
                params![
                    s.session_id, s.task_id, s.parent_session_id, provider_str(s.provider),
                    session_role_str(s.role), s.external_session_id, s.transcript_path,
                    s.agent_id, session_status_str(s.status), s.cwd, s.title,
                    s.created_at as i64, s.updated_at as i64,
                ],
            )?;
        }
        for a in self.artifacts.values() {
            tx.execute(
                "INSERT INTO artifacts VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![
                    a.artifact_id, a.task_id, a.session_id,
                    artifact_kind_str(a.kind), a.title, a.content_ref, a.created_at as i64,
                ],
            )?;
        }
        for ag in self.task_agents.values() {
            tx.execute(
                "INSERT INTO task_agents VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![
                    ag.agent_id, ag.task_id, provider_str(ag.provider), ag.role,
                    ag.display_name, ag.order as i64, ag.created_at as i64, ag.updated_at as i64,
                ],
            )?;
        }
        tx.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('next_id', ?1)",
            params![self.next_id.to_string()],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn load_from_db(conn: &Connection) -> anyhow::Result<Self> {
        let mut store = Self::new();
        // next_id
        if let Ok(val) = conn.query_row(
            "SELECT value FROM meta WHERE key = 'next_id'", [], |r| r.get::<_, String>(0),
        ) {
            store.next_id = val.parse().unwrap_or(0);
        }
        // tasks
        let mut stmt = conn.prepare(
            "SELECT task_id, project_root, task_worktree_root, title, status,
                    lead_session_id, current_coder_session_id,
                    lead_provider, coder_provider, created_at, updated_at
             FROM tasks"
        )?;
        let tasks = stmt.query_map([], |row| {
            Ok(Task {
                task_id: row.get(0)?,
                project_root: row.get(1)?,
                task_worktree_root: row.get(2)?,
                title: row.get(3)?,
                status: parse_status(&row.get::<_, String>(4)?),
                lead_session_id: row.get(5)?,
                current_coder_session_id: row.get(6)?,
                lead_provider: parse_provider(&row.get::<_, String>(7)?),
                coder_provider: parse_provider(&row.get::<_, String>(8)?),
                created_at: row.get::<_, i64>(9)? as u64,
                updated_at: row.get::<_, i64>(10)? as u64,
            })
        })?;
        for t in tasks { let t = t?; store.tasks.insert(t.task_id.clone(), t); }
        // sessions
        let mut stmt = conn.prepare(
            "SELECT session_id, task_id, parent_session_id, provider, role,
                    external_session_id, transcript_path, agent_id, status,
                    cwd, title, created_at, updated_at
             FROM sessions"
        )?;
        let sessions = stmt.query_map([], |row| {
            Ok(SessionHandle {
                session_id: row.get(0)?,
                task_id: row.get(1)?,
                parent_session_id: row.get(2)?,
                provider: parse_provider(&row.get::<_, String>(3)?),
                role: parse_session_role(&row.get::<_, String>(4)?),
                external_session_id: row.get(5)?,
                transcript_path: row.get(6)?,
                agent_id: row.get(7)?,
                status: parse_session_status(&row.get::<_, String>(8)?),
                cwd: row.get(9)?,
                title: row.get(10)?,
                created_at: row.get::<_, i64>(11)? as u64,
                updated_at: row.get::<_, i64>(12)? as u64,
            })
        })?;
        for s in sessions { let s = s?; store.sessions.insert(s.session_id.clone(), s); }
        // artifacts
        let mut stmt = conn.prepare(
            "SELECT artifact_id, task_id, session_id, kind, title, content_ref, created_at
             FROM artifacts"
        )?;
        let arts = stmt.query_map([], |row| {
            Ok(Artifact {
                artifact_id: row.get(0)?,
                task_id: row.get(1)?,
                session_id: row.get(2)?,
                kind: parse_artifact_kind(&row.get::<_, String>(3)?),
                title: row.get(4)?,
                content_ref: row.get(5)?,
                created_at: row.get::<_, i64>(6)? as u64,
            })
        })?;
        for a in arts { let a = a?; store.artifacts.insert(a.artifact_id.clone(), a); }
        // task_agents
        let mut stmt = conn.prepare(
            "SELECT agent_id, task_id, provider, role, display_name,
                    sort_order, created_at, updated_at
             FROM task_agents"
        )?;
        let agents = stmt.query_map([], |row| {
            Ok(TaskAgent {
                agent_id: row.get(0)?,
                task_id: row.get(1)?,
                provider: parse_provider(&row.get::<_, String>(2)?),
                role: row.get(3)?,
                display_name: row.get(4)?,
                order: row.get::<_, i64>(5)? as u32,
                created_at: row.get::<_, i64>(6)? as u64,
                updated_at: row.get::<_, i64>(7)? as u64,
            })
        })?;
        for ag in agents { let ag = ag?; store.task_agents.insert(ag.agent_id.clone(), ag); }
        Ok(store)
    }
}
