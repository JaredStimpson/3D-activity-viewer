import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import {
  Activity,
  Camera,
  ChevronDown,
  CircleCheck,
  Download,
  Film,
  FolderOpen,
  Gauge,
  HardDrive,
  Image,
  Layers3,
  Map,
  MapPin,
  Mountain,
  Pause,
  Play,
  Route,
  Settings2,
  ShieldCheck,
  SlidersHorizontal,
  Sparkles,
  Upload,
} from "lucide-react";
import { RoutePreview, type RoutePreviewHandle } from "./components/RoutePreview";
import { formatDistance, formatDuration, parseGpxLocally } from "./lib/activity";
import { timelineProgress } from "./lib/scene";
import type { Activity as ActivityModel, AspectRatio, CameraPreset, ExportOptions, MapRegion, StylePreset } from "./types";

const sampleActivity: ActivityModel = {
  name: "Big Sur Ridge Ride",
  points: [
    [36.270, -121.807, 41], [36.274, -121.801, 76], [36.279, -121.797, 109],
    [36.285, -121.792, 152], [36.290, -121.785, 205], [36.296, -121.779, 266],
    [36.304, -121.773, 333], [36.313, -121.766, 401], [36.321, -121.758, 465],
    [36.327, -121.749, 512], [36.331, -121.739, 548], [36.329, -121.729, 533],
    [36.324, -121.719, 501], [36.317, -121.710, 462], [36.309, -121.702, 425],
    [36.300, -121.697, 376], [36.292, -121.694, 310], [36.284, -121.697, 242],
    [36.278, -121.703, 179], [36.273, -121.711, 120], [36.269, -121.720, 73],
  ].map(([latitude, longitude, elevation], index, all) => ({
    latitude,
    longitude,
    elevation,
    timestamp: new Date(Date.UTC(2026, 4, 18, 15, index * 7)).toISOString(),
    cumulativeDistanceMeters: index * (25760 / (all.length - 1)),
  })),
  stats: {
    distanceMeters: 25760,
    elevationGainMeters: 812,
    elevationLossMeters: 780,
    durationSeconds: 2 * 3600 + 18 * 60,
    minElevationMeters: 41,
    maxElevationMeters: 548,
    bounds: [-121.807, 36.269, -121.694, 36.331],
  },
};

const sideItems = [
  [Activity, "Activity"], [Image, "Media"], [Map, "Map"], [Route, "Route"],
  [Camera, "Camera"], [Gauge, "Stats"], [Film, "Video"], [Download, "Export"],
] as const;

function isTauri() {
  return "__TAURI_INTERNALS__" in window;
}

