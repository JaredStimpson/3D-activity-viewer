import { useEffect, useMemo, useState } from "react";
import { Channel, invoke } from "@tauri-apps/api/core";
import { CheckCircle2, Download, FolderOpen, HardDrive, MapPinned, Square, XCircle } from "lucide-react";
import { parseBoundsText, replaceCoordinate, type Bounds } from "./bounds";

interface RegionAsset {
  file: string;
  maxZoom: number;
  sizeBytes: number;
}

interface RegionManifest {
  id: string;
  name: string;
  bounds: Bounds;
  basemap: RegionAsset;
  terrain: RegionAsset;
  verifiedAt: string;
}

interface Estimate {
  basemapTiles: number;
  terrainTiles: number;
  basemapBytes: number;
  terrainBytes: number;
  totalBytes: number;
  availableBytes: number;
}

type DownloadEvent =
  | { event: "resolvingSources"; data: { message: string } }
  | { event: "layerStarted"; data: { layer: string; totalTiles: number } }
  | { event: "progress"; data: { layer: string; completedTiles: number; totalTiles: number; downloadedBytes: number } }
  | { event: "verifying"; data: { layer: string } }
  | { event: "complete"; data: { regionPath: string } };

const initialBounds: Bounds = [-121.95, 35.95, -121.55, 36.35];

function formatBytes(bytes: number) {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  return `${(bytes / 1024 ** index).toFixed(index > 1 ? 1 : 0)} ${units[index]}`;
}

function percent(completed: number, total: number) {
  return total > 0 ? Math.min(100, Math.round((completed / total) * 100)) : 0;
}

