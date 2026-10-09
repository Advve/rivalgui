<script lang="ts">
  import type { DeviceStatus } from "./api";
  import { splitName } from "./format";
  import BatteryBadge from "./BatteryBadge.svelte";
  import Icon from "./Icon.svelte";
  import { t, td } from "./i18n/index.svelte";

  let {
    devices,
    selected,
    page,
    onselect,
    onsettings,
  }: {
    devices: DeviceStatus[];
    selected: string | null;
    page: "device" | "settings";
    onselect: (key: string) => void;
    onsettings: () => void;
  } = $props();
</script>

<aside>
  <div class="brand">
    <span class="logo"><Icon name="mouse" size={18} /></span>
    <span>RivalGUI</span>
  </div>

  <div class="section faint">{t("devices")}</div>
  <nav>
    {#each devices as d (d.key)}
      {@const n = splitName(d.name)}
      <button class="device" class:active={page === "device" && d.key === selected} onclick={() => onselect(d.key)}>
        <span class="status" class:online={d.online} class:error={!!d.error}></span>
        <span class="text">
          <span class="name">{n.title}</span>
          <span class="sub">
            {#if d.wireless}<BatteryBadge device={d} />{:else}<span class="faint">{n.mode ? td(n.mode) : t("wired")}</span>{/if}
          </span>
        </span>
      </button>
    {:else}
      <div class="empty faint">{t("waitingForMouse")}</div>
    {/each}
  </nav>

  <div class="grow"></div>
  <button class="device settings" class:active={page === "settings"} onclick={onsettings}>
    <Icon name="settings" /> {t("preferences")}
  </button>
</aside>

<style>
  aside {
    width: 250px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 18px 12px 12px;
    background: var(--bg-elev);
    border-right: 1px solid var(--line-soft);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 8px 18px;
    font-weight: 700;
    font-size: 16px;
    letter-spacing: -0.01em;
  }
  .logo {
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    border-radius: 9px;
    background: var(--accent-grad);
    color: #fff;
  }
  .section {
    padding: 0 10px 4px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-y: auto;
  }
  .device {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 9px 10px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    text-align: left;
    color: var(--text-dim);
  }
  .device:hover {
    background: var(--panel-2);
    color: var(--text);
  }
  .device.active {
    background: var(--accent-soft);
    color: var(--text);
  }
  .status {
    width: 8px;
    height: 8px;
    flex: none;
    border-radius: 50%;
    background: var(--text-faint);
  }
  .status.online {
    background: var(--ok);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--ok) 20%, transparent);
  }
  .status.error {
    background: var(--bad);
  }
  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .name {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sub {
    font-size: 12px;
  }
  .empty {
    padding: 8px 10px;
    font-size: 13px;
  }
  .grow {
    flex: 1;
  }
  .settings {
    gap: 8px;
  }
</style>
