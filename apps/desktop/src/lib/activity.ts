import type { Activity, ActivityPoint } from "../types";

const EARTH_RADIUS_METERS = 6_371_000;

function radians(value: number) {
  return (value * Math.PI) / 180;
}

export function distanceMeters(a: ActivityPoint, b: ActivityPoint) {
  const dLat = radians(b.latitude - a.latitude);
  const dLon = radians(b.longitude - a.longitude);
  const lat1 = radians(a.latitude);
  const lat2 = radians(b.latitude);
  const h =
    Math.sin(dLat / 2) ** 2 +
    Math.cos(lat1) * Math.cos(lat2) * Math.sin(dLon / 2) ** 2;
  return 2 * EARTH_RADIUS_METERS * Math.asin(Math.sqrt(h));
}

export function parseGpxLocally(source: string): Activity {
  const document = new DOMParser().parseFromString(source, "application/xml");
  if (document.querySelector("parsererror")) throw new Error("The GPX file is not valid XML.");

  const name = document.querySelector("trk > name")?.textContent?.trim() || "Imported activity";
  const points: ActivityPoint[] = Array.from(document.querySelectorAll("trkpt")).map((node) => ({
    latitude: Number(node.getAttribute("lat")),
    longitude: Number(node.getAttribute("lon")),
    elevation: node.querySelector("ele") ? Number(node.querySelector("ele")?.textContent) : undefined,
    timestamp: node.querySelector("time")?.textContent?.trim(),
    cumulativeDistanceMeters: 0,
  }));

  if (points.length < 2 || points.some((point) => !Number.isFinite(point.latitude) || !Number.isFinite(point.longitude))) {
    throw new Error("The GPX file needs at least two valid track points.");
  }

  let distance = 0;
  let gain = 0;
  let loss = 0;
  for (let index = 1; index < points.length; index += 1) {
    distance += distanceMeters(points[index - 1], points[index]);
    points[index].cumulativeDistanceMeters = distance;
    const delta = (points[index].elevation ?? 0) - (points[index - 1].elevation ?? 0);
    if (delta > 0) gain += delta;
    if (delta < 0) loss -= delta;
  }

  const elevations = points.map((point) => point.elevation).filter((value): value is number => value !== undefined);
  const times = points.map((point) => point.timestamp).filter((value): value is string => Boolean(value));

  return {
    name,
    points,
    stats: {
      distanceMeters: distance,
      elevationGainMeters: gain,
      elevationLossMeters: loss,
      durationSeconds:
        times.length > 1 ? Math.max(0, (Date.parse(times.at(-1)!) - Date.parse(times[0])) / 1000) : undefined,
      minElevationMeters: elevations.length ? Math.min(...elevations) : undefined,
      maxElevationMeters: elevations.length ? Math.max(...elevations) : undefined,
      bounds: [
        Math.min(...points.map((point) => point.longitude)),
        Math.min(...points.map((point) => point.latitude)),
        Math.max(...points.map((point) => point.longitude)),
        Math.max(...points.map((point) => point.latitude)),
      ],
    },
  };
}

export function pointAtProgress(activity: Activity, progress: number) {
  const target = activity.stats.distanceMeters * Math.max(0, Math.min(1, progress));
  const index = activity.points.findIndex((point) => point.cumulativeDistanceMeters >= target);
  if (index <= 0) return activity.points[0];
  const after = activity.points[index];
  const before = activity.points[index - 1];
  const span = after.cumulativeDistanceMeters - before.cumulativeDistanceMeters || 1;
  const t = (target - before.cumulativeDistanceMeters) / span;
  return {
    ...after,
    latitude: before.latitude + (after.latitude - before.latitude) * t,
    longitude: before.longitude + (after.longitude - before.longitude) * t,
    elevation: (before.elevation ?? 0) + ((after.elevation ?? 0) - (before.elevation ?? 0)) * t,
    cumulativeDistanceMeters: target,
  };
}

export function formatDistance(meters: number) {
  return `${(meters / 1609.344).toFixed(1)} mi`;
}

export function formatDuration(seconds?: number) {
  if (seconds === undefined || !Number.isFinite(seconds)) return "—";
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  return hours ? `${hours}h ${minutes}m` : `${minutes}m`;
}

