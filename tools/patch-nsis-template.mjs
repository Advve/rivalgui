// Applies the RivalGUI changes to an upstream Tauri NSIS template.
//
// Usage: node tools/patch-nsis-template.mjs <upstream installer.nsi> src-tauri/windows/installer.nsi
// Upstream: https://github.com/tauri-apps/tauri/blob/tauri-cli-vX.Y.Z/crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi
//
// NSIS uses ${...} for macros, so the snippets below are plain quoted lines (no template literals).

import fs from "node:fs";

const [src, dst] = process.argv.slice(2);
if (!src || !dst) {
  console.error("usage: node tools/patch-nsis-template.mjs <upstream installer.nsi> <output>");
  process.exit(1);
}

const lines = (...l) => l.join("\n") + "\n";

const header = lines(
  "; RivalGUI installer: Tauri's NSIS template (tauri-cli v2.12.1) with one change:",
  "; upgrading from an older RivalGUI version uninstalls it automatically (no prompt)",
  "; before installing the new one. Settings (%APPDATA%\\rivalgui), shortcuts and the",
  "; autostart entry are kept. Same-version reinstalls and downgrades still show",
  '; the standard maintenance page. Search for "RivalGUI" to find the changes.',
  ";",
  "; Original template: https://github.com/tauri-apps/tauri (crates/tauri-bundler),",
  "; licensed under MIT or Apache-2.0; used here under the MIT License:",
  ";",
  "; Copyright (c) 2017 - Present Tauri Apps Contributors",
  ";",
  "; Permission is hereby granted, free of charge, to any person obtaining a copy",
  '; of this software and associated documentation files (the "Software"), to deal',
  "; in the Software without restriction, including without limitation the rights",
  "; to use, copy, modify, merge, publish, distribute, sublicense, and/or sell",
  "; copies of the Software, and to permit persons to whom the Software is",
  "; furnished to do so, subject to the following conditions:",
  ";",
  "; The above copyright notice and this permission notice shall be included in all",
  "; copies or substantial portions of the Software.",
  ";",
  '; THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR',
  "; IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,",
  "; FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE",
  "; AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER",
  "; LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,",
  "; OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE",
  "; SOFTWARE.",
  ";",
  "; RivalGUI changes: Copyright (c) 2026 Adve, MIT License.",
);

const autoUpgrade = lines(
  "  ; RivalGUI: upgrading from an older version -> uninstall it silently and skip this page",
  "  ${If} $R0 = 1",
  "  ${AndIf} $WixMode <> 1",
  "  ${AndIf} $UpdateMode <> 1",
  "  ${AndIf} $AutoUpgradeFailed <> 1",
  "    Call AutoUninstallOlderVersion",
  "    ${If} $AutoUpgradeFailed <> 1",
  "      Abort ; skip the page, go on with the installation",
  "    ${EndIf}",
  "    ; Failed (e.g. the user refused to close the app): fall back to the standard page",
  "  ${EndIf}",
  "",
);

const autoUninstallFunction = lines(
  "; RivalGUI: runs the previous version's uninstaller in update + passive mode",
  "; (keeps shortcuts, autostart and app data, closes the running app). Sets",
  "; $AutoUpgradeFailed to 1 if it did not succeed.",
  "Function AutoUninstallOlderVersion",
  '  ReadRegStr $5 SHCTX "${MANUPRODUCTKEY}" ""',
  '  ReadRegStr $6 SHCTX "${UNINSTKEY}" "UninstallString"',
  '  ${If} $6 == ""',
  "    StrCpy $AutoUpgradeFailed 1",
  "    Return",
  "  ${EndIf}",
  '  ${IfThen} $5 == "" ${|} StrCpy $5 "$INSTDIR" ${|}',
  "",
  "  HideWindow",
  "  ClearErrors",
  "  ExecWait '$6 /UPDATE /P _?=$5' $7",
  "  ${IfThen} ${Errors} ${|} StrCpy $7 2 ${|}",
  "  BringToFront",
  "",
  "  ${If} $7 <> 0",
  '  ${OrIf} ${FileExists} "$5\\${MAINBINARYNAME}.exe"',
  "    StrCpy $AutoUpgradeFailed 1",
  "  ${Else}",
  "    ; The old uninstaller cannot delete itself when run with _?=",
  '    Delete "$5\\uninstall.exe"',
  '    RMDir "$5"',
  "  ${EndIf}",
  "FunctionEnd",
  "",
);

const uninstallerIcon = lines(
  '!if "${UNINSTALLERICON}" != ""',
  '  !define MUI_UNICON "${UNINSTALLERICON}"',
  "!endif",
);
const uninstallerIconPatched = lines(
  '!if "${UNINSTALLERICON}" != ""',
  '  !define MUI_UNICON "${UNINSTALLERICON}"',
  '!else if "${INSTALLERICON}" != ""',
  "  ; RivalGUI: Tauri has no uninstaller icon option; reuse the installer icon",
  '  !define MUI_UNICON "${INSTALLERICON}"',
  "!endif",
);

/** Inserts `text` before (or replaces) a unique anchor of the upstream template. */
function patch(s, anchor, replacement) {
  const count = s.split(anchor).length - 1;
  if (count !== 1) throw new Error(`anchor found ${count} times (expected 1): ${JSON.stringify(anchor)}`);
  return s.replace(anchor, () => replacement);
}

let s = header + fs.readFileSync(src, "utf8").replace(/\r\n/g, "\n");
s = patch(s, "Var ReinstallPageCheck\n", "Var ReinstallPageCheck\nVar AutoUpgradeFailed ; RivalGUI\n");
s = patch(s, "  ; Skip showing the page if passive\n", autoUpgrade + "  ; Skip showing the page if passive\n");
s = patch(s, "Function PageReinstallUpdateSelection\n", autoUninstallFunction + "Function PageReinstallUpdateSelection\n");
s = patch(s, uninstallerIcon, uninstallerIconPatched);

fs.writeFileSync(dst, s, "utf8");
console.log("patched ->", dst);
