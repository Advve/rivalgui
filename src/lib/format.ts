// Builds values in rivalcfg syntax (the same strings the rivalcfg CLI accepts).
import type { Json, Stop } from "./api";

export function choiceValue(choice: string): Json {
  return /^\d+$/.test(choice) ? Number(choice) : choice;
}

export function dpiValue(values: number[]): string {
  return values.join(", ");
}

/** "800:800, 1600:900" (multidpi_range_choice_xy syntax) */
export function dpiXyValue(values: [number, number][]): string {
  return values.map(([x, y]) => `${x}:${y}`).join(", ");
}

export function gradientValue(isGradient: boolean, duration: number, stops: Stop[]): string {
  if (!isGradient || stops.length === 0) return stops[0]?.color ?? "#ff0000";
  const colors = [...stops]
    .sort((a, b) => a.pos - b.pos)
    .map((s) => `${Math.round(s.pos)}%: ${s.color}`)
    .join(", ");
  return `rgbgradient(duration=${Math.round(duration)}; colors=${colors})`;
}

export function buttonsValue(mapping: { button: string; value: string }[]): string {
  const parts = mapping.map((m) => `${m.button.toLowerCase()}=${m.value}`);
  return `buttons(${parts.join("; ")})`;
}

export function cssGradient(stops: Stop[]): string {
  if (stops.length === 0) return "transparent";
  if (stops.length === 1) return stops[0].color;
  const sorted = [...stops].sort((a, b) => a.pos - b.pos);
  // Close the loop like the device does (back to the first color at 100%)
  const closing = sorted[sorted.length - 1].pos < 100 ? `, ${sorted[0].color} 100%` : "";
  return `linear-gradient(90deg, ${sorted.map((s) => `${s.color} ${s.pos}%`).join(", ")}${closing})`;
}

export function shortName(name: string): string {
  return name.replace(/^SteelSeries\s+/, "");
}

/** Splits "Rival 650 Wireless (2.4 GHz wireless mode)" into name + mode. */
export function splitName(name: string): { title: string; mode: string | null } {
  const short = shortName(name);
  const m = short.match(/^(.*?)\s*\(([^)]*)\)\s*$/);
  return m ? { title: m[1], mode: m[2] } : { title: short, mode: null };
}

export function prettyKey(value: string): string {
  return value.charAt(0).toUpperCase() + value.slice(1);
}
