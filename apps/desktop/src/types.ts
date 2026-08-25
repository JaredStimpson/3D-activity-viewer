export interface ActivityPoint {
  latitude: number;
  longitude: number;
  elevation?: number;
  timestamp?: string;
  cumulativeDistanceMeters: number;
}

export interface ActivityStats {
  distanceMeters: number;
  elevationGainMeters: number;
  elevationLossMeters: number;
  durationSeconds?: number;
  minElevationMeters?: number;
  maxElevationMeters?: number;
  bounds: [number, number, number, number];
}

export interface Activity {
  name: string;
  points: ActivityPoint[];
  stats: ActivityStats;
}

export type StylePreset = "outdoor" | "dark" | "topographic";
export type CameraPreset = "follow" | "cinematic" | "overview";
export type AspectRatio = "16:9" | "9:16" | "1:1";

export interface ExportOptions {
  width: number;
  height: number;
  fps: number;
  durationSeconds: number;
}

