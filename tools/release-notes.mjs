// Release helper used by CI when a version tag (v1.2.3) is pushed.
//
//   node tools/release-notes.mjs --check 1.2.3   fails unless every version field equals 1.2.3
//   node tools/release-notes.mjs 1.2.3           prints the GitHub release notes (Markdown)
//
// The notes come from the README changelog entry "- **1.2.3** ...".

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const readText = (file) => fs.readFileSync(path.join(ROOT, file), "utf8");

function appVersions() {
  const cargo = readText("src-tauri/Cargo.toml").match(/^version\s*=\s*"([^"]+)"/m);
  return {
    "package.json": JSON.parse(readText("package.json")).version,
    "src-tauri/tauri.conf.json": JSON.parse(readText("src-tauri/tauri.conf.json")).version,
    "src-tauri/Cargo.toml": cargo?.[1],
  };
}

/** The README changelog entry of a version, as Markdown (without the version heading). */
function changelogEntry(version) {
  const lines = readText("README.md").replace(/\r\n/g, "\n").split("\n");
  const start = lines.findIndex((l) => l.startsWith(`- **${version}**`));
  if (start < 0) return null;
  const entry = [lines[start].replace(`- **${version}**`, "").replace(/^[\s,:-]+/, "")];
  for (const line of lines.slice(start + 1)) {
    if (!line.startsWith("  ")) break; // next entry or end of the list
    entry.push(line.slice(2));
  }
  // Re-join wrapped list items so GitHub renders them as single bullets
  return entry
    .join("\n")
    .replace(/\n {2}(?!-)/g, " ")
    .trim();
}

function notes(version) {
  const entry = changelogEntry(version);
  if (!entry) throw new Error(`No changelog entry for ${version} in README.md (expected a line "- **${version}** ...")`);
  const [rawSummary, ...items] = entry.split("\n");
  // "(first release), with ..." -> "First release, with ..."
  const summary = rawSummary.replace(/^\(([^)]*)\)/, "$1").replace(/^./, (c) => c.toUpperCase());
  return `${summary ? `**${summary}**\n\n` : ""}${items.join("\n")}

## Downloads

| System | File |
|---|---|
| Windows 10/11 | \`RivalGUI_${version}_x64-setup.exe\` (recommended) or the \`.msi\` |
| Debian / Ubuntu | \`.deb\` (includes the udev rules) |
| Fedora / openSUSE | \`.rpm\` (includes the udev rules) |
| Other Linux | \`.AppImage\` (install the udev rules once from *Preferences*) |

**Windows:** the installer is not code-signed yet, so SmartScreen may show "Windows protected your PC".
Click **More info → Run anyway**. Installing over an older version upgrades it and keeps your settings.

**Linux:** the tray icon needs AppIndicator support; on GNOME install the
"AppIndicator and KStatusNotifierItem Support" extension.

Verify a download with \`SHA256SUMS.txt\` (\`sha256sum -c SHA256SUMS.txt --ignore-missing\` on Linux,
\`Get-FileHash <file>\` in PowerShell).

---

RivalGUI is an independent project, not affiliated with or endorsed by SteelSeries. Device support is
based on [rivalcfg](https://github.com/flozz/rivalcfg).
`;
}

const args = process.argv.slice(2);
if (args[0] === "--check") {
  const wanted = args[1];
  const mismatched = Object.entries(appVersions()).filter(([, v]) => v !== wanted);
  if (!wanted || mismatched.length) {
    for (const [file, v] of mismatched) console.error(`${file} has version ${v}, the tag wants ${wanted}`);
    process.exit(1);
  }
  if (!changelogEntry(wanted)) {
    console.error(`README.md has no changelog entry for ${wanted}`);
    process.exit(1);
  }
  console.log(`All versions are ${wanted} and the changelog has an entry for it.`);
} else if (args[0]) {
  process.stdout.write(notes(args[0]));
} else {
  console.error("usage: node tools/release-notes.mjs [--check] <version>");
  process.exit(1);
}
