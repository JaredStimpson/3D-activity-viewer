export type Bounds = [number, number, number, number];

export function parseBoundsText(value: string): Bounds | null {
  const values = value.split(",").map((part) => Number(part.trim()));
  return values.length === 4 && values.every(Number.isFinite) ? values as Bounds : null;
}

export function replaceCoordinate(bounds: Bounds, index: number, value: string): Bounds {
  const next = [...bounds] as Bounds;
  next[index] = Number(value);
  return next;
}
