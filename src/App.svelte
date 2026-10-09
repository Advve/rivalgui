<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorText, type AppInfo, type DeviceStatus } from "./lib/api";
  import { toasts } from "./lib/toast.svelte";
  import Sidebar from "./lib/Sidebar.svelte";
  import DevicePanel from "./lib/DevicePanel.svelte";
  import Preferences from "./lib/Preferences.svelte";
  import Icon from "./lib/Icon.svelte";
  import { setLanguage, t } from "./lib/i18n/index.svelte";

  let info = $state<AppInfo | null>(null);
  let devices = $state<DeviceStatus[]>([]);
  let selected = $state<string | null>(null);
  let page = $state<"device" | "settings">("device");
  let fatal = $state<string | null>(null);

  const current = $derived(devices.find((d) => d.key === selected) ?? null);

  let mainEl = $state<HTMLElement>();
  // Start each page at the top
  $effect(() => {
    void page;
    void selected;
    if (mainEl) mainEl.scrollTop = 0;
  });

  function setDevices(list: DeviceStatus[]) {
    devices = list;
    // Keep the selection; otherwise pick the first device
    if (!list.some((d) => d.key === selected)) selected = list[0]?.key ?? null;
  }

  onMount(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      try {
        unlisten = await api.onDevicesChanged(setDevices);
        info = await api.appInfo();
        setLanguage(info.prefs.language);
        setDevices(await api.listDevices());
      } catch (e) {
        fatal = errorText(e);
      }
    })();
    return () => unlisten?.();
  });
</script>

<div class="app">
  <Sidebar
    {devices}
    {selected}
    {page}
    onselect={(k) => {
      selected = k;
      page = "device";
    }}
    onsettings={() => (page = "settings")}
  />

  <main bind:this={mainEl}>
    {#if fatal}
      <div class="empty"><p class="bad">{fatal}</p></div>
    {:else if !info}
      <div class="empty"></div>
    {:else if page === "settings"}
      <Preferences
        {info}
        onchange={(i) => {
          info = i;
          setLanguage(i.prefs.language);
        }}
      />
    {:else if current}
      <DevicePanel status={current} {info} />
    {:else}
      <div class="empty">
        <div class="art"><Icon name="mouse" size={44} /></div>
        <h1>{t("emptyTitle")}</h1>
        <p class="muted">{t("emptyBody", { count: info.supported_devices })}</p>
        {#if info.platform === "linux"}
          <p class="faint">
            {t("emptyLinuxBefore")}
            <button class="link" onclick={() => (page = "settings")}>{t("preferences")}</button>.
          </p>
        {/if}
        {#if info.hid_error}<p class="bad">{t("hidError", { error: info.hid_error })}</p>{/if}
      </div>
    {/if}
  </main>
</div>

<div class="toasts" aria-live="polite">
  {#each toasts as t (t.id)}
    <div class="toast {t.kind}">{t.text}</div>
  {/each}
</div>

<style>
  .app {
    display: flex;
    height: 100%;
  }
  main {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
  }
  .empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 40px;
    gap: 4px;
  }
  .empty h1 {
    margin: 18px 0 4px;
    font-size: 20px;
  }
  .empty p {
    max-width: 440px;
    margin: 4px 0;
  }
  .art {
    width: 96px;
    height: 96px;
    display: grid;
    place-items: center;
    border-radius: 28px;
    background: var(--accent-soft);
    color: var(--accent);
    animation: breathe 2.4s ease-in-out infinite;
  }
  @keyframes breathe {
    50% {
      transform: scale(1.06);
      box-shadow: 0 0 0 12px color-mix(in srgb, var(--accent) 6%, transparent);
    }
  }
  .bad {
    color: var(--bad);
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--accent);
    text-decoration: underline;
  }
  .toasts {
    position: fixed;
    right: 18px;
    bottom: 18px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 10;
    pointer-events: none;
  }
  .toast {
    max-width: 380px;
    padding: 10px 14px;
    border-radius: var(--radius-sm);
    background: var(--panel-2);
    border: 1px solid var(--line);
    box-shadow: var(--shadow);
    font-size: 13px;
    white-space: pre-line;
    animation: slide 0.18s ease-out;
  }
  .toast.error {
    border-color: var(--bad);
  }
  .toast.ok {
    border-color: var(--ok);
  }
  @keyframes slide {
    from {
      transform: translateY(8px);
      opacity: 0;
    }
  }
</style>