export default function App() {
  const [name, setName] = useState("Big Sur");
  const [bounds, setBounds] = useState<Bounds>(initialBounds);
  const [boundsText, setBoundsText] = useState(initialBounds.join(","));
  const [estimate, setEstimate] = useState<Estimate | null>(null);
  const [regions, setRegions] = useState<RegionManifest[]>([]);
  const [status, setStatus] = useState("Enter an area to estimate its download.");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [layerProgress, setLayerProgress] = useState({
    basemap: { completed: 0, total: 0, bytes: 0 },
    terrain: { completed: 0, total: 0, bytes: 0 },
  });

  const request = useMemo(() => ({ name, bounds }), [name, bounds]);

  async function refreshRegions() {
    try {
      setRegions(await invoke<RegionManifest[]>("list_local_regions"));
    } catch (reason) {
      setError(String(reason));
    }
  }

  useEffect(() => {
    void refreshRegions();
  }, []);

  async function estimateArea() {
    setError(null);
    try {
      const validated = await invoke<Bounds>("validate_bounds", { bounds });
      setBounds(validated);
      setBoundsText(validated.join(","));
      const next = await invoke<Estimate>("estimate_download", { request: { name, bounds: validated } });
      setEstimate(next);
      setStatus("Estimate ready. Actual compressed size varies by location.");
    } catch (reason) {
      setEstimate(null);
      setError(String(reason));
    }
  }

  function setCoordinate(index: number, value: string) {
    const next = replaceCoordinate(bounds, index, value);
    setBounds(next);
    setBoundsText(next.join(","));
    setEstimate(null);
  }

  function applyBoundsText(value: string) {
    setBoundsText(value);
    const values = parseBoundsText(value);
    if (values) {
      setBounds(values);
      setEstimate(null);
    }
  }

  async function startDownload() {
    setBusy(true);
    setError(null);
    setLayerProgress({
      basemap: { completed: 0, total: 0, bytes: 0 },
      terrain: { completed: 0, total: 0, bytes: 0 },
    });
    const events = new Channel<DownloadEvent>();
    events.onmessage = (message) => {
      if (message.event === "resolvingSources") setStatus(message.data.message);
      if (message.event === "layerStarted") {
        setStatus(`Downloading ${message.data.layer}…`);
        setLayerProgress((current) => ({
          ...current,
          [message.data.layer]: { completed: 0, total: message.data.totalTiles, bytes: 0 },
        }));
      }
      if (message.event === "progress") {
        setLayerProgress((current) => ({
          ...current,
          [message.data.layer]: {
            completed: message.data.completedTiles,
            total: message.data.totalTiles,
            bytes: message.data.downloadedBytes,
          },
        }));
      }
      if (message.event === "verifying") setStatus(`Verifying ${message.data.layer}…`);
      if (message.event === "complete") setStatus(`Installed ${message.data.regionPath}`);
    };
    try {
      const manifest = await invoke<RegionManifest>("start_download", { request, events });
      setStatus(`Map data ready: maps/regions/${manifest.id}`);
      await refreshRegions();
    } catch (reason) {
      setError(String(reason));
      setStatus("Download did not complete.");
    } finally {
      setBusy(false);
    }
  }

  async function cancelDownload() {
    await invoke("cancel_download");
    setStatus("Cancelling download…");
  }

  return (
    <main className="shell">
      <header>
        <div className="brand-icon"><MapPinned size={24} /></div>
        <div>
          <span className="eyebrow">WAYPOINT TOOL</span>
          <h1>Map Downloader</h1>
          <p>Download one offline basemap and terrain package for a rectangular area.</p>
        </div>
        <button className="quiet-button" onClick={() => void invoke("open_maps_folder")}><FolderOpen size={16} /> Maps folder</button>
      </header>

      <section className="card form-card">
        <div className="section-title"><span>1</span><div><h2>Choose an area</h2><p>WGS84 coordinates in west, south, east, north order.</p></div></div>
        <label>Area name<input value={name} maxLength={80} onChange={(event) => { setName(event.target.value); setEstimate(null); }} disabled={busy} /></label>
        <label>Paste bounds<input value={boundsText} onChange={(event) => applyBoundsText(event.target.value)} disabled={busy} /></label>
        <div className="coordinate-grid">
          {["West", "South", "East", "North"].map((label, index) => (
            <label key={label}>{label}<input type="number" step="0.000001" value={bounds[index]} onChange={(event) => setCoordinate(index, event.target.value)} disabled={busy} /></label>
          ))}
        </div>
        <div className="hint">Example: <code>-121.95,35.95,-121.55,36.35</code></div>
        <button className="secondary-button" onClick={() => void estimateArea()} disabled={busy || !name.trim()}>Estimate download</button>
      </section>

      <section className="card download-card">
        <div className="section-title"><span>2</span><div><h2>Download map data</h2><p>Standard quality is fixed for consistent Waypoint renders.</p></div></div>
        <div className="quality-row">
          <div><strong>Vector basemap</strong><span>Zoom 0–15</span></div>
          <div><strong>3D terrain</strong><span>Terrarium · up to zoom 14</span></div>
          <div><strong>Storage</strong><span>Repo-local maps/regions</span></div>
        </div>
        {estimate && (
          <div className="estimate-grid">
            <div><span>Basemap</span><strong>{formatBytes(estimate.basemapBytes)}</strong><small>{estimate.basemapTiles.toLocaleString()} possible tiles</small></div>
            <div><span>Terrain</span><strong>{formatBytes(estimate.terrainBytes)}</strong><small>{estimate.terrainTiles.toLocaleString()} possible tiles</small></div>
            <div><span>Estimated total</span><strong>{formatBytes(estimate.totalBytes)}</strong><small>{formatBytes(estimate.availableBytes)} available</small></div>
          </div>
        )}
        {(["basemap", "terrain"] as const).map((layer) => {
          const progress = layerProgress[layer];
          if (!busy && progress.total === 0) return null;
          return <div className="progress-block" key={layer}>
            <div><strong>{layer === "basemap" ? "Basemap" : "Terrain"}</strong><span>{percent(progress.completed, progress.total)}% · {formatBytes(progress.bytes)}</span></div>
            <progress max={Math.max(progress.total, 1)} value={progress.completed} />
          </div>;
        })}
        <div className="actions">
          <button className="primary-button" onClick={() => void startDownload()} disabled={busy || !estimate}><Download size={17} /> Download Map Data</button>
          {busy && <button className="danger-button" onClick={() => void cancelDownload()}><Square size={14} /> Cancel</button>}
        </div>
        <div className={`status ${error ? "error" : ""}`}>
          {error ? <XCircle size={17} /> : <CheckCircle2 size={17} />}
          <span>{error ?? status}</span>
        </div>
      </section>

      <section className="card regions-card">
        <div className="section-title"><HardDrive size={20} /><div><h2>Downloaded regions</h2><p>Waypoint discovers these folders automatically.</p></div></div>
        {regions.length === 0 ? <div className="empty">No complete regions downloaded yet.</div> : regions.map((region) => (
          <article className="region" key={region.id}>
            <div><strong>{region.name}</strong><code>{region.bounds.join(", ")}</code></div>
            <span>{formatBytes(region.basemap.sizeBytes + region.terrain.sizeBytes)}</span>
          </article>
        ))}
      </section>
    </main>
  );
}
