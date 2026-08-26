import { forwardRef, useEffect, useImperativeHandle, useRef } from "react";
import * as maplibregl from "maplibre-gl";
import type { Map as MapLibreMap, StyleSpecification } from "maplibre-gl";
import { DARK, LIGHT, layers as protomapsLayers } from "@protomaps/basemaps";
import { formatDistance, formatDuration } from "../lib/activity";
import { registerMapRegion } from "../lib/mapAssets";
import { evaluateScene } from "../lib/scene";
import type { Activity, CameraPreset, MapRegion, StylePreset } from "../types";

export interface RoutePreviewHandle {
  captureFrame(progress: number, width: number, height: number): Promise<Uint8Array>;
  disposeExport(): void;
}

interface RoutePreviewProps {
  activity: Activity;
  region: MapRegion | null;
  progress: number;
  stylePreset: StylePreset;
  cameraPreset: CameraPreset;
  terrainExaggeration: number;
}

const SOURCE = "waypoint-basemap";
const TERRAIN = "waypoint-terrain";
const ROUTE = "waypoint-route";
const COMPLETED = "waypoint-route-completed";
const CURRENT = "waypoint-current";
const ENDPOINTS = "waypoint-endpoints";

function sanitizeLayer(layer: unknown) {
  const clean = structuredClone(layer) as Record<string, unknown>;
  const layout = clean.layout as Record<string, unknown> | undefined;
  const paint = clean.paint as Record<string, unknown> | undefined;
  if (layout) {
    delete layout["icon-image"];
    if (layout["text-field"]) layout["text-font"] = ["Segoe UI"];
  }
  if (paint) {
    delete paint["fill-pattern"];
    delete paint["line-pattern"];
  }
  return clean;
}

function mapStyle(region: MapRegion, stylePreset: StylePreset): StyleSpecification {
  const urls = registerMapRegion(maplibregl, region.id);
  const base = protomapsLayers(SOURCE, stylePreset === "dark" ? DARK : LIGHT, { lang: "en" })
    .map(sanitizeLayer);
  const routeColor = stylePreset === "topographic" ? "#ff704d" : stylePreset === "dark" ? "#88f7cf" : "#d5ff4f";
  return {
    version: 8,
    sources: {
      [SOURCE]: { type: "vector", url: urls.basemapUrl, attribution: region.basemap.attribution },
      [TERRAIN]: {
        type: "raster-dem",
        url: urls.terrainUrl,
        tileSize: 512,
        encoding: "terrarium",
        attribution: region.terrain.attribution,
      },
      [ROUTE]: { type: "geojson", data: { type: "FeatureCollection", features: [] } },
      [COMPLETED]: { type: "geojson", data: { type: "FeatureCollection", features: [] } },
      [CURRENT]: { type: "geojson", data: { type: "FeatureCollection", features: [] } },
      [ENDPOINTS]: { type: "geojson", data: { type: "FeatureCollection", features: [] } },
    },
    layers: [
      ...base,
      {
        id: "waypoint-hillshade", type: "hillshade", source: TERRAIN,
        paint: { "hillshade-exaggeration": 0.45, "hillshade-shadow-color": "#16201a" },
      },
      {
        id: "waypoint-buildings", type: "fill-extrusion", source: SOURCE, "source-layer": "buildings",
        minzoom: 14,
        paint: {
          "fill-extrusion-color": stylePreset === "dark" ? "#53605c" : "#c4c0b4",
          "fill-extrusion-height": ["coalesce", ["get", "height"], 4],
          "fill-extrusion-base": ["coalesce", ["get", "min_height"], 0],
          "fill-extrusion-opacity": 0.75,
        },
      },
      { id: "waypoint-route-shadow", type: "line", source: ROUTE, paint: { "line-color": "#07100c", "line-width": 8, "line-opacity": 0.55 }, layout: { "line-cap": "round", "line-join": "round" } },
      { id: "waypoint-route-base", type: "line", source: ROUTE, paint: { "line-color": "rgba(255,255,255,.3)", "line-width": 4 }, layout: { "line-cap": "round", "line-join": "round" } },
      { id: "waypoint-route-progress", type: "line", source: COMPLETED, paint: { "line-color": routeColor, "line-width": 5 }, layout: { "line-cap": "round", "line-join": "round" } },
      { id: "waypoint-endpoints", type: "circle", source: ENDPOINTS, paint: { "circle-radius": 9, "circle-color": "#101713", "circle-stroke-width": 2, "circle-stroke-color": "#f4f7f4" } },
      { id: "waypoint-endpoint-labels", type: "symbol", source: ENDPOINTS, layout: { "text-field": ["get", "label"], "text-font": ["Segoe UI"], "text-size": 10 }, paint: { "text-color": "#ffffff" } },
      { id: "waypoint-current", type: "circle", source: CURRENT, paint: { "circle-radius": 8, "circle-color": routeColor, "circle-stroke-width": 3, "circle-stroke-color": "#ffffff" } },
    ] as StyleSpecification["layers"],
    terrain: { source: TERRAIN, exaggeration: 1 },
  };
}

function setGeoJson(map: MapLibreMap, source: string, data: object) {
  (map.getSource(source) as maplibregl.GeoJSONSource | undefined)?.setData(data as never);
}

