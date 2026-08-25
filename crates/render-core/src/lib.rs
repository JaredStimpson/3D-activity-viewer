use activity_core::{point_at_progress, Activity};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderOptions {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub duration_seconds: u32,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            fps: 30,
            duration_seconds: 24,
        }
    }
}

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("invalid render settings: {0}")]
    InvalidSettings(String),
    #[error("FFmpeg could not be started; install FFmpeg and make sure it is on PATH: {0}")]
    FfmpegUnavailable(#[source] std::io::Error),
    #[error("video renderer failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("FFmpeg did not complete successfully")]
    EncodingFailed,
    #[error("ffprobe could not verify the finished video")]
    VerificationFailed,
}

pub fn render_activity(
    activity: &Activity,
    output: &Path,
    options: &RenderOptions,
) -> Result<PathBuf, RenderError> {
    validate_options(options)?;
    if output.exists() {
        return Err(RenderError::InvalidSettings(
            "the destination already exists; choose a new filename".into(),
        ));
    }
    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let temporary = temporary_path(output);
    if temporary.exists() {
        fs::remove_file(&temporary)?;
    }

    let size = format!("{}x{}", options.width, options.height);
    let fps = options.fps.to_string();
    let mut child = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "rawvideo",
            "-pixel_format",
            "rgb24",
            "-video_size",
            &size,
            "-framerate",
            &fps,
            "-i",
            "-",
            "-an",
            "-c:v",
            "libx264",
            "-preset",
            "medium",
            "-crf",
            "18",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&temporary)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .map_err(RenderError::FfmpegUnavailable)?;

    let total_frames = options.fps * options.duration_seconds;
    let background = terrain_background(options.width, options.height);
    let mut stdin = child.stdin.take().expect("piped FFmpeg input");
    for frame_number in 0..total_frames {
        let time = frame_number as f64 / options.fps as f64;
        let progress = timeline_progress(time, options.duration_seconds as f64);
        let mut frame = background.clone();
        draw_activity_frame(
            &mut frame,
            options.width,
            options.height,
            activity,
            progress,
        );
        stdin.write_all(&frame)?;
    }
    drop(stdin);

    if !child.wait()?.success() {
        return Err(RenderError::EncodingFailed);
    }
    verify_video(&temporary, options)?;
    fs::rename(&temporary, output)?;
    Ok(output.to_owned())
}

pub fn timeline_progress(time: f64, duration: f64) -> f64 {
    if duration <= 0.0 {
        return 0.0;
    }
    let normalized = (time / duration).clamp(0.0, 1.0);
    let route_window = ((normalized - 0.07) / 0.83).clamp(0.0, 1.0);
    route_window * route_window * (3.0 - 2.0 * route_window)
}

fn validate_options(options: &RenderOptions) -> Result<(), RenderError> {
    if options.width < 320 || options.height < 320 || options.width > 3840 || options.height > 3840
    {
        return Err(RenderError::InvalidSettings(
            "resolution must be between 320 and 3840 pixels".into(),
        ));
    }
    if options.width % 2 != 0 || options.height % 2 != 0 {
        return Err(RenderError::InvalidSettings(
            "H.264 output dimensions must be even".into(),
        ));
    }
    if !(1..=60).contains(&options.fps) || !(1..=180).contains(&options.duration_seconds) {
        return Err(RenderError::InvalidSettings(
            "FPS or duration is outside the supported range".into(),
        ));
    }
    Ok(())
}

fn temporary_path(output: &Path) -> PathBuf {
    let stem = output
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("activity");
    output.with_file_name(format!("{stem}.rendering.tmp.mp4"))
}

fn terrain_background(width: u32, height: u32) -> Vec<u8> {
    let mut frame = vec![0_u8; width as usize * height as usize * 3];
    for y in 0..height {
        for x in 0..width {
            let nx = x as f32 / width as f32;
            let ny = y as f32 / height as f32;
            let ridge = ((nx * 19.0).sin() * 0.5
                + (ny * 13.0).cos() * 0.35
                + ((nx + ny) * 31.0).sin() * 0.15)
                * 0.5
                + 0.5;
            let contour = ((ridge * 18.0).fract() < 0.045) as u8;
            let horizon = (1.0 - ny).powf(1.5);
            let index = ((y * width + x) * 3) as usize;
            frame[index] = (13.0 + ridge * 17.0 + horizon * 20.0) as u8 + contour * 20;
            frame[index + 1] = (25.0 + ridge * 36.0 + horizon * 35.0) as u8 + contour * 23;
            frame[index + 2] = (22.0 + ridge * 25.0 + horizon * 24.0) as u8 + contour * 18;
        }
    }
    frame
}

