use map_assets::{DownloadEvent, DownloadRequest, GeoBounds, RegionManifest};
use serde::Serialize;
use std::{
    collections::VecDeque,
    io::{Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::{ipc::Channel, State};

const DIAGNOSTICS_FIRST_PORT: u16 = 4765;
const DIAGNOSTICS_LAST_PORT: u16 = 4774;
const MAX_DIAGNOSTIC_LINES: usize = 4_000;

#[derive(Default)]
struct DownloadState {
    cancellation: Mutex<Option<Arc<AtomicBool>>>,
}

struct DiagnosticLog {
    started: Instant,
    lines: Mutex<VecDeque<String>>,
}

impl DiagnosticLog {
    fn new() -> Self {
        Self {
            started: Instant::now(),
            lines: Mutex::new(VecDeque::new()),
        }
    }

    fn record(&self, level: &str, message: impl AsRef<str>) {
        let elapsed = self.started.elapsed();
        let line = format!(
            "[+{:05}.{:03}s] {:<7} {}",
            elapsed.as_secs(),
            elapsed.subsec_millis(),
            level.to_uppercase(),
            message.as_ref()
        );
        #[cfg(not(test))]
        println!("{line}");
        if let Ok(mut lines) = self.lines.lock() {
            lines.push_back(line);
            while lines.len() > MAX_DIAGNOSTIC_LINES {
                lines.pop_front();
            }
        }
    }

    fn record_download_event(&self, event: &DownloadEvent) {
        match event {
            DownloadEvent::Diagnostic { level, message } => self.record(level, message),
            DownloadEvent::ResolvingSources { message } => self.record("stage", message),
            DownloadEvent::LayerStarted { layer, total_tiles } => self.record(
                "stage",
                format!("Started {layer} extraction for {total_tiles} tile coordinates."),
            ),
            DownloadEvent::Progress {
                layer,
                completed_tiles,
                total_tiles,
                downloaded_bytes,
            } => self.record(
                "progress",
                format!(
                    "{layer}: {completed_tiles}/{total_tiles} coordinates; {downloaded_bytes} tile bytes received."
                ),
            ),
            DownloadEvent::Verifying { layer } => {
                self.record("stage", format!("Verifying {layer} archive."));
            }
            DownloadEvent::Complete { region_path } => {
                self.record("complete", format!("Installed {region_path}."));
            }
        }
    }

    fn snapshot(&self) -> String {
        self.lines
            .lock()
            .map(|lines| lines.iter().cloned().collect::<Vec<_>>().join("\n"))
            .unwrap_or_else(|_| "Diagnostic log is temporarily unavailable.".into())
    }
}

struct DiagnosticsState {
    log: Arc<DiagnosticLog>,
    url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticsInfo {
    url: String,
    text_url: String,
}

impl DiagnosticsState {
    fn start() -> std::io::Result<Self> {
        let listener = (DIAGNOSTICS_FIRST_PORT..=DIAGNOSTICS_LAST_PORT)
            .find_map(|port| TcpListener::bind(("127.0.0.1", port)).ok())
            .map(Ok)
            .unwrap_or_else(|| TcpListener::bind(("127.0.0.1", 0)))?;
        let address = listener.local_addr()?;
        let url = format!("http://{address}/");
        let log = Arc::new(DiagnosticLog::new());
        let server_log = log.clone();
        thread::Builder::new()
            .name("waypoint-diagnostics".into())
            .spawn(move || serve_diagnostics(listener, server_log))?;
        log.record(
            "info",
            format!("Live downloader diagnostics are available at {url}"),
        );
        Ok(Self { log, url })
    }
}

fn serve_diagnostics(listener: TcpListener, log: Arc<DiagnosticLog>) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_diagnostics_request(stream, &log),
            Err(error) => log.record("warning", format!("Diagnostics connection failed: {error}")),
        }
    }
}

