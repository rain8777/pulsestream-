//! Provider abstraction. To add a service: implement `Provider`, register it in main.rs.
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub mod audius;
pub mod deezer;
pub mod local;
pub mod radio;
pub mod ytmusic;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Track {
    pub id: String,
    pub provider: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub art: Option<String>,
    /// Directly playable URL (e.g. Deezer 30s preview). `None` means the provider
    /// needs its own playback SDK / licensed stream.
    pub stream_url: Option<String>,
    pub duration: u32,
}

#[async_trait]
pub trait Provider: Send + Sync {
    #[allow(dead_code)]
    fn id(&self) -> &'static str;
    async fn search(&self, q: &str) -> anyhow::Result<Vec<Track>>;
}
