use super::{Provider, Track};
use async_trait::async_trait;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub type Library = Arc<Mutex<Vec<Track>>>;
pub struct Local(pub Library);
const EXT: [&str; 7] = ["mp3", "flac", "ogg", "opus", "m4a", "aac", "wav"];

fn make(p: &Path) -> Track {
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("Unknown").to_string();
    let folder = p.parent().and_then(|d| d.file_name()).and_then(|s| s.to_str()).unwrap_or("").to_string();
    let (artist, title) = match stem.split_once(" - ") {
        Some((a, t)) => (a.trim().to_string(), t.trim().to_string()),
        None => (folder.clone(), stem.clone()),
    };
    let path = p.to_string_lossy().to_string();
    Track { stream_url: Some(format!("local:{path}")), id: path, provider: "local".into(), title, artist, album: folder, art: None, duration: 0 }
}

/// Walks the folder (max 5000 files). Names like "Artist - Title.mp3" are split; otherwise the folder name is the artist.
pub fn scan(root: &Path) -> Vec<Track> {
    let mut out = vec![];
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_dir() { stack.push(p); }
            else if p.extension().and_then(|x| x.to_str()).map(|x| EXT.contains(&x.to_lowercase().as_str())).unwrap_or(false) {
                out.push(make(&p));
                if out.len() >= 5000 { stack.clear(); break; }
            }
        }
    }
    out.sort_by_key(|t| t.title.to_lowercase());
    out
}

#[async_trait]
impl Provider for Local {
    fn id(&self) -> &'static str { "local" }
    async fn search(&self, q: &str) -> anyhow::Result<Vec<Track>> {
        let ql = q.to_lowercase();
        let r: Vec<Track> = {
            let lib = self.0.lock().map_err(|_| anyhow::anyhow!("library lock"))?;
            lib.iter().filter(|t| format!("{} {} {}", t.title, t.artist, t.album).to_lowercase().contains(&ql)).take(40).cloned().collect()
        };
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scans_and_names_tracks() {
        let root = std::env::temp_dir().join(format!("ps_scan_{}", std::process::id()));
        let sub = root.join("Some Band");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(root.join("Artist - Song One.MP3"), b"x").unwrap();
        std::fs::write(sub.join("Plain Title.flac"), b"x").unwrap();
        std::fs::write(sub.join("notes.txt"), b"x").unwrap();
        let t = scan(&root);
        assert_eq!(t.len(), 2);
        let a = t.iter().find(|x| x.title == "Song One").unwrap();
        assert_eq!(a.artist, "Artist");
        assert!(a.stream_url.as_ref().unwrap().starts_with("local:"));
        let b = t.iter().find(|x| x.title == "Plain Title").unwrap();
        assert_eq!(b.artist, "Some Band");
        std::fs::remove_dir_all(&root).ok();
    }
}
