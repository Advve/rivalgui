<script lang="ts">
  import { api, errorText, type AppInfo, type DeviceDetail, type DeviceStatus, type SettingView } from "./api";
  import { splitName } from "./format";
  import { toast } from "./toast.svelte";
  import BatteryBadge from "./BatteryBadge.svelte";
  import Icon from "./Icon.svelte";
  import ProfileBar from "./ProfileBar.svelte";
  import SettingRow from "./SettingRow.svelte";
  import { profileName, t, td } from "./i18n/index.svelte";
  import type { Key } from "./i18n/en";

  let { status, info }: { status: DeviceStatus; info: AppInfo } = $props();

  let detail = $state<DeviceDetail | null>(null);
  let loadError = $state<string | null>(null);
  let confirmDelete = $state(false);
  let confirmReset = $state(false);
  let busy = $state(false);

  const groups = [
    { id: "performance", title: "groupPerformance" },
    { id: "lighting", title: "groupLighting" },
    { id: "buttons", title: "groupButtons" },
    { id: "power", title: "groupPower" },
  ] as const;

  /** Splits a translated sentence around {name} so the name can be emphasized. */
  function around(key: Key): [string, string] {
    const marker = "\u0000";
    const [before, after = ""] = t(key, { name: marker }).split(marker);
    return [before, after];
  }

  const name = $derived(splitName(status.name));

  async function load(key: string) {
    try {
      detail = await api.device(key);
      loadError = null;
    } catch (e) {
      loadError = errorText(e);
    }
  }

  // Reload when switching device or when the profile changed elsewhere (tray)
  $effect(() => {
    const key = status.key;
    const profile = status.active_profile;
    if (!detail || detail.status.key !== key || detail.active !== profile) {
      confirmDelete = false;
      confirmReset = false;
      load(key);
    }
  });

  function updated(view: SettingView) {
    if (!detail) return;
    const i = detail.settings.findIndex((s) => s.name === view.name);
    if (i >= 0) detail.settings[i] = view;
  }

  async function act<T>(label: string, f: () => Promise<T>, done?: (r: T) => void) {
    busy = true;
    try {
      const r = await f();
      done?.(r);
    } catch (e) {
      toast(`${label}: ${errorText(e)}`, "error", 5000);
    } finally {
      busy = false;
    }
  }

  const setDetail = (d: DeviceDetail) => (detail = d);

  /** Re-apply errors are "<setting label>: <error>". */
  function applyError(e: string) {
    const i = e.indexOf(": ");
    return i > 0 ? `${td(e.slice(0, i))}: ${errorText(e.slice(i + 2))}` : errorText(e);
  }
</script>

