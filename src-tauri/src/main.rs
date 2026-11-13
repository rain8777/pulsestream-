#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod cache;
mod providers;

use providers::{deezer::Deezer, Provider, Track};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as _};
use tauri_plugin_global_shortcut::{Builder as GsBuilder, GlobalShortcutExt, ShortcutState};
use tauri_plugin_notification::NotificationExt;

struct AppState {
    cache: cache::Cache,
    providers: Vec<Box<dyn Provider>>,
    http: reqwest::Client,
}

fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") { let _ = w.show(); let _ = w.unminimize(); let _ = w.set_focus(); }
}
fn js<T: serde::Serialize>(t: &T) -> String { serde_json::to_string(t).unwrap_or_default() }

#[tauri::command]
async fn search(q: String, st: tauri::State<'_, AppState>) -> Result<Vec<Track>, String> {
    let key = format!("s:{}", q.trim().to_lowercase());
    if let Some(v) = st.cache.get(&key, 600) {
        if let Ok(t) = serde_json::from_str(&v) { return Ok(t); }
    }
    // All providers in parallel; one failing provider never breaks the search.
    let results = futures::future::join_all(st.providers.iter().map(|p| p.search(&q))).await;
    let mut out = vec![];
    for r in results {
        match r { Ok(mut t) => out.append(&mut t), Err(e) => eprintln!("provider error: {e}") }
    }
    st.cache.put(&key, &js(&out));
    Ok(out)
}

#[tauri::command]
fn played(app: tauri::AppHandle, track: Track, st: tauri::State<'_, AppState>) {
    st.cache.add_history(&js(&track));
    let hidden = app.get_webview_window("main").map(|w| !w.is_visible().unwrap_or(true)).unwrap_or(false);
    if hidden {
        let _ = app.notification().builder().title(track.title.clone()).body(track.artist.clone()).show();
    }
}

#[tauri::command]
fn recent(st: tauri::State<'_, AppState>) -> Vec<String> { st.cache.recent(60) }
#[tauri::command]
fn toggle_like(track: Track, st: tauri::State<'_, AppState>) -> bool {
    st.cache.toggle_like(&format!("{}:{}", track.provider, track.id), &js(&track))
}
#[tauri::command]
fn likes(st: tauri::State<'_, AppState>) -> Vec<String> { st.cache.likes() }

#[tauri::command]
fn playlist_create(name: String, st: tauri::State<'_, AppState>) -> Result<i64, String> {
    let n = name.trim();
    if n.is_empty() { return Err("empty name".into()); }
    st.cache.pl_create(n).ok_or_else(|| "could not create playlist".to_string())
}
#[tauri::command]
fn playlists(st: tauri::State<'_, AppState>) -> Vec<(i64, String)> { st.cache.pl_list() }
#[tauri::command]
fn playlist_add(pid: i64, track: Track, st: tauri::State<'_, AppState>) { st.cache.pl_add(pid, &js(&track)); }
#[tauri::command]
fn playlist_tracks(pid: i64, st: tauri::State<'_, AppState>) -> Vec<String> { st.cache.pl_tracks(pid) }
#[tauri::command]
fn playlist_delete(pid: i64, st: tauri::State<'_, AppState>) { st.cache.pl_delete(pid); }

/// Plain-text lyrics from the free LRCLIB service (no API key). Cached for 7 days.
#[tauri::command]
async fn lyrics(artist: String, title: String, duration: u32, st: tauri::State<'_, AppState>) -> Result<Option<String>, String> {
    let key = format!("ly:{}|{}", artist.to_lowercase(), title.to_lowercase());
    if let Some(v) = st.cache.get(&key, 604800) { return Ok(if v.is_empty() { None } else { Some(v) }); }
    let d = duration.to_string();
    let r = st.http.get("https://lrclib.net/api/get")
        .query(&[("artist_name", artist.as_str()), ("track_name", title.as_str()), ("duration", d.as_str())])
        .send().await.map_err(|e| e.to_string())?;
    let text = if r.status().is_success() {
        let v: serde_json::Value = r.json().await.map_err(|e| e.to_string())?;
        v["plainLyrics"].as_str().unwrap_or("").to_string()
    } else { String::new() };
    st.cache.put(&key, &text);
    Ok(if text.is_empty() { None } else { Some(text) })
}

#[tauri::command]
fn autostart_get(app: tauri::AppHandle) -> bool { app.autolaunch().is_enabled().unwrap_or(false) }
#[tauri::command]
fn autostart_set(app: tauri::AppHandle, on: bool) -> Result<(), String> {
    let m = app.autolaunch();
    (if on { m.enable() } else { m.disable() }).map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))
        .plugin(
            GsBuilder::new()
                .with_handler(|app, sc, ev| {
                    if ev.state() == ShortcutState::Pressed {
                        let n = format!("{:?}", sc.key).to_lowercase();
                        let a = if n.contains("playpause") { "play" } else if n.contains("next") { "next" } else { "prev" };
                        let _ = app.emit("media-key", a);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let client = reqwest::Client::builder().user_agent("PulseStream/0.2 (https://github.com/pulsestream)").build()?;
            app.manage(AppState {
                cache: cache::Cache::open(&dir.join("cache.db"))?,
                providers: vec![Box::new(Deezer(client.clone()))],
                http: client,
            });
            let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let mut tray = TrayIconBuilder::new().show_menu_on_left_click(false);
            if let Some(icon) = app.default_window_icon() { tray = tray.icon(icon.clone()); }
            tray.menu(&Menu::with_items(app, &[&show, &quit])?)
                .on_menu_event(|app, e| match e.id.as_ref() {
                    "show" => show_main(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, ev| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = ev {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;
            // Individually registered, errors ignored: if another app owns a media key we still start.
            for k in ["MediaPlayPause", "MediaTrackNext", "MediaTrackPrevious"] {
                let _ = app.global_shortcut().register(k);
            }
            // Launched by auto-start: stay in the tray.
            if std::env::args().any(|a| a == "--minimized") {
                if let Some(w) = app.get_webview_window("main") { let _ = w.hide(); }
            }
            Ok(())
        })
        // Close = hide to tray, so playback continues in the background.
        .on_window_event(|w, e| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = e { api.prevent_close(); let _ = w.hide(); }
        })
        .invoke_handler(tauri::generate_handler![
            search, played, recent, toggle_like, likes, playlist_create, playlists, playlist_add,
            playlist_tracks, playlist_delete, lyrics, autostart_get, autostart_set
        ])
        .run(tauri::generate_context!())
        .expect("error while running PulseStream");
}
