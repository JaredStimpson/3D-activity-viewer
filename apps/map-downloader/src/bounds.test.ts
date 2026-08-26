import { describe, expect, it } from "vitest";
import { parseBoundsText, replaceCoordinate } from "./bounds";

describe("bounding-box fields", () => {
  it("parses standard west,south,east,north text", () => {
    expect(parseBoundsText("-121.95, 35.95, -121.55, 36.35")).toEqual([
      -121.95, 35.95, -121.55, 36.35,
    ]);
    expect(parseBoundsText("-121,35,broken,36")).toBeNull();
  });

  it("keeps numeric fields synchronized", () => {
    expect(replaceCoordinate([-121, 35, -120, 36], 2, "-119.5")).toEqual([
      -121, 35, -119.5, 36,
    ]);
  });
});
