import { describe, expect, it } from "vitest";
import { evaluateScene, timelineProgress } from "./scene";
import type { Activity } from "../types";

const activity: Activity = {
  name: "Fixture",
  points: [
    { latitude: 36, longitude: -122, cumulativeDistanceMeters: 0 },
    { latitude: 36.1, longitude: -121.9, cumulativeDistanceMeters: 1000 },
  ],
  stats: {
    distanceMeters: 1000,
    elevationGainMeters: 0,
    elevationLossMeters: 0,
    bounds: [-122, 36, -121.9, 36.1],
  },
};

describe("deterministic scene evaluation", () => {
  it("evaluates export progress strictly from frame index", () => {
    expect(timelineProgress(0, 30, 20)).toBe(0);
    expect(timelineProgress(599, 30, 20)).toBe(1);
    expect(timelineProgress(300, 30, 20)).toBe(timelineProgress(300, 30, 20));
  });

  it("uses the same route evaluator for progressive geometry", () => {
    const scene = evaluateScene(activity, 0.5, "follow");
    expect(scene.current[0]).toBeCloseTo(-121.95);
    expect(scene.completedRoute.geometry.coordinates.at(-1)).toEqual(scene.current);
    expect(scene.endpoints.features).toHaveLength(2);
  });
});