fn handle_diagnostics_request(mut stream: TcpStream, log: &DiagnosticLog) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let mut request = Vec::with_capacity(2048);
    let mut chunk = [0_u8; 512];
    while request.len() < 8192 {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(length) => {
                request.extend_from_slice(&chunk[..length]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            Err(_) => return,
        }
    }
    let request = String::from_utf8_lossy(&request);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/")
        .split('?')
        .next()
        .unwrap_or("/");

    match path {
        "/" => write_http_response(
            &mut stream,
            "200 OK",
            "text/html; charset=utf-8",
            diagnostics_html(),
        ),
        "/logs" => write_http_response(
            &mut stream,
            "200 OK",
            "text/plain; charset=utf-8",
            log.snapshot(),
        ),
        "/health" => write_http_response(
            &mut stream,
            "200 OK",
            "text/plain; charset=utf-8",
            "ok\n".into(),
        ),
        _ => write_http_response(
            &mut stream,
            "404 Not Found",
            "text/plain; charset=utf-8",
            "Not found.\n".into(),
        ),
    }
}

fn write_http_response(stream: &mut TcpStream, status: &str, content_type: &str, body: String) {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body.as_bytes());
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Write);
}

fn diagnostics_html() -> String {
    r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Waypoint Map Downloader Diagnostics</title>
  <style>
    :root { color-scheme: dark; font-family: "Cascadia Code", Consolas, monospace; }
    body { margin: 0; background: #080d0a; color: #dce8df; }
    header { position: sticky; top: 0; display: flex; align-items: baseline; gap: 16px; padding: 14px 18px; background: #111914f2; border-bottom: 1px solid #2c3b31; }
    h1 { margin: 0; font: 700 15px system-ui, sans-serif; color: #d5ff4f; }
    #state { color: #8fa097; font: 12px system-ui, sans-serif; }
    pre { margin: 0; padding: 18px; white-space: pre-wrap; overflow-wrap: anywhere; font-size: 12px; line-height: 1.55; }
  </style>
</head>
<body>
  <header><h1>Waypoint Map Downloader · Live diagnostics</h1><span id="state">Connecting…</span></header>
  <pre id="log">Waiting for log output…</pre>
  <script>
    const output = document.getElementById('log');
    const state = document.getElementById('state');
    async function refresh() {
      const follow = window.innerHeight + window.scrollY >= document.body.scrollHeight - 80;
      try {
        const response = await fetch('/logs', { cache: 'no-store' });
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        output.textContent = await response.text() || 'Waiting for log output…';
        state.textContent = `Live · refreshed ${new Date().toLocaleTimeString()}`;
        if (follow) window.scrollTo(0, document.body.scrollHeight);
      } catch (error) {
        state.textContent = `Disconnected · ${error}`;
      }
    }
    refresh();
    setInterval(refresh, 1000);
  </script>
</body>
</html>"#
        .into()
}

#[tauri::command]
fn validate_bounds(
    bounds: GeoBounds,
    diagnostics: State<'_, DiagnosticsState>,
) -> Result<GeoBounds, String> {
    diagnostics.log.record("info", "Validating entered bounds.");
    bounds.validate().map_err(|error| {
        diagnostics
            .log
            .record("error", format!("Bounds validation failed: {error}"));
        error.to_string()
    })
}

#[tauri::command]
fn estimate_download(
    request: DownloadRequest,
    diagnostics: State<'_, DiagnosticsState>,
) -> Result<map_assets::DownloadEstimate, String> {
    diagnostics.log.record(
        "info",
        format!("Estimating download for '{}'.", request.name),
    );
    let root = map_assets::maps_root().map_err(|error| error.to_string())?;
    map_assets::estimate_download(&root, &request)
        .inspect(|estimate| {
            diagnostics.log.record(
                "info",
                format!(
                    "Estimate complete: {} possible tiles, approximately {} bytes.",
                    estimate.basemap_tiles + estimate.terrain_tiles,
                    estimate.total_bytes
                ),
            );
        })
        .map_err(|error| {
            diagnostics
                .log
                .record("error", format!("Download estimate failed: {error}"));
            error.to_string()
        })
}

#[tauri::command]
async fn start_download(
    request: DownloadRequest,
    events: Channel<DownloadEvent>,
    state: State<'_, DownloadState>,
    diagnostics: State<'_, DiagnosticsState>,
) -> Result<RegionManifest, String> {
    diagnostics.log.record(
        "info",
        format!("Download requested for '{}'.", request.name),
    );
    let root = map_assets::maps_root().map_err(|error| error.to_string())?;
    let cancellation = Arc::new(AtomicBool::new(false));
    {
        let mut active = state
            .cancellation
            .lock()
            .map_err(|_| "Download state is unavailable.")?;
        if active.is_some() {
            diagnostics.log.record(
                "warning",
                "Rejected download because another job is active.",
            );
            return Err("Another map download is already running.".into());
        }
        *active = Some(cancellation.clone());
    }
    let task_log = diagnostics.log.clone();
    let task = tauri::async_runtime::spawn_blocking(move || {
        task_log.record("info", "Starting the background download runtime.");
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
                    task_log.record_download_event(&event);
                    let _ = events.send(event);
                },
            ))
            .map_err(|error| error.to_string())
    })
    .await;
    if let Ok(mut active) = state.cancellation.lock() {
        *active = None;
    }
    let result = task.map_err(|error| error.to_string())?;
    match &result {
        Ok(manifest) => diagnostics.log.record(
            "complete",
            format!(
                "Download job finished successfully as region {}.",
                manifest.id
            ),
        ),
        Err(error) => diagnostics
            .log
            .record("error", format!("Download job failed: {error}")),
    }
    result
}