<section class="panel">
  <header>
    <div class="hero">
      <div class="glyph"><Icon name="mouse" size={28} /></div>
      <div class="ident">
        <h1>{name.title}</h1>
        <div class="meta">
          <span class="chip">
            <Icon name={/ghz|wireless/i.test(name.mode ?? "") ? "wifi" : "usb"} size={12} />
            {name.mode ? td(name.mode) : t("wired")}
          </span>
          <BatteryBadge device={status} />
          {#if status.firmware}<span class="faint">{t("firmware", { version: status.firmware })}</span>{/if}
          <span class="faint">{status.key}</span>
        </div>
      </div>
    </div>
    <div class="actions">
      <button
        class="btn"
        title={t("reapplyTitle")}
        disabled={busy || !status.online}
        onclick={() =>
          act(t("reapply"), () => api.applyProfile(status.key), (errors) =>
            errors.length
              ? toast(errors.map(applyError).join("\n"), "error", 6000)
              : toast(t("reapplied"), "ok", 2000),
          )}><Icon name="refresh" size={14} /> {t("reapply")}</button
      >
      {#if status.can_save}
        <button
          class="btn primary"
          title={t("saveToMouseTitle")}
          disabled={busy || !status.online}
          onclick={() => act(t("saveToMouse"), () => api.saveToDevice(status.key), () => toast(t("savedToMouse"), "ok"))}
          ><Icon name="save" size={14} /> {t("saveToMouse")}</button
        >
      {/if}
    </div>
  </header>

  {#if status.error}
    <div class="banner bad">
      <Icon name="info" />
      <div>
        {errorText(status.error)}
        {#if info.platform === "linux"}
          <button
            class="btn"
            onclick={() =>
              act("udev", () => api.installUdevRules(), () => toast(t("udevInstalledReplug"), "ok", 5000))}
            >{t("installUdev")}</button
          >
        {/if}
      </div>
    </div>
  {:else if !status.online}
    <div class="banner">
      <Icon name="moon" />
      <div>{t("asleepBanner")}</div>
    </div>
  {/if}

  {#if detail}
    <ProfileBar
      profiles={detail.profiles}
      active={detail.active}
      onactivate={(p) => act(t("profile"), () => api.activateProfile(status.key, p), setDetail)}
      oncreate={(p, copy) => act(t("profile"), () => api.createProfile(status.key, p, copy ? detail!.active : null), setDetail)}
      onrename={(p) => act(t("rename"), () => api.renameProfile(status.key, detail!.active, p), setDetail)}
      ondelete={() => (confirmDelete = true)}
    />

    {#if confirmDelete}
      {@const [before, after] = around("deleteProfileConfirm")}
      <div class="banner">
        <div>{before}<b>{profileName(detail.active)}</b>{after}</div>
        <div class="grow"></div>
        <button
          class="btn danger"
          onclick={() => {
            confirmDelete = false;
            act(t("delete"), () => api.deleteProfile(status.key, detail!.active), setDetail);
          }}>{t("delete")}</button
        >
        <button class="btn ghost" onclick={() => (confirmDelete = false)}>{t("cancel")}</button>
      </div>
    {/if}

    <div class="groups">
      {#each groups as group (group.id)}
        {@const settings = detail.settings.filter((s) => s.group === group.id)}
        {#if settings.length}
          <div class="card">
            <h2>{t(group.title)}</h2>
            {#each settings as setting (setting.name)}
              <SettingRow
                {setting}
                deviceKey={status.key}
                online={status.online}
                keyboardKeys={info.qwerty_keys}
                multimediaKeys={info.multimedia_keys}
                onupdated={updated}
              />
            {/each}
          </div>
        {/if}
      {/each}
    </div>

    <footer>
      {#if confirmReset}
        {@const [before, after] = around("resetProfileConfirm")}
        <span>{before}<b>{profileName(detail.active)}</b>{after}</span>
        <button
          class="btn danger"
          onclick={() => {
            confirmReset = false;
            act(t("reset"), () => api.resetProfile(status.key), setDetail);
          }}>{t("reset")}</button
        >
        <button class="btn ghost" onclick={() => (confirmReset = false)}>{t("cancel")}</button>
      {:else}
        <button class="btn ghost" onclick={() => (confirmReset = true)}><Icon name="reset" size={14} /> {t("resetProfile")}</button>
      {/if}
    </footer>
  {:else if loadError}
    <div class="banner bad"><Icon name="info" /> {loadError}</div>
  {/if}
</section>

<style>
  .panel {
    max-width: 920px;
    margin: 0 auto;
    padding: 24px 28px 40px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }
  .hero {
    display: flex;
    align-items: center;
    gap: 14px;
    min-width: 0;
  }
  .glyph {
    width: 52px;
    height: 52px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 14px;
    background: var(--accent-soft);
    color: var(--accent);
  }
  h1 {
    margin: 0;
    font-size: 22px;
    font-weight: 650;
    letter-spacing: -0.01em;
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    font-size: 12px;
    margin-top: 2px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 8px;
    border-radius: 10px;
    background: var(--panel-2);
    color: var(--text-dim);
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  .banner {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border-radius: var(--radius-sm);
    background: var(--panel-2);
    color: var(--text-dim);
    font-size: 13px;
  }
  .banner > div {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .banner.bad {
    color: var(--bad);
    background: color-mix(in srgb, var(--bad) 10%, transparent);
  }
  .grow {
    flex: 1;
  }
  .groups {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .card {
    background: var(--panel);
    border: 1px solid var(--line-soft);
    border-radius: var(--radius);
    padding: 6px 20px 6px;
    box-shadow: var(--shadow);
  }
  h2 {
    margin: 14px 0 4px;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-faint);
  }
  footer {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--text-dim);
    font-size: 13px;
  }
</style>
