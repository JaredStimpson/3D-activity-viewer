use map_assets::{DownloadEvent, DownloadRequest, GeoBounds, RegionManifest};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{ipc::Channel, State};

#[derive(Default)]
struct DownloadState {
    cancellation: Mutex<Option<Arc<AtomicBool>>>,
}

#[tauri::command]
fn validate_bounds(bounds: GeoBounds) -> Result<GeoBounds, String> {
    bounds.validate().map_err(|error| error.to_string())
}

#[tauri::command]
fn estimate_download(request: DownloadRequest) -> Result<map_assets::DownloadEstimate, String> {
    let root = map_assets::maps_root().map_err(|error| error.to_string())?;
    map_assets::estimate_download(&root, &request).map_err(|error| error.to_string())
}

#[tauri::command]
async fn start_download(
    request: DownloadRequest,
    events: Channel<DownloadEvent>,
    state: State<'_, DownloadState>,
) -> Result<RegionManifest, String> {
    let root = map_assets::maps_root().map_err(|error| error.to_string())?;
    let cancellation = Arc::new(AtomicBool::new(false));
    {
        let mut active = state
            .cancellation
            .lock()
            .map_err(|_| "Download state is unavailable.")?;
        if active.is_some() {
            return Err("Another map download is already running.".into());
        }
        *active = Some(cancellation.clone());
    }
    let task = tauri::async_runtime::spawn_blocking(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| error.to_string())?;
        runtime
            .block_on(map_assets::download_region(
                &root,
                request,
                cancellation,
                |event| {
                    let _ = events.send(event);
                },
            ))
            .map_err(|error| error.to_string())
    })
    .await;
    if let Ok(mut active) = state.cancellation.lock() {
        *active = None;
    }
    task.map_err(|error| error.to_string())?
}

#[tauri::command]
fn cancel_download(state: State<'_, DownloadState>) -> Result<(), String> {
    if let Some(cancellation) = state
        .cancellation
        .lock()
        .map_err(|_| "Download state is unavailable.")?
        .as_ref()
    {
        cancellation.store(true, Ordering::Relaxed);
    }
    Ok(())
}

#[tauri::command]
fn list_local_regions() -> Result<Vec<RegionManifest>, String> {
    let root = map_assets::maps_root().map_err(|error| error.to_string())?;
    map_assets::list_regions(&root).map_err(|error| error.to_string())
}

#[tauri::command]
fn verify_local_region(region_id: String) -> Result<RegionManifest, String> {
    let root = map_assets::maps_root().map_err(|error| error.to_string())?;
    map_assets::verify_region(&root, &region_id).map_err(|error| error.to_string())
}

#[tauri::command]
fn open_maps_folder() -> Result<(), String> {
    let root = map_assets::maps_root().map_err(|error| error.to_string())?;
    map_assets::ensure_maps_layout(&root).map_err(|error| error.to_string())?;
    #[cfg(target_os = "windows")]
    std::process::Command::new("explorer.exe")
        .arg(&root)
        .spawn()
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(DownloadState::default())
        .invoke_handler(tauri::generate_handler![
            validate_bounds,
            estimate_download,
            start_download,
            cancel_download,
            list_local_regions,
            verify_local_region,
            open_maps_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Waypoint Map Downloader");
}
