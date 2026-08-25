import { describe, expect, it } from "vitest";
import { distanceMeters } from "./activity";

describe("distanceMeters", () => {
  it("calculates a stable great-circle distance", () => {
    const base = { elevation: 0, cumulativeDistanceMeters: 0 };
    const distance = distanceMeters(
      { ...base, latitude: 36, longitude: -121 },
      { ...base, latitude: 36.01, longitude: -121 },
    );
    expect(distance).toBeGreaterThan(1110);
    expect(distance).toBeLessThan(1113);
  });
});