function applyScene(map: MapLibreMap, activity: Activity, progress: number, camera: CameraPreset, terrain: number) {
  const scene = evaluateScene(activity, progress, camera);
  setGeoJson(map, ROUTE, scene.route);
  setGeoJson(map, COMPLETED, scene.completedRoute);
  setGeoJson(map, ENDPOINTS, scene.endpoints);
  setGeoJson(map, CURRENT, { type: "Feature", properties: {}, geometry: { type: "Point", coordinates: scene.current } });
  map.setTerrain({ source: TERRAIN, exaggeration: terrain });
  if (camera === "overview") {
    map.fitBounds(
      [[activity.stats.bounds[0], activity.stats.bounds[1]], [activity.stats.bounds[2], activity.stats.bounds[3]]],
      { padding: 70, pitch: scene.pitch, bearing: scene.bearing, duration: 0 },
    );
  } else {
    map.jumpTo({ center: scene.center, zoom: camera === "cinematic" ? 14.2 : 14.8, pitch: scene.pitch, bearing: scene.bearing });
  }
  map.triggerRepaint();
}

function waitForLoad(map: MapLibreMap) {
  if (map.loaded()) return Promise.resolve();
  return new Promise<void>((resolve) => map.once("load", () => resolve()));
}

async function waitForIdle(map: MapLibreMap) {
  const started = performance.now();
  while (!map.areTilesLoaded() && performance.now() - started < 20_000) {
    await new Promise((resolve) => setTimeout(resolve, 25));
  }
  if (!map.areTilesLoaded()) {
    throw new Error("Local map tiles did not finish loading within 20 seconds.");
  }
  await new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
}

export const RoutePreview = forwardRef<RoutePreviewHandle, RoutePreviewProps>(function RoutePreview(
  { activity, region, progress, stylePreset, cameraPreset, terrainExaggeration }, ref,
) {
  const containerRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<MapLibreMap | null>(null);
  const exportMapRef = useRef<{ map: MapLibreMap; container: HTMLDivElement; width: number; height: number } | null>(null);

  useEffect(() => {
    const container = containerRef.current;
    if (!container || !region) return;
    const map = new maplibregl.Map({
      container,
      style: mapStyle(region, stylePreset),
      center: [(activity.stats.bounds[0] + activity.stats.bounds[2]) / 2, (activity.stats.bounds[1] + activity.stats.bounds[3]) / 2],
      zoom: 12,
      pitch: 55,
      attributionControl: false,
      interactive: true,
      canvasContextAttributes: { preserveDrawingBuffer: true },
    });
    mapRef.current = map;
    map.once("load", () => applyScene(map, activity, progress, cameraPreset, terrainExaggeration));
    return () => {
      mapRef.current = null;
      map.remove();
    };
  }, [activity, region, stylePreset]);

  useEffect(() => {
    const map = mapRef.current;
    if (map?.loaded()) applyScene(map, activity, progress, cameraPreset, terrainExaggeration);
  }, [activity, cameraPreset, progress, terrainExaggeration]);

  function disposeExport() {
    const exportMap = exportMapRef.current;
    if (!exportMap) return;
    exportMap.map.remove();
    exportMap.container.remove();
    exportMapRef.current = null;
  }

  useImperativeHandle(ref, () => ({
    async captureFrame(frameProgress, width, height) {
      if (!region) throw new Error("No verified local map region covers this activity.");
      let current = exportMapRef.current;
      if (!current || current.width !== width || current.height !== height) {
        disposeExport();
        const container = document.createElement("div");
        container.className = "map-export-canvas";
        Object.assign(container.style, { position: "fixed", left: "-20000px", top: "0", width: `${width}px`, height: `${height}px` });
        document.body.appendChild(container);
        const map = new maplibregl.Map({
          container,
          style: mapStyle(region, stylePreset),
          center: [0, 0],
          zoom: 1,
          attributionControl: false,
          interactive: false,
          fadeDuration: 0,
          canvasContextAttributes: { preserveDrawingBuffer: true },
        });
        current = { map, container, width, height };
        exportMapRef.current = current;
        await waitForLoad(map);
      }
      applyScene(current.map, activity, frameProgress, cameraPreset, terrainExaggeration);
      await waitForIdle(current.map);

      const capture = document.createElement("canvas");
      capture.width = width;
      capture.height = height;
      const context = capture.getContext("2d", { willReadFrequently: true });
      if (!context) throw new Error("Could not create the export capture canvas.");
      context.drawImage(current.map.getCanvas(), 0, 0, width, height);
      const shade = context.createLinearGradient(0, 0, 0, height);
      shade.addColorStop(0, "rgba(4,8,6,.18)");
      shade.addColorStop(0.65, "rgba(4,8,6,0)");
      shade.addColorStop(1, "rgba(4,8,6,.72)");
      context.fillStyle = shade;
      context.fillRect(0, 0, width, height);
      context.fillStyle = "#ffffff";
      context.font = `700 ${Math.max(34, Math.round(width * 0.031))}px Segoe UI`;
      context.fillText(activity.name, width * 0.045, height * 0.1);
      context.font = `600 ${Math.max(18, Math.round(width * 0.014))}px Segoe UI`;
      context.fillStyle = "rgba(255,255,255,.88)";
      context.fillText(`${formatDistance(activity.stats.distanceMeters)}   ·   ${Math.round(activity.stats.elevationGainMeters)} m gain   ·   ${formatDuration(activity.stats.durationSeconds)}`, width * 0.045, height * 0.94);
      return new Uint8Array(context.getImageData(0, 0, width, height).data.buffer);
    },
    disposeExport,
  }), [activity, cameraPreset, region, stylePreset, terrainExaggeration]);

  useEffect(() => disposeExport, []);

  return (
    <div className="map-preview" aria-label="Offline three-dimensional route preview">
      <div ref={containerRef} className="maplibre-container" />
      {!region && (
        <div className="map-required">
          <strong>Local map data needed</strong>
          <span>Download a region covering {activity.stats.bounds.map((value) => value.toFixed(5)).join(", ")} with Waypoint Map Downloader, then refresh Map data.</span>
        </div>
      )}
    </div>
  );
});
