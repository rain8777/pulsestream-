use super::{Provider, Track};
use async_trait::async_trait;

pub struct Deezer(pub reqwest::Client);

#[async_trait]
impl Provider for Deezer {
    fn id(&self) -> &'static str { "deezer" }
    async fn search(&self, q: &str) -> anyhow::Result<Vec<Track>> {
        let v: serde_json::Value = self.0
            .get("https://api.deezer.com/search")
            .query(&[("q", q), ("limit", "25")])
            .send().await?.error_for_status()?.json().await?;
        let s = |x: &serde_json::Value| x.as_str().unwrap_or("").to_string();
        Ok(v["data"].as_array().cloned().unwrap_or_default().iter().map(|t| Track {
            id: t["id"].to_string(),
            provider: "deezer".into(),
            title: s(&t["title"]),
            artist: s(&t["artist"]["name"]),
            album: s(&t["album"]["title"]),
            art: t["album"]["cover_medium"].as_str().map(String::from),
            stream_url: t["preview"].as_str().filter(|u| !u.is_empty()).map(String::from),
            duration: t["duration"].as_u64().unwrap_or(0) as u32,
        }).collect())
    }
}
