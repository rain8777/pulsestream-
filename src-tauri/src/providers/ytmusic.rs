use super::{Provider, Track};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashSet;

/// YouTube Music SEARCH ONLY (metadata, same web requests ytmusicapi sends).
/// Playback is done by YouTube's official embedded player in the UI (`yt:<videoId>`).
pub struct YtMusic(pub reqwest::Client);

fn today() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let z = (secs / 86400) as i64 + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}{m:02}{d:02}")
}

fn collect<'a>(v: &'a Value, out: &mut Vec<&'a Value>) {
    match v {
        Value::Object(m) => {
            if let Some(x) = m.get("musicResponsiveListItemRenderer") { out.push(x); }
            for c in m.values() { collect(c, out); }
        }
        Value::Array(a) => for c in a { collect(c, out); },
        _ => {}
    }
}

fn find_video_id(v: &Value) -> Option<String> {
    match v {
        Value::Object(m) => {
            if let Some(s) = m.get("videoId").and_then(|x| x.as_str()) { return Some(s.to_string()); }
            m.values().find_map(find_video_id)
        }
        Value::Array(a) => a.iter().find_map(find_video_id),
        _ => None,
    }
}

fn is_dur(s: &str) -> bool { s.contains(':') && s.chars().all(|c| c.is_ascii_digit() || c == ':') }

fn parse(item: &Value) -> Option<Track> {
    let id = item.pointer("/playlistItemData/videoId").and_then(|x| x.as_str()).map(String::from)
        .or_else(|| item.get("overlay").and_then(find_video_id))?;
    let title = item.pointer("/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs/0/text")?.as_str()?.to_string();
    let runs = item.pointer("/flexColumns/1/musicResponsiveListItemFlexColumnRenderer/text/runs")
        .and_then(|r| r.as_array()).cloned().unwrap_or_default();
    let mut parts: Vec<String> = runs.iter().filter_map(|r| r["text"].as_str())
        .map(|s| s.trim().to_string()).filter(|s| !s.is_empty() && s != "•").collect();
    if matches!(parts.first().map(|s| s.as_str()), Some("Song") | Some("Video")) { parts.remove(0); }
    let duration = parts.last().filter(|s| is_dur(s.as_str()))
        .map(|s| s.split(':').fold(0u32, |a, p| a * 60 + p.parse::<u32>().unwrap_or(0))).unwrap_or(0);
    let artist = parts.first().cloned().unwrap_or_default();
    let album = parts.get(1).filter(|s| !is_dur(s.as_str()) && !s.ends_with("views")).cloned().unwrap_or_default();
    let art = item.pointer("/thumbnail/musicThumbnailRenderer/thumbnail/thumbnails")
        .and_then(|t| t.as_array()).and_then(|t| t.last()).and_then(|t| t["url"].as_str()).map(String::from);
    Some(Track { stream_url: Some(format!("yt:{id}")), id, provider: "ytmusic".into(), title, artist, album, art, duration })
}

#[async_trait]
impl Provider for YtMusic {
    fn id(&self) -> &'static str { "ytmusic" }
    async fn search(&self, q: &str) -> anyhow::Result<Vec<Track>> {
        let body = json!({"context":{"client":{"clientName":"WEB_REMIX","clientVersion":format!("1.{}.01.00", today()),"hl":"en","gl":"US"}},"query":q});
        let v: Value = self.0.post("https://music.youtube.com/youtubei/v1/search?prettyPrint=false")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:88.0) Gecko/20100101 Firefox/88.0")
            .header("Origin", "https://music.youtube.com")
            .header("Cookie", "SOCS=CAI")
            .json(&body).send().await?.error_for_status()?.json().await?;
        let mut items = vec![];
        collect(&v, &mut items);
        let mut seen = HashSet::new();
        Ok(items.into_iter().filter_map(parse).filter(|t| seen.insert(t.id.clone())).take(20).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn col(runs: &[&str]) -> Value {
        json!({"musicResponsiveListItemFlexColumnRenderer":{"text":{"runs": runs.iter().map(|t| json!({"text": t})).collect::<Vec<_>>()}}})
    }
    #[test]
    fn parses_songs_and_videos_and_skips_albums() {
        let song = json!({"musicResponsiveListItemRenderer":{"playlistItemData":{"videoId":"abc123"},
            "thumbnail":{"musicThumbnailRenderer":{"thumbnail":{"thumbnails":[{"url":"u1"},{"url":"u2"}]}}},
            "flexColumns":[col(&["Wonderwall"]), col(&["Song"," • ","Oasis"," • ","Morning Glory"," • ","4:18"])]}});
        let video = json!({"musicResponsiveListItemRenderer":{"overlay":{"a":{"watchEndpoint":{"videoId":"vid999"}}},
            "flexColumns":[col(&["Live Set"]), col(&["Video"," • ","Some Artist"," • ","1.2M views"," • ","3:45"])]}});
        let album = json!({"musicResponsiveListItemRenderer":{"overlay":{"a":{"watchPlaylistEndpoint":{"playlistId":"OLAK"}}},
            "flexColumns":[col(&["Some Album"]), col(&["Album"," • ","Oasis"])]}});
        let v = json!({"contents":[{"musicShelfRenderer":{"contents":[song, video, album]}}]});
        let mut items = vec![];
        collect(&v, &mut items);
        assert_eq!(items.len(), 3);
        let t: Vec<Track> = items.into_iter().filter_map(parse).collect();
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].id.as_str(), t[0].title.as_str(), t[0].artist.as_str()), ("abc123", "Wonderwall", "Oasis"));
        assert_eq!(t[0].album, "Morning Glory");
        assert_eq!(t[0].duration, 258);
        assert_eq!(t[0].stream_url.as_deref(), Some("yt:abc123"));
        assert_eq!(t[0].art.as_deref(), Some("u2"));
        assert_eq!((t[1].artist.as_str(), t[1].album.as_str(), t[1].duration), ("Some Artist", "", 225));
    }
    #[test]
    fn client_version_date_is_sane() {
        let d = today();
        assert_eq!(d.len(), 8);
        assert!(d.starts_with("20"));
    }
}
