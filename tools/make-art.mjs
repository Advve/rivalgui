// Draws the project artwork from the app icon (app-icon.svg):
// - the NSIS installer images (sidebar + header BMPs) in src-tauri/windows/
// - the README banner (docs/banner.png)
//
// Usage: npm run art

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { Resvg } from "@resvg/resvg-js";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const OUT = path.join(ROOT, "src-tauri", "windows");
const DOCS = path.join(ROOT, "docs");
const ICON_SVG = fs.readFileSync(path.join(ROOT, "app-icon.svg"), "utf8");

/** The app icon as a nested <svg> placed at (x, y) with the given size. */
function icon(x, y, size) {
  return ICON_SVG.replace("<svg ", `<svg x="${x}" y="${y}" width="${size}" height="${size}" `);
}

const FONT = "Segoe UI, Arial, sans-serif";

/** Welcome / finish page image (164x314). */
const sidebar = `<svg xmlns="http://www.w3.org/2000/svg" width="164" height="314">
  <defs>
    <linearGradient id="side" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#1d2230"/>
      <stop offset="1" stop-color="#0d1017"/>
    </linearGradient>
  </defs>
  <rect width="164" height="314" fill="url(#side)"/>
  ${icon(40, 70, 84)}
  <text x="82" y="194" text-anchor="middle" font-family="${FONT}" font-weight="700" font-size="22" fill="#e8ecf3">RivalGUI</text>
  <text x="82" y="218" text-anchor="middle" font-family="${FONT}" font-size="11" fill="#9aa5b8">SteelSeries mouse</text>
  <text x="82" y="234" text-anchor="middle" font-family="${FONT}" font-size="11" fill="#9aa5b8">configurator</text>
  <rect x="62" y="250" width="40" height="3" fill="#ff6a3d"/>
</svg>`;

/** Header image of the inner pages (150x57, shown on a white bar). */
const header = `<svg xmlns="http://www.w3.org/2000/svg" width="150" height="57">
  <rect width="150" height="57" fill="#ffffff"/>
  ${icon(100, 8, 40)}
</svg>`;

/** README banner (1280x400, shown at half size for sharp text on high-DPI screens). */
const banner = `<svg xmlns="http://www.w3.org/2000/svg" width="1280" height="400">
  <defs>
    <linearGradient id="bannerBg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#1d2230"/>
      <stop offset="1" stop-color="#0d1017"/>
    </linearGradient>
    <radialGradient id="bannerGlow" cx="0.27" cy="0.5" r="0.45">
      <stop offset="0" stop-color="#ff6a3d" stop-opacity="0.22"/>
      <stop offset="1" stop-color="#ff6a3d" stop-opacity="0"/>
    </radialGradient>
    <linearGradient id="bannerAccent" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0" stop-color="#ff7a3d"/>
      <stop offset="1" stop-color="#ff3d6e"/>
    </linearGradient>
  </defs>
  <rect width="1280" height="400" rx="36" fill="url(#bannerBg)"/>
  <rect width="1280" height="400" rx="36" fill="url(#bannerGlow)"/>
  ${icon(150, 80, 240)}
  <text x="460" y="205" font-family="${FONT}" font-weight="700" font-size="104" fill="#e8ecf3">RivalGUI</text>
  <text x="464" y="262" font-family="${FONT}" font-size="36" fill="#9aa5b8">Lightweight SteelSeries mouse configurator</text>
  <rect x="466" y="298" width="96" height="6" rx="3" fill="url(#bannerAccent)"/>
  <text x="586" y="307" font-family="${FONT}" font-size="24" fill="#6b768a">Windows · Linux · no SteelSeries GG</text>
</svg>`;

/** Renders an SVG and encodes it as a 24-bit bottom-up BMP (what NSIS expects). */
function toBmp(svg) {
  const image = render(svg);
  const { width, height, pixels } = image;
  const rowSize = Math.ceil((width * 3) / 4) * 4;
  const bmp = Buffer.alloc(54 + rowSize * height);
  bmp.write("BM", 0, "ascii");
  bmp.writeUInt32LE(bmp.length, 2);
  bmp.writeUInt32LE(54, 10); // pixel data offset
  bmp.writeUInt32LE(40, 14); // BITMAPINFOHEADER size
  bmp.writeInt32LE(width, 18);
  bmp.writeInt32LE(height, 22); // positive: bottom-up rows
  bmp.writeUInt16LE(1, 26); // planes
  bmp.writeUInt16LE(24, 28); // bits per pixel
  bmp.writeUInt32LE(rowSize * height, 34);
  bmp.writeInt32LE(2835, 38); // 72 DPI
  bmp.writeInt32LE(2835, 42);
  for (let y = 0; y < height; y++) {
    const row = 54 + (height - 1 - y) * rowSize;
    for (let x = 0; x < width; x++) {
      const i = (y * width + x) * 4;
      bmp[row + x * 3] = pixels[i + 2]; // B
      bmp[row + x * 3 + 1] = pixels[i + 1]; // G
      bmp[row + x * 3 + 2] = pixels[i]; // R
    }
  }
  return bmp;
}

const render = (svg) => new Resvg(svg, { font: { loadSystemFonts: true, defaultFontFamily: "Segoe UI" } }).render();

fs.mkdirSync(OUT, { recursive: true });
fs.writeFileSync(path.join(OUT, "sidebar.bmp"), toBmp(sidebar));
fs.writeFileSync(path.join(OUT, "header.bmp"), toBmp(header));
console.log("Installer artwork written to", OUT);

fs.mkdirSync(DOCS, { recursive: true });
fs.writeFileSync(path.join(DOCS, "banner.png"), render(banner).asPng());
console.log("README banner written to", path.join(DOCS, "banner.png"));
