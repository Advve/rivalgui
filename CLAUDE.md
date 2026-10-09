# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

RivalGUI is a Tauri 2 desktop app (Rust backend, Svelte 5 + TypeScript UI) that configures SteelSeries mice on
Windows and Linux. It has a tray background service. Device support is a **Rust port of
[rivalcfg](https://github.com/flozz/rivalcfg)** (currently in sync with rivalcfg 4.17.0). The Python rivalcfg
source is *not* part of this repo; it is only used by `tools/` to export data and generate test vectors.

## Commands

On Windows Git Bash, `~/.cargo/bin` may not be on PATH: `export PATH="$HOME/.cargo/bin:$PATH"`.

```bash
npm install
npm run tauri dev                                   # run the app (Vite on :1420 + cargo run)
npm run check                                       # svelte-check / TypeScript
npx tauri build --bundles nsis                      # Windows installer -> src-tauri/target/release/bundle/nsis/
cargo test --manifest-path src-tauri/Cargo.toml     # all tests (unit + golden)
cargo test --manifest-path src-tauri/Cargo.toml --test golden -- --nocapture   # golden only, prints case count
cargo test --manifest-path src-tauri/Cargo.toml --lib encode::range            # one unit-test module
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
```

Simulation without hardware: `RIVALGUI_DRY=1` fakes an Aerox 3 Wireless, a Rival 3 and a Rival 600.
`RIVALGUI_DRY=1038:1872,1038:1729` picks specific vid:pid. In both cases packets are printed to stderr as
`[dry vvvv:pppp] <report_type> <bytes>` instead of being sent. `RIVALGUI_CONFIG_DIR` points the config at a
throwaway directory.

**Single instance:** the app is single-instance, keyed by the bundle identifier. If an installed RivalGUI is
already running, `tauri dev` just focuses it and exits with code 0. Run dev builds with a different identifier:
`npx tauri dev --config '{"identifier":"io.github.rivalgui.devtest"}'`.

**Driving the UI programmatically:** set `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`
and talk CDP to `http://127.0.0.1:9222/json`, using `Runtime.evaluate` and `Page.captureScreenshot`. Svelte 5
delegates events, so synthetic events need `{bubbles: true}`.

## Architecture

### Data flow from rivalcfg

- `tools/export_profiles.py` imports rivalcfg with a stubbed `hid` module and writes two files:
  - `src-tauri/resources/profiles.json`: every device profile expanded per USB PID, plus the keyboard and
    multimedia key layouts. It is embedded with `include_str!` and parsed once (`core::profile::catalog()`).
    - Python dict keys that are ints (`choices`, `output_choices`) are exported as lists to keep their type
      and order.
    - Battery lambdas become `battery_level.format: "aerox" | "rival"`.
  - `src-tauri/resources/99-steelseries-rival.rules`, the udev rules. They are embedded for the in-app
    installer and shipped in the .deb/.rpm.
- `tools/gen_golden.py` drives rivalcfg's `Mouse` + `FakeDevice` over every PID × setting × representative
  values. Invalid inputs are included and are expected to fail. Output goes to `src-tauri/tests/golden.json`.
- `tools/_rivalcfg_import.py` picks the newest `rivalcfg-X.Y.Z` folder next to the repo or in `~/Downloads`,
  or `RIVALCFG_SRC` if set. Use the system `python`.
- These two rivalcfg sync scripts are the only Python in the repo, because they must run rivalcfg itself.
  Everything else (app, build, CI, other tools) is Node + Rust; keep it that way.

### Upgrading rivalcfg

Re-run both tools, then `cargo test`. A new rivalcfg handler shows up as a golden failure
`Unknown handler '<name>'`. To add it:
1. Port it to `src-tauri/src/core/encode/` and wire it in `encode_value()`.
2. Map it to a `UiValue` kind in `core/view.rs`.
3. Add a control or reuse one in `src/lib/SettingRow.svelte`.
4. Add value generators for it in `gen_golden.py`.

### Rust backend (`src-tauri/src/`)

- **`core/`**: pure device engine, no Tauri.
  - `encode/` mirrors `rivalcfg/handlers/*`, one module per value type. `mod.rs` also reproduces
    `Mouse._hid_write` framing: report id `0x00` + command + data + suffix, zero-padded to `packet_length`,
    bytes checked to be in range 0–255.
  - `pyval.rs` deliberately replicates Python semantics (`str()`, `int()`, `parse_param_string`, colors, and
    the integer-overflow check), including rivalcfg quirks. **The golden test requires byte-exact parity with
    rivalcfg, including which inputs error. Don't "fix" a quirk without regenerating golden from rivalcfg.**
  - `device.rs`: `Device` over a `Transport` trait. It applies a 50 ms delay after each command, then an
    optional readback; battery and firmware queries live here.
  - `hid.rs`: hidapi discovery. It filters vendor 0x1038, keeps profiles found in the catalog, and keeps the
    interfaces whose number matches the profile's `endpoint`.
    - On Windows, vendor-defined usage pages (≥0xFF00) are tried first and the transport rotates on write
      failure.
    - `DryTransport` is the `RIVALGUI_DRY` path.
  - `view.rs` turns a stored value into a UI-friendly `SettingView`/`UiValue`, serialized with
    `kind: snake_case`, and assigns UI groups (performance / lighting / buttons / power).
- **`service/`**: runtime.
  - `state.rs`: `AppState`. All HID I/O goes through `with_device()`, which opens per operation under the HID
    mutex.
  - `store.rs`: `config.json` in `dirs::config_dir()/rivalgui`, holding prefs and per-device named profiles.
  - `watcher.rs`: thread that scans every 2 s for hot-plug. It polls wireless battery every 60 s while online
    and every 4 s while asleep (to catch wake-up), and handles low-battery notification and auto-apply.
  - `tray.rs`: menu, tooltip and a battery gauge drawn into the icon.
- **`commands.rs`**: Tauri IPC commands. Device work runs in `spawn_blocking`.
- **`lib.rs`**:
  - The main window is created with `create: false` and built on demand.
  - Closing the window destroys the webview to save RAM, while `ExitRequested` is prevented so the tray keeps
    running. Only the tray's "Quit" exits.
  - `--minimized` (passed by the autostart entry) skips opening the window.

### Settings semantics

- **Value format:** stored values use rivalcfg CLI syntax, e.g. `"400, 800"`, `"800:900, 1600:1600"`,
  `"#ff0000"`, `"rgbgradient(duration=1000; colors=0%: red, 50%: blue)"`, `"buttons(button1=button1; ...)"`.
  The UI builds these strings in `src/lib/format.ts`; Rust parses them for display.
- **Profile values:** only values the user explicitly set are stored in a profile. Unset settings fall back to
  profile defaults.
- **Auto-apply** on plug-in or wake sends only explicitly set values (`apply_profile(key, false)`), so a mouse
  configured elsewhere isn't reset. Profile switch, "Re-apply" and reset send the full profile, with defaults
  for unset values (`full = true`).
- **Action settings:** `none`-type settings (e.g. rainbow effect) are one-shot actions and are never stored or
  replayed.
- **Wireless offline:** a wireless device counts as online only while its battery query answers. Writes are
  skipped while it is offline, and the active profile is applied on the offline→online transition.

### Frontend (`src/`)

`App.svelte` (sidebar + page) → `DevicePanel.svelte` → `SettingRow.svelte`. `SettingRow` dispatches on
`setting.ui.kind` to `lib/controls/*` and debounces writes. It ignores stale server echoes while edits are
pending. Theming is CSS tokens in `src/app.css`, dark by default with a light media query.

### Translations (English, Polish)

- **Language preference:** stored in `prefs.language` (`"auto"`, `"en"` or `"pl"`). `"auto"` follows the
  system: `navigator.languages` in the UI and `sys-locale` in Rust.
- **UI strings:** `src/lib/i18n/en.ts`, which defines the keys, and `pl.ts`. `pl` is typed `Record<Key, string>`,
  so a missing key is a type error. Use `t("key", {param})` in components.
- **Device data from rivalcfg** (setting labels, descriptions, choice names, media keys, connection modes):
  stays English in `profiles.json`. The UI translates it with `td(englishText)` through `plData` in `pl.ts`.
  - The keys are the label as shown (`view::display_label`) and the description with the trailing
    `(unit)` removed.
  - After a rivalcfg upgrade, check that new labels and descriptions have a `plData` entry. Untranslated text
    falls back to English.
- **Backend errors:** user-facing errors are returned as `@code|arg` (`crate::i18n::coded`) and shown by the
  UI through the `err_<code>` keys. Technical device and encoder errors stay English.
- **Tray and notifications:** `src-tauri/src/i18n.rs` (`tr(lang, Text::...)`). Changing prefs republishes the
  tray. The built-in profile is stored as `"Default"` and only its display name is translated.

### Windows installer

`src-tauri/windows/installer.nsi` is Tauri's upstream NSIS template (tauri-cli v2.12.1), patched by
`tools/patch-nsis-template.mjs` (`npm run installer:template -- <upstream.nsi> <output>`). The patch adds
two things:
- **Auto-upgrade:** when an older version is installed, its uninstaller is run silently with `/UPDATE /P`
  (keeps shortcuts, autostart and settings) and the "already installed" page is skipped.
- **Uninstaller icon:** falls back to the installer icon.

After upgrading `@tauri-apps/cli`, fetch the matching upstream template and re-run the patch script. Installer
artwork and the README banner (`docs/banner.png`) come from `npm run art` (`tools/make-art.mjs`,
SVG rendered with the `@resvg/resvg-js` dev dependency).

## Licensing

- **Project license:** MIT ("Copyright (c) 2026 Adve"); publisher and author is "Adve".
- **Third-party notices:** `THIRD-PARTY-NOTICES.txt` is generated by `npm run notices` (`tools/gen-third-party.mjs`), which
  reads `cargo metadata` and the UI bundle's source maps. Regenerate it whenever dependencies change. It is
  installed next to the app as a bundle resource, together with `LICENSE.txt`.
- **README disclaimer:** keep the "not affiliated with SteelSeries" note. Don't use SteelSeries logos.

## CI

- **Releases:** pushing a `vX.Y.Z` tag runs the `release` job, which publishes a GitHub Release with all
  installers and `SHA256SUMS.txt`.
  - `tools/release-notes.mjs --check X.Y.Z` fails the build early unless `package.json`,
    `tauri.conf.json` and `Cargo.toml` all say X.Y.Z and the README changelog has a `- **X.Y.Z**` entry.
  - The release notes are that changelog entry plus the install notes.

`.github/workflows/build.yml` runs `npm run check`, `cargo test` and the bundle build on Windows
(nsis, msi) and Ubuntu 22.04 (deb, appimage, rpm). Linux needs
`libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libudev-dev`. The Linux build has only
been verified through CI, never locally.
