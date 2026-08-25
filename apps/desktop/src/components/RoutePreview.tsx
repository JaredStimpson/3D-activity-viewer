import { useEffect, useRef } from "react";
import type { Activity, CameraPreset, StylePreset } from "../types";
import { pointAtProgress } from "../lib/activity";

interface RoutePreviewProps {
  activity: Activity;
  progress: number;
  stylePreset: StylePreset;
  cameraPreset: CameraPreset;
  terrainExaggeration: number;
}

const palettes = {
  outdoor: { sky: "#b9d5c1", horizon: "#496958", low: "#172a22", high: "#34523f", contour: "#77907d", route: "#d5ff4f" },
  dark: { sky: "#121b21", horizon: "#1b2930", low: "#080d10", high: "#17232a", contour: "#34454d", route: "#88f7cf" },
  topographic: { sky: "#d7ccb1", horizon: "#8a8069", low: "#35372e", high: "#5b604b", contour: "#a49c7d", route: "#ff704d" },
};

function terrainHeight(x: number, y: number, seed: number) {
  return (
    Math.sin(x * 0.017 + seed) * 28 +
    Math.cos(y * 0.021 - seed * 0.7) * 21 +
    Math.sin((x + y) * 0.009) * 34
  );
}

export function RoutePreview({ activity, progress, stylePreset, cameraPreset, terrainExaggeration }: RoutePreviewProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const context = canvas.getContext("2d");
    if (!context) return;

    const ratio = window.devicePixelRatio || 1;
    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    canvas.width = Math.floor(width * ratio);
    canvas.height = Math.floor(height * ratio);
    context.scale(ratio, ratio);

    const palette = palettes[stylePreset];
    const gradient = context.createLinearGradient(0, 0, 0, height);
    gradient.addColorStop(0, palette.sky);
    gradient.addColorStop(0.45, palette.horizon);
    gradient.addColorStop(1, palette.low);
    context.fillStyle = gradient;
    context.fillRect(0, 0, width, height);

    const current = pointAtProgress(activity, progress);
    const bounds = activity.stats.bounds;
    const lonSpan = Math.max(bounds[2] - bounds[0], 0.0001);
    const latSpan = Math.max(bounds[3] - bounds[1], 0.0001);
    const focusX = (current.longitude - bounds[0]) / lonSpan;
    const focusY = (current.latitude - bounds[1]) / latSpan;
    const seed = Math.abs(bounds[0] * 0.17 + bounds[1] * 0.31);
    const zoom = cameraPreset === "overview" ? 0.82 : cameraPreset === "cinematic" ? 1.2 : 1.42;
    const pitch = cameraPreset === "overview" ? 0.45 : 0.68;

    context.globalAlpha = 0.72;
    for (let row = 0; row < 18; row += 1) {
      context.beginPath();
      for (let column = 0; column <= 50; column += 1) {
        const nx = column / 50;
        const ny = row / 17;
        const perspective = 0.34 + ny * 0.9;
        const terrain = terrainHeight(column * 29, row * 37, seed) * terrainExaggeration;
        const x = width / 2 + (nx - 0.5) * width * perspective * 1.25;
        const y = height * (0.37 + ny * 0.7 * pitch) - terrain * perspective;
        if (column === 0) context.moveTo(x, y);
        else context.lineTo(x, y);
      }
      context.strokeStyle = palette.contour;
      context.lineWidth = row % 3 === 0 ? 1.2 : 0.65;
      context.stroke();
    }
    context.globalAlpha = 1;

    const project = (latitude: number, longitude: number) => {
      const normalizedX = (longitude - bounds[0]) / lonSpan;
      const normalizedY = (latitude - bounds[1]) / latSpan;
      const centeredX = (normalizedX - focusX) * zoom;
      const centeredY = (normalizedY - focusY) * zoom;
      const depth = 0.82 + centeredY * 0.28;
      const terrain = terrainHeight(normalizedX * 820, normalizedY * 820, seed) * terrainExaggeration;
      return {
        x: width * 0.5 + centeredX * width * 0.66 * depth,
        y: height * 0.62 - centeredY * height * 0.64 * pitch - terrain * 0.22,
      };
    };

    const visibleDistance = activity.stats.distanceMeters * progress;
    const visiblePoints = activity.points.filter((point) => point.cumulativeDistanceMeters <= visibleDistance);
    visiblePoints.push(current);

    context.lineCap = "round";
    context.lineJoin = "round";
    context.beginPath();
    activity.points.forEach((point, index) => {
      const projected = project(point.latitude, point.longitude);
      if (index === 0) context.moveTo(projected.x, projected.y);
      else context.lineTo(projected.x, projected.y);
    });
    context.strokeStyle = "rgba(255,255,255,0.16)";
    context.lineWidth = 4;
    context.stroke();

    context.shadowColor = palette.route;
    context.shadowBlur = 14;
    context.beginPath();
    visiblePoints.forEach((point, index) => {
      const projected = project(point.latitude, point.longitude);
      if (index === 0) context.moveTo(projected.x, projected.y);
      else context.lineTo(projected.x, projected.y);
    });
    context.strokeStyle = palette.route;
    context.lineWidth = 4.5;
    context.stroke();
    context.shadowBlur = 0;

    const start = project(activity.points[0].latitude, activity.points[0].longitude);
    const finishPoint = activity.points.at(-1)!;
    const finish = project(finishPoint.latitude, finishPoint.longitude);
    const marker = project(current.latitude, current.longitude);
    for (const [position, label] of [[start, "S"], [finish, "F"]] as const) {
      context.fillStyle = "rgba(8, 14, 12, 0.9)";
      context.beginPath();
      context.arc(position.x, position.y, 10, 0, Math.PI * 2);
      context.fill();
      context.fillStyle = "#eef5ec";
      context.font = "600 9px system-ui";
      context.textAlign = "center";
      context.textBaseline = "middle";
      context.fillText(label, position.x, position.y + 0.5);
    }
    context.fillStyle = palette.route;
    context.shadowColor = palette.route;
    context.shadowBlur = 18;
    context.beginPath();
    context.arc(marker.x, marker.y, 7, 0, Math.PI * 2);
    context.fill();
    context.shadowBlur = 0;
  }, [activity, cameraPreset, progress, stylePreset, terrainExaggeration]);

  return <canvas ref={canvasRef} className="route-canvas" aria-label="Animated three-dimensional route preview" />;
}