fn draw_activity_frame(
    frame: &mut [u8],
    width: u32,
    height: u32,
    activity: &Activity,
    progress: f64,
) {
    let current = point_at_progress(activity, progress);
    let [min_lon, min_lat, max_lon, max_lat] = activity.stats.bounds;
    let lon_span = (max_lon - min_lon).max(0.000_001);
    let lat_span = (max_lat - min_lat).max(0.000_001);
    let focus_x = (current.longitude - min_lon) / lon_span;
    let focus_y = (current.latitude - min_lat) / lat_span;
    let zoom = 0.78 + (1.0 - (progress * std::f64::consts::PI).sin()) * 0.16;

    let project = |latitude: f64, longitude: f64| {
        let x = (longitude - min_lon) / lon_span;
        let y = (latitude - min_lat) / lat_span;
        let screen_x = width as f64 * (0.5 + (x - focus_x) * zoom);
        let depth = 0.9 + (y - focus_y) * 0.18;
        let screen_y = height as f64 * (0.58 - (y - focus_y) * zoom * 0.68 * depth);
        (screen_x.round() as i32, screen_y.round() as i32)
    };

    let visible_distance = activity.stats.distance_meters * progress;
    for pair in activity.points.windows(2) {
        let color = if pair[1].cumulative_distance_meters <= visible_distance {
            [207, 255, 79]
        } else {
            [67, 81, 72]
        };
        let (x0, y0) = project(pair[0].latitude, pair[0].longitude);
        let (x1, y1) = project(pair[1].latitude, pair[1].longitude);
        draw_line(frame, width, height, x0, y0, x1, y1, color, 4);
    }
    let (marker_x, marker_y) = project(current.latitude, current.longitude);
    draw_disc(frame, width, height, marker_x, marker_y, 8, [224, 255, 126]);
}

#[allow(clippy::too_many_arguments)]
fn draw_line(
    frame: &mut [u8],
    width: u32,
    height: u32,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    color: [u8; 3],
    radius: i32,
) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        draw_disc(frame, width, height, x0, y0, radius, color);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let twice = 2 * error;
        if twice >= dy {
            error += dy;
            x0 += sx;
        }
        if twice <= dx {
            error += dx;
            y0 += sy;
        }
    }
}

fn draw_disc(
    frame: &mut [u8],
    width: u32,
    height: u32,
    center_x: i32,
    center_y: i32,
    radius: i32,
    color: [u8; 3],
) {
    for y in (center_y - radius)..=(center_y + radius) {
        for x in (center_x - radius)..=(center_x + radius) {
            if x < 0
                || y < 0
                || x >= width as i32
                || y >= height as i32
                || (x - center_x).pow(2) + (y - center_y).pow(2) > radius.pow(2)
            {
                continue;
            }
            let index = ((y as u32 * width + x as u32) * 3) as usize;
            frame[index..index + 3].copy_from_slice(&color);
        }
    }
}

fn verify_video(path: &Path, options: &RenderOptions) -> Result<(), RenderError> {
    let expected = format!("{},{}", options.width, options.height);
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .output()
        .map_err(|_| RenderError::VerificationFailed)?;
    let actual = String::from_utf8_lossy(&output.stdout)
        .trim()
        .replace('\r', "");
    if !output.status.success() || actual != expected {
        return Err(RenderError::VerificationFailed);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_is_deterministic_and_bounded() {
        assert_eq!(timeline_progress(0.0, 30.0), 0.0);
        assert_eq!(timeline_progress(30.0, 30.0), 1.0);
        assert_eq!(timeline_progress(15.0, 30.0), timeline_progress(15.0, 30.0));
    }

    #[test]
    fn validates_encoder_dimensions() {
        let options = RenderOptions {
            width: 321,
            height: 320,
            fps: 30,
            duration_seconds: 1,
        };
        assert!(validate_options(&options).is_err());
    }
}
