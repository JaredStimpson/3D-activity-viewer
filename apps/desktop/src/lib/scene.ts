import { pointAtProgress } from "./activity";
import type { Activity, CameraPreset } from "../types";

type LineFeature = { type: "Feature"; properties: Record<string, never>; geometry: { type: "LineString"; coordinates: [number, number][] } };
type PointCollection = { type: "FeatureCollection"; features: Array<{ type: "Feature"; properties: { label: string }; geometry: { type: "Point"; coordinates: [number, number] } }> };

export interface EvaluatedScene {
  current: [number, number];
  route: LineFeature;
  completedRoute: LineFeature;
  endpoints: PointCollection;
  center: [number, number];
  bearing: number;
  pitch: number;
}

export function timelineProgress(frameIndex: number, fps: number, durationSeconds: number) {
  const normalized = Math.min(1, Math.max(0, frameIndex / Math.max(1, fps * durationSeconds - 1)));
  const routeWindow = Math.min(1, Math.max(0, (normalized - 0.07) / 0.83));
  return routeWindow * routeWindow * (3 - 2 * routeWindow);
}

export function evaluateScene(activity: Activity, progress: number, camera: CameraPreset): EvaluatedScene {
  const current = pointAtProgress(activity, progress);
  const completed = activity.points.filter(
    (point) => point.cumulativeDistanceMeters <= activity.stats.distanceMeters * progress,
  );
  const currentCoordinate: [number, number] = [current.longitude, current.latitude];
  const completedCoordinates = completed.map((point) => [point.longitude, point.latitude] as [number, number]);
  if (completedCoordinates.length === 0) completedCoordinates.push(currentCoordinate);
  if (completedCoordinates.at(-1)?.[0] !== currentCoordinate[0] || completedCoordinates.at(-1)?.[1] !== currentCoordinate[1]) {
    completedCoordinates.push(currentCoordinate);
  }
  if (completedCoordinates.length === 1) completedCoordinates.push(currentCoordinate);

  const next = pointAtProgress(activity, Math.min(1, progress + 0.003));
  const bearing = Math.atan2(next.longitude - current.longitude, next.latitude - current.latitude) * 180 / Math.PI;
  const first = activity.points[0];
  const last = activity.points.at(-1)!;

  return {
    current: currentCoordinate,
    route: {
      type: "Feature",
      properties: {},
      geometry: { type: "LineString", coordinates: activity.points.map((point) => [point.longitude, point.latitude]) },
    },
    completedRoute: {
      type: "Feature",
      properties: {},
      geometry: { type: "LineString", coordinates: completedCoordinates },
    },
    endpoints: {
      type: "FeatureCollection",
      features: [
        { type: "Feature", properties: { label: "S" }, geometry: { type: "Point", coordinates: [first.longitude, first.latitude] } },
        { type: "Feature", properties: { label: "F" }, geometry: { type: "Point", coordinates: [last.longitude, last.latitude] } },
      ],
    },
    center: camera === "overview"
      ? [(activity.stats.bounds[0] + activity.stats.bounds[2]) / 2, (activity.stats.bounds[1] + activity.stats.bounds[3]) / 2]
      : currentCoordinate,
    bearing: camera === "follow" ? bearing : camera === "cinematic" ? bearing - 18 : 0,
    pitch: camera === "overview" ? 38 : camera === "cinematic" ? 68 : 58,
  };
}
