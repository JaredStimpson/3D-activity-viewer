use activity_core::Activity;
use map_assets::{AssetKind, GeoBounds, RegionManifest};
use render_core::{FrameEncoder, RenderOptions};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};
use tauri::{
    ipc::{InvokeBody, Request, Response},
    State,
};

#[derive(Default)]
struct RenderSessions {
    next_id: AtomicU64,
    sessions: Mutex<HashMap<String, FrameEncoder>>,
}

#[tauri::command]
fn parse_gpx(source: String) -> Result<Activity, String> {
    activity_core::parse_gpx(&source).map_err(|error| error.to_string())
}

#[tauri::command]
async fn render_gpx(
    source: String,
    output_path: String,
    options: RenderOptions,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let activity = activity_core::parse_gpx(&source).map_err(|error| error.to_string())?;
        let output = PathBuf::from(output_path);
        render_core::render_activity(&activity, &output, &options)
            .map(|path| path.display().to_string())
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
fn list_map_regions() -> Result<Vec<RegionManifest>, String> {
    let root = map_assets::maps_root().map_err(|error| error.to_string())?;
    map_assets::list_regions(&root).map_err(|error| error.to_string())
}

#[tauri::command]
fn find_map_region(bounds: GeoBounds) -> Result<Option<RegionManifest>, String> {
    let root = map_assets::maps_root().map_err(|error| error.to_string())?;
    map_assets::find_covering_region(&root, bounds).map_err(|error| error.to_string())
}

#[tauri::command]
fn read_map_range(
    region_id: String,
    kind: AssetKind,
    offset: u64,
    length: usize,
) -> Result<Response, String> {
    let root = map_assets::maps_root().map_err(|error| error.to_string())?;
    map_assets::read_asset_range(&root, &region_id, kind, offset, length)
        .map(Response::new)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn begin_map_render(
    output_path: String,
    options: RenderOptions,
    state: State<'_, RenderSessions>,
) -> Result<String, String> {
    let id = format!(
        "render-{}",
        state.next_id.fetch_add(1, Ordering::Relaxed) + 1
    );
    let encoder = render_core::begin_rgba_render(&PathBuf::from(output_path), &options)
        .map_err(|error| error.to_string())?;
    state
        .sessions
        .lock()
        .map_err(|_| "Render session state is unavailable.".to_string())?
        .insert(id.clone(), encoder);
    Ok(id)
}

#[tauri::command]
fn write_map_frame(request: Request<'_>, state: State<'_, RenderSessions>) -> Result<(), String> {
    let session_id = request
        .headers()
        .get("x-waypoint-render-session")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| "Missing render session header.".to_string())?;
    let frame_number = request
        .headers()
        .get("x-waypoint-frame-number")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or_else(|| "Missing frame number header.".to_string())?;
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err("Frame body must contain raw RGBA bytes.".into());
    };
    let mut sessions = state
        .sessions
        .lock()
        .map_err(|_| "Render session state is unavailable.".to_string())?;
    sessions
        .get_mut(session_id)
        .ok_or_else(|| "Render session was not found.".to_string())?
        .write_rgba_frame(frame_number, bytes)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn finish_map_render(
    session_id: String,
    state: State<'_, RenderSessions>,
) -> Result<String, String> {
    let encoder = state
        .sessions
        .lock()
        .map_err(|_| "Render session state is unavailable.".to_string())?
        .remove(&session_id)
        .ok_or_else(|| "Render session was not found.".to_string())?;
    encoder
        .finish()
        .map(|path| path.display().to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn cancel_map_render(session_id: String, state: State<'_, RenderSessions>) -> Result<(), String> {
    if let Some(encoder) = state
        .sessions
        .lock()
        .map_err(|_| "Render session state is unavailable.".to_string())?
        .remove(&session_id)
    {
        encoder.cancel();
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(RenderSessions::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            parse_gpx,
            render_gpx,
            list_map_regions,
            find_map_region,
            read_map_range,
            begin_map_render,
            write_map_frame,
            finish_map_render,
            cancel_map_render,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Waypoint");
}
