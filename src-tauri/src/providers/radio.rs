use super::{Provider, Track};
use async_trait::async_trait;

/// Radio Browser: free open database of live radio stations (continuous, full-length audio).
pub struct Radio(pub reqwest::Client);
const HOSTS: [&str; 3] = ["de1", "de2", "fi1"];

#[async_trait]
impl Provider for Radio {
    fn id(&self) -> &'static str { "radio" }
    async fn search(&self, q: &str) -> anyhow::Result<Vec<Track>> {
        for h in HOSTS {
            let url = format!("https://{h}.api.radio-browser.info/json/stations/search");
            let r = self.0.get(&url)
                .query(&[("name", q), ("limit", "25"), ("hidebroken", "true"), ("order", "votes"), ("reverse", "true")])
                .send().await;
            if let Ok(r) = r {
                if r.status().is_success() {
                    let v: serde_json::Value = r.json().await?;
                    return Ok(parse(&v));
                }
            }
        }
        anyhow::bail!("no radio-browser server reachable")
    }
}

fn parse(v: &serde_json::Value) -> Vec<Track> {
    let s = |x: &serde_json::Value| x.as_str().unwrap_or("").trim().to_string();
    v.as_array().cloned().unwrap_or_default().iter().filter_map(|t| {
        let url = s(&t["url_resolved"]);
        if url.is_empty() || url.contains(".m3u8") { return None; } // HLS can't play in <audio>
        let tags = s(&t["tags"]);
        let first: Vec<&str> = tags.split(',').filter(|x| !x.is_empty()).take(2).collect();
        Some(Track {
            id: s(&t["stationuuid"]),
            provider: "radio".into(),
            title: s(&t["name"]),
            artist: format!("Live radio{}{}", if first.is_empty() { "" } else { " · " }, first.join(", ")),
            album: tags,
            art: Some(s(&t["favicon"])).filter(|u| u.starts_with("http")),
            stream_url: Some(url),
            duration: 0,
        })
    }).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn keeps_playable_streams_only() {
        let v = serde_json::json!([
            {"stationuuid":"a","name":" Jazz FM ","url_resolved":"https://x/stream.mp3","tags":"jazz,smooth,chill","favicon":"https://x/i.png"},
            {"stationuuid":"b","name":"HLS","url_resolved":"https://x/live.m3u8","tags":"","favicon":""},
            {"stationuuid":"c","name":"Empty","url_resolved":"","tags":"","favicon":""}
        ]);
        let t = super::parse(&v);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].title, "Jazz FM");
        assert_eq!(t[0].artist, "Live radio · jazz, smooth");
        assert_eq!(t[0].duration, 0);
    }
}
