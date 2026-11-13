use super::{Provider, Track};
use async_trait::async_trait;

/// Audius: open music platform, full-length streams, no API key needed.
pub struct Audius(pub reqwest::Client);

#[async_trait]
impl Provider for Audius {
    fn id(&self) -> &'static str { "audius" }
    async fn search(&self, q: &str) -> anyhow::Result<Vec<Track>> {
        let v: serde_json::Value = self.0
            .get("https://api.audius.co/v1/tracks/search")
            .query(&[("query", q), ("limit", "25"), ("app_name", "PulseStream")])
            .send().await?.error_for_status()?.json().await?;
        let s = |x: &serde_json::Value| x.as_str().unwrap_or("").to_string();
        Ok(v["data"].as_array().cloned().unwrap_or_default().iter()
            .filter(|t| t["is_streamable"].as_bool().unwrap_or(true))
            .filter_map(|t| {
                let id = t["id"].as_str()?.to_string();
                Some(Track {
                    provider: "audius".into(),
                    title: s(&t["title"]),
                    artist: s(&t["user"]["name"]),
                    album: String::new(),
                    art: t["artwork"]["480x480"].as_str().or(t["artwork"]["150x150"].as_str()).map(String::from),
                    stream_url: Some(format!("https://api.audius.co/v1/tracks/{id}/stream?app_name=PulseStream")),
                    duration: t["duration"].as_u64().unwrap_or(0) as u32,
                    id,
                })
            }).collect())
    }
}
