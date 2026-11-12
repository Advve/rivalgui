// Writes latest.json, the manifest the in-app updater (tauri-plugin-updater) reads from the latest
// GitHub release. CI runs it in the release job, after collecting the installers and their .sig files.
//
//   node tools/updater-manifest.mjs 1.2.3 release > release/latest.json
//
// Each installer type gets its own key, so every install updates with the package it came from:
// windows-x86_64-nsis / -msi, linux-x86_64-appimage / -deb / -rpm. The plain windows-x86_64 and
// linux-x86_64 keys (NSIS, AppImage) are the fallback for anything else. An installer without a
// .sig is left out; with none at all it fails, so a release is never published without updates.

import fs from "node:fs";
import path from "node:path";

const [version, dir] = process.argv.slice(2);
if (!/^\d+\.\d+\.\d+$/.test(version ?? "") || !dir) {
  console.error("usage: node tools/updater-manifest.mjs X.Y.Z <folder with installers and .sig files>");
  process.exit(1);
}

const kinds = [
  { key: "windows-x86_64-nsis", match: (f) => f.endsWith("-setup.exe") },
  { key: "windows-x86_64-msi", match: (f) => f.endsWith(".msi") },
  { key: "linux-x86_64-appimage", match: (f) => f.endsWith(".AppImage") },
  { key: "linux-x86_64-deb", match: (f) => f.endsWith(".deb") },
  { key: "linux-x86_64-rpm", match: (f) => f.endsWith(".rpm") },
];
const fallbacks = { "windows-x86_64": "windows-x86_64-nsis", "linux-x86_64": "linux-x86_64-appimage" };

const files = fs.readdirSync(dir);
const platforms = {};
for (const { key, match } of kinds) {
  const file = files.find((f) => match(f) && files.includes(f + ".sig"));
  if (!file) {
    console.error(`no signed installer for ${key}`);
    continue;
  }
  platforms[key] = {
    signature: fs.readFileSync(path.join(dir, file + ".sig"), "utf8").trim(),
    url: `https://github.com/Advve/rivalgui/releases/download/v${version}/${encodeURIComponent(file)}`,
  };
}
for (const [key, from] of Object.entries(fallbacks)) {
  if (platforms[from]) platforms[key] = platforms[from];
}
if (Object.keys(platforms).length === 0) {
  console.error("no signed installers found: is TAURI_SIGNING_PRIVATE_KEY set for the build?");
  process.exit(1);
}

const manifest = {
  version,
  notes: `RivalGUI ${version}: https://github.com/Advve/rivalgui/releases/tag/v${version}`,
  pub_date: new Date().toISOString(),
  platforms,
};
process.stdout.write(JSON.stringify(manifest, null, 2) + "\n");
