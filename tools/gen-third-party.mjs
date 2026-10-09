// Generates THIRD-PARTY-NOTICES.txt: the licenses of everything shipped in RivalGUI.
//
// - Rust crates linked into the app (normal dependencies) for Windows and Linux,
//   from `cargo metadata`, with the license/notice files found in each crate.
// - JavaScript packages actually bundled into the UI, found by building the UI
//   with source maps and listing the node_modules files it contains.
// - Other bundled or derived work (rivalcfg data, NSIS installer).
//
// Usage: npm run notices

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "vite";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const TAURI_DIR = path.join(ROOT, "src-tauri");
const OUTPUT = path.join(ROOT, "THIRD-PARTY-NOTICES.txt");
const TARGETS = ["x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu"];
const LICENSE_FILE = /^(licen[cs]e|copying|notice|copyright|unlicense)([-._].*)?$/i;

const MIT_TEMPLATE = `MIT License

Copyright (c) {holders}

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
`;

const EXTRA = [
  {
    name: "rivalcfg",
    url: "https://github.com/flozz/rivalcfg",
    license: "WTFPL",
    text: `            DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
                    Version 2, December 2004

 Copyright (C) 2004 Sam Hocevar <sam@hocevar.net>

 Everyone is permitted to copy and distribute verbatim or modified
 copies of this license document, and changing it is allowed as long
 as the name is changed.

            DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
   TERMS AND CONDITIONS FOR COPYING, DISTRIBUTION AND MODIFICATION

  0. You just DO WHAT THE FUCK YOU WANT TO.
`,
  },
  {
    name: "NSIS (Nullsoft Scriptable Install System)",
    url: "https://nsis.sourceforge.io/License",
    license: "zlib/libpng",
    text: `Copyright (C) 1999-2025 Contributors

This software is provided 'as-is', without any express or implied warranty.
In no event will the authors be held liable for any damages arising from the
use of this software.

Permission is granted to anyone to use this software for any purpose,
including commercial applications, and to alter it and redistribute it
freely, subject to the following restrictions:

1. The origin of this software must not be misrepresented; you must not claim
   that you wrote the original software. If you use this software in a
   product, an acknowledgment in the product documentation would be
   appreciated but is not required.
2. Altered source versions must be plainly marked as such, and must not be
   misrepresented as being the original software.
3. This notice may not be removed or altered from any source distribution.
`,
  },
];

const read = (file) => fs.readFileSync(file, "utf8").trim() + "\n";
const byString = (a, b) => (a < b ? -1 : a > b ? 1 : 0);

function licenseFiles(directory) {
  return fs
    .readdirSync(directory)
    .sort(byString)
    .map((name) => path.join(directory, name))
    .filter((file) => LICENSE_FILE.test(path.basename(file)) && fs.statSync(file).isFile());
}

