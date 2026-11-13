use rusqlite::{params, Connection};
use std::sync::Mutex;

pub struct Cache(Mutex<Connection>);

impl Cache {
    pub fn open(path: &std::path::Path) -> anyhow::Result<Self> {
        let c = Connection::open(path)?;
        c.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;
             CREATE TABLE IF NOT EXISTS kv(k TEXT PRIMARY KEY, v TEXT, ts INTEGER);
             CREATE TABLE IF NOT EXISTS history(id INTEGER PRIMARY KEY, track TEXT, ts INTEGER);
             CREATE TABLE IF NOT EXISTS likes(track_key TEXT PRIMARY KEY, track TEXT);
             CREATE TABLE IF NOT EXISTS playlists(id INTEGER PRIMARY KEY, name TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS pl_items(id INTEGER PRIMARY KEY, pid INTEGER NOT NULL, track TEXT NOT NULL);
             DELETE FROM kv WHERE ts < strftime('%s','now') - 604800;
             DELETE FROM history WHERE id NOT IN (SELECT id FROM history ORDER BY id DESC LIMIT 1000);",
        )?;
        Ok(Self(Mutex::new(c)))
    }
    fn now() -> i64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
    }
    pub fn get(&self, k: &str, max_age: i64) -> Option<String> {
        let c = self.0.lock().ok()?;
        c.query_row("SELECT v FROM kv WHERE k=?1 AND ts>?2", params![k, Self::now() - max_age], |r| r.get(0)).ok()
    }
    pub fn put(&self, k: &str, v: &str) {
        if let Ok(c) = self.0.lock() {
            let _ = c.execute("INSERT OR REPLACE INTO kv VALUES(?1,?2,?3)", params![k, v, Self::now()]);
        }
    }
    pub fn add_history(&self, track_json: &str) {
        if let Ok(c) = self.0.lock() {
            let _ = c.execute("INSERT INTO history(track,ts) VALUES(?1,?2)", params![track_json, Self::now()]);
        }
    }
    pub fn recent(&self, n: i64) -> Vec<String> {
        let Ok(c) = self.0.lock() else { return vec![] };
        let Ok(mut s) = c.prepare("SELECT track FROM history ORDER BY id DESC LIMIT ?1") else { return vec![] };
        let rows = s.query_map(params![n], |r| r.get::<_, String>(0));
        rows.map(|i| i.flatten().collect()).unwrap_or_default()
    }
    pub fn toggle_like(&self, key: &str, track_json: &str) -> bool {
        let Ok(c) = self.0.lock() else { return false };
        let n = c.execute("DELETE FROM likes WHERE track_key=?1", params![key]).unwrap_or(0);
        if n == 0 {
            let _ = c.execute("INSERT INTO likes VALUES(?1,?2)", params![key, track_json]);
            true
        } else { false }
    }
    pub fn likes(&self) -> Vec<String> {
        let Ok(c) = self.0.lock() else { return vec![] };
        let Ok(mut s) = c.prepare("SELECT track FROM likes") else { return vec![] };
        let rows = s.query_map([], |r| r.get::<_, String>(0));
        rows.map(|i| i.flatten().collect()).unwrap_or_default()
    }

    pub fn pl_create(&self, name: &str) -> Option<i64> {
        let c = self.0.lock().ok()?;
        c.execute("INSERT INTO playlists(name) VALUES(?1)", params![name]).ok()?;
        Some(c.last_insert_rowid())
    }
    pub fn pl_list(&self) -> Vec<(i64, String)> {
        let Ok(c) = self.0.lock() else { return vec![] };
        let Ok(mut s) = c.prepare("SELECT id,name FROM playlists ORDER BY id") else { return vec![] };
        let rows = s.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)));
        rows.map(|i| i.flatten().collect()).unwrap_or_default()
    }
    pub fn pl_add(&self, pid: i64, track_json: &str) {
        if let Ok(c) = self.0.lock() {
            let _ = c.execute("INSERT INTO pl_items(pid,track) VALUES(?1,?2)", params![pid, track_json]);
        }
    }
    pub fn pl_tracks(&self, pid: i64) -> Vec<String> {
        let Ok(c) = self.0.lock() else { return vec![] };
        let Ok(mut s) = c.prepare("SELECT track FROM pl_items WHERE pid=?1 ORDER BY id") else { return vec![] };
        let rows = s.query_map(params![pid], |r| r.get::<_, String>(0));
        rows.map(|i| i.flatten().collect()).unwrap_or_default()
    }
    pub fn pl_delete(&self, pid: i64) {
        if let Ok(c) = self.0.lock() {
            let _ = c.execute("DELETE FROM pl_items WHERE pid=?1", params![pid]);
            let _ = c.execute("DELETE FROM playlists WHERE id=?1", params![pid]);
        }
    }
}
