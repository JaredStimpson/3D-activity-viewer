use activity_core::Activity;
use render_core::RenderOptions;
use std::path::PathBuf;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![parse_gpx, render_gpx])
        .run(tauri::generate_context!())
        .expect("error while running Waypoint");
}
