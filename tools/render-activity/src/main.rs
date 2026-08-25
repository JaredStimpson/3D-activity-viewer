use render_core::RenderOptions;
use std::{env, fs, path::PathBuf, process};

fn main() {
    let arguments: Vec<_> = env::args().skip(1).collect();
    if arguments.len() != 2 {
        eprintln!("Usage: render-activity <activity.gpx> <output.mp4>");
        process::exit(2);
    }
    let input = PathBuf::from(&arguments[0]);
    let output = PathBuf::from(&arguments[1]);
    let source = fs::read_to_string(&input).unwrap_or_else(|error| {
        eprintln!("Could not read {}: {error}", input.display());
        process::exit(1);
    });
    let activity = activity_core::parse_gpx(&source).unwrap_or_else(|error| {
        eprintln!("Could not parse {}: {error}", input.display());
        process::exit(1);
    });
    let options = RenderOptions::default();
    render_core::render_activity(&activity, &output, &options).unwrap_or_else(|error| {
        eprintln!("Render failed: {error}");
        process::exit(1);
    });
    println!("Rendered {}", output.display());
}
