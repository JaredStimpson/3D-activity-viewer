import { invoke } from "@tauri-apps/api/core";
import { PMTiles, Protocol, type Source } from "pmtiles";

class TauriPmtilesSource implements Source {
  constructor(
    private readonly regionId: string,
    private readonly kind: "basemap" | "terrain",
  ) {}

  getKey() {
    return `waypoint-${this.kind}-${this.regionId}`;
  }

  async getBytes(offset: number, length: number): Promise<{ data: ArrayBuffer }> {
    const response = await invoke<ArrayBuffer | Uint8Array>("read_map_range", {
      regionId: this.regionId,
      kind: this.kind,
      offset,
      length,
    });
    if (response instanceof ArrayBuffer) return { data: response };
    return { data: response.buffer.slice(response.byteOffset, response.byteOffset + response.byteLength) as ArrayBuffer };
  }
}

const protocol = new Protocol({ metadata: true });
let registered = false;

export function registerMapRegion(maplibregl: typeof import("maplibre-gl"), regionId: string) {
  if (!registered) {
    maplibregl.addProtocol("pmtiles", protocol.tile);
    registered = true;
  }

  const basemap = new PMTiles(new TauriPmtilesSource(regionId, "basemap"));
  const terrain = new PMTiles(new TauriPmtilesSource(regionId, "terrain"));
  protocol.add(basemap);
  protocol.add(terrain);

  return {
    basemapUrl: `pmtiles://${basemap.source.getKey()}`,
    terrainUrl: `pmtiles://${terrain.source.getKey()}`,
  };
}