export default function App() {
  const [activity, setActivity] = useState(sampleActivity);
  const [gpxSource, setGpxSource] = useState<string | null>(null);
  const [mapRegion, setMapRegion] = useState<MapRegion | null>(null);
  const [activePanel, setActivePanel] = useState("Activity");
  const [playing, setPlaying] = useState(true);
  const [progress, setProgress] = useState(0.36);
  const [stylePreset, setStylePreset] = useState<StylePreset>("outdoor");
  const [cameraPreset, setCameraPreset] = useState<CameraPreset>("follow");
  const [terrainExaggeration, setTerrainExaggeration] = useState(1.2);
  const [aspectRatio, setAspectRatio] = useState<AspectRatio>("16:9");
  const [durationSeconds, setDurationSeconds] = useState(32);
  const [exportState, setExportState] = useState<"idle" | "rendering" | "done" | "error">("idle");
  const [message, setMessage] = useState("All processing stays on this computer");
  const inputRef = useRef<HTMLInputElement>(null);
  const previewRef = useRef<RoutePreviewHandle>(null);
  const lastFrame = useRef(performance.now());

  async function refreshMapRegion(targetActivity = activity) {
    if (!isTauri()) return;
    try {
      const region = await invoke<MapRegion | null>("find_map_region", { bounds: targetActivity.stats.bounds });
      setMapRegion(region);
      setMessage(region
        ? `Using verified local map region: ${region.name}`
        : `Map data required for ${targetActivity.stats.bounds.join(",")}. Open Waypoint Map Downloader, then refresh Map data.`);
    } catch (error) {
      setMapRegion(null);
      setMessage(`Could not scan local maps: ${String(error)}`);
    }
  }

  useEffect(() => {
    void refreshMapRegion(activity);
  }, [activity]);

  useEffect(() => {
    if (!playing) return;
    let frame = 0;
    const tick = (now: number) => {
      const delta = (now - lastFrame.current) / 1000;
      lastFrame.current = now;
      setProgress((value) => (value + delta / durationSeconds) % 1);
      frame = requestAnimationFrame(tick);
    };
    lastFrame.current = performance.now();
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [durationSeconds, playing]);

  async function importGpx(file: File) {
    const source = await file.text();
    try {
      const parsed = isTauri()
        ? await invoke<ActivityModel>("parse_gpx", { source })
        : parseGpxLocally(source);
      setActivity(parsed);
      setGpxSource(source);
      setProgress(0);
      await refreshMapRegion(parsed);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  }

  async function exportVideo() {
    if (!isTauri()) {
      setMessage("Video export is available in the installed desktop app.");
      return;
    }
    if (!gpxSource) {
      setMessage("Import a GPX file before exporting a video.");
      return;
    }
    if (!mapRegion || !previewRef.current) {
      setMessage(`Download map data covering ${activity.stats.bounds.join(",")} before exporting.`);
      return;
    }
    const dimensions: Record<AspectRatio, [number, number]> = {
      "16:9": [1920, 1080], "9:16": [1080, 1920], "1:1": [1080, 1080],
    };
    const destination = await save({
      title: "Export activity video",
      defaultPath: `${activity.name.replace(/[^a-z0-9]+/gi, "-")}_Landscape_1080p.mp4`,
      filters: [{ name: "MP4 Video", extensions: ["mp4"] }],
    });
    if (!destination) return;
    const options: ExportOptions = {
      width: dimensions[aspectRatio][0], height: dimensions[aspectRatio][1], fps: 30, durationSeconds,
    };
    setExportState("rendering");
    setMessage("Rendering the offline MapLibre scene with FFmpeg…");
    let sessionId: string | null = null;
    try {
      sessionId = await invoke<string>("begin_map_render", { outputPath: destination, options });
      const frameCount = options.fps * options.durationSeconds;
      for (let frame = 0; frame < frameCount; frame += 1) {
        const frameBytes = await previewRef.current.captureFrame(
          timelineProgress(frame, options.fps, options.durationSeconds),
          options.width,
          options.height,
        );
        await invoke("write_map_frame", frameBytes, {
          headers: {
            "x-waypoint-render-session": sessionId,
            "x-waypoint-frame-number": String(frame),
          },
        });
        if (frame % options.fps === 0) {
          setMessage(`Rendering frame ${frame + 1} of ${frameCount}…`);
        }
      }
      const result = await invoke<string>("finish_map_render", { sessionId });
      sessionId = null;
      setExportState("done");
      setMessage(`Export complete: ${result}`);
    } catch (error) {
      if (sessionId) await invoke("cancel_map_render", { sessionId }).catch(() => undefined);
      setExportState("error");
      setMessage(String(error));
    } finally {
      previewRef.current?.disposeExport();
    }
  }

  const completion = Math.round(progress * 100);

  return (
    <main className="app-shell">
      <header className="topbar">
        <div className="brand"><span className="brand-mark"><Mountain size={18} /></span><span>Waypoint</span></div>
        <button className="project-switcher">{activity.name}<ChevronDown size={14} /></button>
        <div className="topbar-actions">
          <span className="local-badge"><ShieldCheck size={13} /> Local only</span>
          <button className="icon-button" aria-label="Settings"><Settings2 size={18} /></button>
          <button className="secondary-button" onClick={() => inputRef.current?.click()}><Upload size={15} /> Import GPX</button>
          <button className="primary-button" onClick={exportVideo} disabled={exportState === "rendering"}>
            {exportState === "rendering" ? <Sparkles className="spin" size={15} /> : <Download size={15} />}
            {exportState === "rendering" ? "Rendering" : "Export video"}
          </button>
          <input ref={inputRef} className="visually-hidden" type="file" accept=".gpx,application/gpx+xml" onChange={(event) => {
            const file = event.target.files?.[0];
            if (file) void importGpx(file);
          }} />
        </div>
      </header>

      <aside className="sidebar">
        <div className="sidebar-heading">PROJECT</div>
        {sideItems.map(([Icon, label]) => (
          <button key={label} className={activePanel === label ? "side-item active" : "side-item"} onClick={() => setActivePanel(label)}>
            <Icon size={17} /><span>{label}</span>{label === "Activity" && <CircleCheck size={13} className="item-check" />}
          </button>
        ))}
        <div className="sidebar-spacer" />
        <button className="side-item" onClick={() => void refreshMapRegion()}><HardDrive size={17} /><span>Refresh map data</span></button>
        <button className="side-item"><FolderOpen size={17} /><span>Projects</span></button>
      </aside>

      <section className="workspace">
        <div className={`preview-frame ratio-${aspectRatio.replace(":", "-")}`}>
          <RoutePreview ref={previewRef} activity={activity} region={mapRegion} progress={progress} stylePreset={stylePreset} cameraPreset={cameraPreset} terrainExaggeration={terrainExaggeration} />
          <div className="preview-wash" />
          <div className="preview-title"><span>RIDGE SERIES 01</span><strong>{activity.name}</strong><small>California Central Coast</small></div>
          <div className="preview-stats">
            <div><strong>{formatDistance(activity.stats.distanceMeters)}</strong><span>DISTANCE</span></div>
            <div><strong>{Math.round(activity.stats.elevationGainMeters).toLocaleString()} m</strong><span>ELEVATION</span></div>
            <div><strong>{formatDuration(activity.stats.durationSeconds)}</strong><span>TIME</span></div>
          </div>
          <span className="preview-mode"><Layers3 size={12} /> {mapRegion ? "OFFLINE PMTILES" : "MAP DATA NEEDED"}</span>
        </div>
        <div className="transport">
          <button className="play-button" onClick={() => setPlaying((value) => !value)} aria-label={playing ? "Pause" : "Play"}>
            {playing ? <Pause size={16} fill="currentColor" /> : <Play size={16} fill="currentColor" />}
          </button>
          <span className="timecode">00:{String(Math.floor(progress * durationSeconds)).padStart(2, "0")}</span>
          <input className="scrubber" type="range" min="0" max="1" step="0.001" value={progress} onChange={(event) => setProgress(Number(event.target.value))} aria-label="Video position" />
          <span className="timecode muted">00:{String(durationSeconds).padStart(2, "0")}</span>
          <span className="quality">1080p · 30 fps</span>
        </div>
      </section>

      <aside className="inspector">
        <div className="inspector-title"><div><span>INSPECTOR</span><h2>{activePanel}</h2></div><SlidersHorizontal size={17} /></div>
        <section className="panel-section">
          <label>Map style</label>
          <div className="segmented three">
            {(["outdoor", "dark", "topographic"] as StylePreset[]).map((style) => (
              <button key={style} onClick={() => setStylePreset(style)} className={stylePreset === style ? "selected" : ""}>{style === "topographic" ? "Topo" : style}</button>
            ))}
          </div>
        </section>
        <section className="panel-section">
          <label>Camera</label>
          <select value={cameraPreset} onChange={(event) => setCameraPreset(event.target.value as CameraPreset)}>
            <option value="follow">Smooth follow</option>
            <option value="cinematic">Cinematic chase</option>
            <option value="overview">Route overview</option>
          </select>
        </section>
        <section className="panel-section">
          <div className="label-row"><label>Terrain</label><span>{terrainExaggeration.toFixed(1)}×</span></div>
          <input type="range" min="0" max="2" step="0.1" value={terrainExaggeration} onChange={(event) => setTerrainExaggeration(Number(event.target.value))} />
        </section>
        <section className="panel-section">
          <label>Format</label>
          <div className="segmented">
            {(["16:9", "9:16", "1:1"] as AspectRatio[]).map((ratio) => (
              <button key={ratio} onClick={() => setAspectRatio(ratio)} className={aspectRatio === ratio ? "selected" : ""}>{ratio}</button>
            ))}
          </div>
        </section>
        <section className="panel-section">
          <div className="label-row"><label>Duration</label><span>{durationSeconds}s</span></div>
          <input type="range" min="20" max="60" value={durationSeconds} onChange={(event) => setDurationSeconds(Number(event.target.value))} />
        </section>
        <section className="activity-summary">
          <div><MapPin size={15} /><span>{activity.points.length} processed points</span></div>
          <div><ShieldCheck size={15} /><span>Original activity unchanged</span></div>
          <div><CircleCheck size={15} /><span>Export readiness {!gpxSource ? "needs GPX" : !mapRegion ? "needs map data" : "complete"}</span></div>
        </section>
      </aside>

      <section className="timeline">
        <div className="timeline-head"><span>TIMELINE</span><span>{completion}% route complete</span></div>
        <div className="track"><span>ROUTE</span><div className="track-line route-line"><i style={{ width: `${completion}%` }} /><b style={{ left: `${completion}%` }} /></div></div>
        <div className="track"><span>CAMERA</span><div className="track-line camera-line"><i /><i /><i /><i /></div></div>
        <div className="track"><span>LABELS</span><div className="track-line labels-line"><em style={{ left: "4%" }}>Start</em><em style={{ left: "51%" }}>High point</em><em style={{ left: "90%" }}>Finish</em></div></div>
      </section>

      <footer className="statusbar"><span className={exportState === "error" ? "status-error" : ""}>{message}</span><span>Project revision 1 · Autosaved locally</span></footer>
    </main>
  );
}