/** SPDX-ish expression -> alternatives ("MIT OR Apache-2.0", "MIT/Apache-2.0"). */
function licenseOptions(expr) {
  if (!expr || expr.includes(" AND ")) return [expr || ""];
  return expr
    .split(/\s+OR\s+|\//)
    .map((o) => o.trim())
    .filter(Boolean);
}

const isMitText = (file, text) =>
  path.basename(file).toLowerCase().includes("mit") || text.includes("Permission is hereby granted, free of charge");

/**
 * License/notice texts to reproduce for a package. For packages offering MIT as
 * one of their alternatives, RivalGUI uses them under MIT, so only the MIT text
 * (with the package's copyright line) is kept.
 */
function licenseTexts(directory, expr, holders) {
  const files = licenseFiles(directory).map((file) => [file, read(file)]);
  if (licenseOptions(expr).includes("MIT")) {
    const mit = files.filter(([file, text]) => isMitText(file, text)).map(([, text]) => text);
    return mit.length ? mit : [MIT_TEMPLATE.replace("{holders}", holders || "the authors")];
  }
  return files.map(([, text]) => text);
}

const normalized = (text) => text.replace(/\s+/g, " ").trim();

/**
 * Packages that ship no license file: borrow from the same project, or from a
 * package under the same copyright-less license (e.g. the MPL-2.0 text).
 */
function fillMissingTexts(entries) {
  const byName = new Map(entries.map((e) => [e.name, e]));
  const siblings = { "alloc-stdlib": "alloc-no-stdlib" }; // same repository and authors
  const generic = new Set(["MPL-2.0"]); // license texts without a per-project copyright line
  for (const e of entries) {
    if (e.texts?.length) continue;
    let source = byName.get(siblings[e.name] ?? "");
    if (!source && generic.has(e.license)) {
      source = entries.find((o) => o.license === e.license && o.texts?.length);
    }
    if (source) {
      e.texts = source.texts;
      e.note = `No license file in the package; text taken from ${source.name}.`;
    }
  }
}

// --------------------------------------------------------------------- Rust

function rustPackages() {
  const packages = new Map();
  for (const target of TARGETS) {
    const out = execFileSync("cargo", ["metadata", "--format-version", "1", "--filter-platform", target], {
      cwd: TAURI_DIR,
      maxBuffer: 256 * 1024 * 1024,
    });
    const meta = JSON.parse(out.toString("utf8"));
    const byId = new Map(meta.packages.map((p) => [p.id, p]));
    const nodes = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
    // Walk normal (linked) dependencies only; build/dev dependencies are not shipped
    const seen = new Set();
    const stack = [meta.resolve.root];
    while (stack.length) {
      for (const dep of nodes.get(stack.pop()).deps) {
        if (dep.dep_kinds.some((k) => k.kind === null) && !seen.has(dep.pkg)) {
          seen.add(dep.pkg);
          stack.push(dep.pkg);
        }
      }
    }
    for (const id of seen) {
      const p = byId.get(id);
      packages.set(`${p.name}\u0000${p.version}`, p);
    }
  }
  return [...packages.values()]
    .sort((a, b) => byString(a.name, b.name) || byString(a.version, b.version))
    .map((p) => {
      const authors = (p.authors ?? []).map((a) => a.replace(/\s*<[^>]*>/g, "")).join(", ");
      return {
        name: p.name,
        version: p.version,
        license: p.license || (p.license_file ? `see ${p.license_file}` : "unknown"),
        url: p.repository || p.homepage || `https://crates.io/crates/${p.name}`,
        texts: licenseTexts(path.dirname(p.manifest_path), p.license, authors),
      };
    });
}

// --------------------------------------------------------------- JavaScript

/** Packages whose files end up in the built UI bundle (via source maps). */
async function jsPackages() {
  const outDir = fs.mkdtempSync(path.join(os.tmpdir(), "rivalgui-licenses-"));
  const packageDirs = new Set();
  try {
    await build({ root: ROOT, logLevel: "error", build: { outDir, emptyOutDir: true, sourcemap: true } });
    for (const file of fs.readdirSync(outDir, { recursive: true })) {
      if (!String(file).endsWith(".map")) continue;
      const { sources = [] } = JSON.parse(fs.readFileSync(path.join(outDir, String(file)), "utf8"));
      for (const source of sources) {
        const src = source.replaceAll("\\", "/");
        if (!src.includes("node_modules/")) continue;
        const rest = src.slice(src.lastIndexOf("node_modules/") + "node_modules/".length).split("/");
        const name = rest[0].startsWith("@") ? rest.slice(0, 2).join("/") : rest[0];
        packageDirs.add(path.join(ROOT, "node_modules", ...name.split("/")));
      }
    }
  } finally {
    fs.rmSync(outDir, { recursive: true, force: true });
  }

  return [...packageDirs].sort(byString).map((directory) => {
    const pkg = JSON.parse(fs.readFileSync(path.join(directory, "package.json"), "utf8"));
    const author = typeof pkg.author === "object" ? pkg.author?.name : pkg.author;
    const repo = typeof pkg.repository === "object" ? pkg.repository?.url : pkg.repository;
    return {
      name: pkg.name,
      version: pkg.version ?? "",
      license: pkg.license || "unknown",
      url: repo || pkg.homepage || `https://www.npmjs.com/package/${pkg.name}`,
      texts: licenseTexts(directory, pkg.license, author),
    };
  });
}

// ------------------------------------------------------------------- output

const SECTIONS = ["Other components", "Rust crates", "JavaScript packages (UI)"];
const label = (e) => `${e.name}${e.version ? " " + e.version : ""}`;

async function main() {
  const rust = rustPackages();
  const js = await jsPackages();
  const entries = [
    ...EXTRA.map((e) => ({ ...e, section: SECTIONS[0] })),
    ...rust.map((e) => ({ ...e, section: SECTIONS[1] })),
    ...js.map((e) => ({ ...e, section: SECTIONS[2] })),
  ];
  fillMissingTexts(entries);

  // Group identical license texts (ignoring whitespace) so each one is printed once
  const groups = new Map();
  for (const e of entries) {
    const texts = e.texts?.length ? e.texts : e.text ? [e.text] : [];
    const key = texts.length
      ? createHash("sha256").update(texts.map(normalized).join("\n----\n")).digest("hex")
      : "none";
    if (!groups.has(key)) groups.set(key, { texts, entries: [] });
    groups.get(key).entries.push(e);
  }

  const lines = [
    "RivalGUI - third-party notices",
    "==============================",
    "",
    "RivalGUI is distributed under the MIT License (see LICENSE). It includes or is",
    "derived from the third-party software listed below, under their own licenses.",
    "Where a package offers a choice of licenses that includes MIT, it is used under",
    "the MIT License. MPL-2.0 components are unmodified; their source code is",
    "available at the URLs listed below.",
    "This file is generated by tools/gen-third-party.mjs (npm run notices).",
    "",
    "Summary",
    "-------",
  ];
  for (const section of SECTIONS) {
    const items = entries.filter((e) => e.section === section);
    lines.push("", `${section} (${items.length}):`);
    for (const e of items) lines.push(`  - ${label(e)} (${e.license}) ${e.url}`);
  }

  lines.push("", "", "License texts", "-------------");
  const missing = [];
  for (const group of groups.values()) {
    lines.push("", "=".repeat(79), `Applies to: ${group.entries.map(label).join(", ")}`, "=".repeat(79), "");
    lines.push(...group.entries.filter((e) => e.note).map((e) => e.note));
    if (group.texts.length) {
      lines.push(group.texts.join("\n---\n\n").trimEnd());
    } else {
      missing.push(...group.entries.map((e) => e.name));
      const licenses = [...new Set(group.entries.map((e) => e.license))].sort(byString);
      lines.push(`No license file shipped by the package; license: ${licenses.join(", ")}.`);
    }
  }

  fs.writeFileSync(OUTPUT, lines.join("\n") + "\n", "utf8");
  console.log(`${rust.length} Rust crates, ${js.length} JS packages, ${groups.size} distinct license texts -> ${OUTPUT}`);
  if (missing.length) console.log("No license text found for:", [...new Set(missing)].sort(byString).join(", "));
}

await main();
