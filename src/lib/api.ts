import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { translateError } from "./i18n/index.svelte";

export type Json = null | boolean | number | string | Json[] | { [key: string]: Json };

export interface Battery {
  level: number | null;
  charging: boolean | null;
}

export interface DeviceStatus {
  key: string;
  name: string;
  vendor_id: number;
  product_id: number;
  wireless: boolean;
  online: boolean;
  battery: Battery | null;
  firmware: string | null;
  can_save: boolean;
  active_profile: string;
  error: string | null;
}

export interface Stop {
  pos: number;
  color: string;
}

export type UiValue =
  | { kind: "choice"; value: string }
  | { kind: "range"; value: number }
  | { kind: "dpi"; values: number[] }
  | { kind: "dpi_xy"; values: [number, number][] }
  | { kind: "color"; color: string }
  | { kind: "reactive"; color: string | null }
  | { kind: "gradient"; is_gradient: boolean; duration: number; stops: Stop[] }
  | { kind: "buttons"; mapping: { button: string; value: string }[] }
  | { kind: "action" }
  | { kind: "invalid"; error: string };

export interface ButtonTargets {
  buttons: string[];
  specials: string[];
  keyboard: boolean;
  multimedia: boolean;
  defaults: Record<string, string>;
}

export interface SettingView {
  name: string;
  label: string;
  description: string;
  value_type: string;
  group: "performance" | "lighting" | "buttons" | "power";
  value: Json;
  is_set: boolean;
  default: Json | null;
  ui: UiValue;
  choices: string[] | null;
  range: [number, number, number] | null;
  max_presets: number | null;
  max_stops: number | null;
  button_targets: ButtonTargets | null;
}

export interface DeviceDetail {
  status: DeviceStatus;
  settings: SettingView[];
  profiles: string[];
  active: string;
}

export interface Prefs {
  /** "auto", "en" or "pl" */
  language: string;
  auto_apply: boolean;
  notify_low_battery: boolean;
  low_battery_threshold: number;
}

export interface AppInfo {
  version: string;
  platform: string;
  dry_run: boolean;
  hid_error: string | null;
  config_path: string | null;
  supported_devices: number;
  qwerty_keys: string[];
  multimedia_keys: string[];
  prefs: Prefs;
  autostart: boolean;
}

export const api = {
  appInfo: () => invoke<AppInfo>("get_app_info"),
  listDevices: () => invoke<DeviceStatus[]>("list_devices"),
  device: (key: string) => invoke<DeviceDetail>("get_device", { key }),
  setSetting: (key: string, name: string, value: Json) =>
    invoke<SettingView>("set_setting", { key, name, value }),
  resetSetting: (key: string, name: string) => invoke<SettingView>("reset_setting", { key, name }),
  runAction: (key: string, name: string) => invoke<void>("run_action", { key, name }),
  saveToDevice: (key: string) => invoke<void>("save_to_device", { key }),
  applyProfile: (key: string) => invoke<string[]>("apply_profile", { key }),
  resetProfile: (key: string) => invoke<DeviceDetail>("reset_profile", { key }),
  activateProfile: (key: string, name: string) => invoke<DeviceDetail>("activate_profile", { key, name }),
  createProfile: (key: string, name: string, copyFrom: string | null) =>
    invoke<DeviceDetail>("create_profile", { key, name, copyFrom }),
  renameProfile: (key: string, oldName: string, newName: string) =>
    invoke<DeviceDetail>("rename_profile", { key, old: oldName, new: newName }),
  deleteProfile: (key: string, name: string) => invoke<DeviceDetail>("delete_profile", { key, name }),
  setPrefs: (prefs: Prefs) => invoke<Prefs>("set_prefs", { prefs }),
  setAutostart: (enabled: boolean) => invoke<boolean>("set_autostart", { enabled }),
  installUdevRules: () => invoke<void>("install_udev_rules"),
  onDevicesChanged: (cb: (devices: DeviceStatus[]) => void): Promise<UnlistenFn> =>
    listen<DeviceStatus[]>("devices-changed", (e) => cb(e.payload)),
};

export function errorText(e: unknown): string {
  const text = typeof e === "string" ? e : e instanceof Error ? e.message : JSON.stringify(e);
  return translateError(text);
}