#[tauri::command]
fn cancel_download(
    state: State<'_, DownloadState>,
    diagnostics: State<'_, DiagnosticsState>,
) -> Result<(), String> {
    diagnostics.log.record("warning", "Cancellation requested.");
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
fn get_diagnostics_info(diagnostics: State<'_, DiagnosticsState>) -> DiagnosticsInfo {
    DiagnosticsInfo {
        url: diagnostics.url.clone(),
        text_url: format!("{}logs", diagnostics.url),
    }
}

#[tauri::command]
fn open_diagnostics(diagnostics: State<'_, DiagnosticsState>) -> Result<(), String> {
    diagnostics.log.record(
        "info",
        "Opening the live diagnostics page in the default browser.",
    );
    #[cfg(target_os = "windows")]
    std::process::Command::new("explorer.exe")
        .arg(&diagnostics.url)
        .spawn()
        .map_err(|error| error.to_string())?;
    Ok(())
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
    let diagnostics = DiagnosticsState::start().expect("could not start local diagnostics server");
    tauri::Builder::default()
        .manage(DownloadState::default())
        .manage(diagnostics)
        .invoke_handler(tauri::generate_handler![
            validate_bounds,
            estimate_download,
            start_download,
            cancel_download,
            list_local_regions,
            verify_local_region,
            open_maps_folder,
            get_diagnostics_info,
            open_diagnostics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Waypoint Map Downloader");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_log_is_timestamped_and_bounded() {
        let log = DiagnosticLog::new();
        for index in 0..=MAX_DIAGNOSTIC_LINES {
            log.record("info", format!("entry {index}"));
        }
        let snapshot = log.snapshot();
        assert!(!snapshot.contains("entry 0\n"));
        assert!(snapshot.contains(&format!("entry {MAX_DIAGNOSTIC_LINES}")));
        assert!(snapshot.starts_with("[+"));
    }

    #[test]
    fn diagnostics_page_refreshes_plain_text_log() {
        let html = diagnostics_html();
        assert!(html.contains("fetch('/logs'"));
        assert!(html.contains("setInterval(refresh, 1000)"));
    }

    #[test]
    fn diagnostics_server_exposes_health_and_log_endpoints() {
        let diagnostics = DiagnosticsState::start().unwrap();
        diagnostics.log.record("test", "endpoint marker");
        let address = diagnostics
            .url
            .strip_prefix("http://")
            .and_then(|value| value.strip_suffix('/'))
            .unwrap();

        let request = |path: &str| {
            let mut stream = TcpStream::connect(address).unwrap();
            write!(stream, "GET {path} HTTP/1.1\r\nHost: {address}\r\n\r\n").unwrap();
            stream.flush().unwrap();
            let mut response = Vec::new();
            let mut chunk = [0_u8; 1024];
            loop {
                match stream.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(length) => response.extend_from_slice(&chunk[..length]),
                    Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => break,
                    Err(error) => panic!("could not read diagnostics response: {error}"),
                }
            }
            String::from_utf8(response).unwrap()
        };

        let health = request("/health");
        assert!(
            health.ends_with("ok\n"),
            "unexpected health response: {health:?}"
        );
        let logs = request("/logs");
        assert!(
            logs.contains("TEST    endpoint marker"),
            "unexpected log response: {logs:?}"
        );
    }
}
