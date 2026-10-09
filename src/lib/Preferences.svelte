<script lang="ts">
  import { api, errorText, type AppInfo, type Prefs } from "./api";
  import { toast } from "./toast.svelte";
  import { LANGUAGES, t } from "./i18n/index.svelte";

  let { info, onchange }: { info: AppInfo; onchange: (info: AppInfo) => void } = $props();

  async function setPrefs(patch: Partial<Prefs>) {
    try {
      const prefs = await api.setPrefs({ ...info.prefs, ...patch });
      onchange({ ...info, prefs });
    } catch (e) {
      toast(errorText(e), "error");
    }
  }

  async function setAutostart(enabled: boolean) {
    try {
      const autostart = await api.setAutostart(enabled);
      onchange({ ...info, autostart });
    } catch (e) {
      toast(errorText(e), "error");
    }
  }
</script>

<section class="panel">
  <h1>{t("preferences")}</h1>

  <div class="card">
    <div class="row">
      <div>
        <div class="title">{t("language")}</div>
        <div class="faint">{t("languageHint")}</div>
      </div>
      <select
        class="input"
        value={info.prefs.language || "auto"}
        aria-label={t("language")}
        onchange={(e) => setPrefs({ language: e.currentTarget.value })}
      >
        <option value="auto">{t("languageAuto")}</option>
        {#each LANGUAGES as lang (lang.code)}
          <option value={lang.code}>{lang.name}</option>
        {/each}
      </select>
    </div>
    <div class="row">
      <div>
        <div class="title">{t("startWithSystem")}</div>
        <div class="faint">{t("startWithSystemHint")}</div>
      </div>
      <button
        class="switch"
        class:on={info.autostart}
        role="switch"
        aria-checked={info.autostart}
        aria-label={t("startWithSystem")}
        onclick={() => setAutostart(!info.autostart)}
      ></button>
    </div>
    <div class="row">
      <div>
        <div class="title">{t("autoApply")}</div>
        <div class="faint">{t("autoApplyHint")}</div>
      </div>
      <button
        class="switch"
        class:on={info.prefs.auto_apply}
        role="switch"
        aria-checked={info.prefs.auto_apply}
        aria-label={t("autoApply")}
        onclick={() => setPrefs({ auto_apply: !info.prefs.auto_apply })}
      ></button>
    </div>
    <div class="row">
      <div>
        <div class="title">{t("lowBattery")}</div>
        <div class="faint">{t("lowBatteryHint")}</div>
      </div>
      <div class="inline">
        <select
          class="input"
          value={String(info.prefs.low_battery_threshold)}
          disabled={!info.prefs.notify_low_battery}
          onchange={(e) => setPrefs({ low_battery_threshold: Number(e.currentTarget.value) })}
        >
          {#each [5, 10, 15, 20, 25, 30] as level (level)}
            <option value={String(level)}>{level}%</option>
          {/each}
        </select>
        <button
          class="switch"
          class:on={info.prefs.notify_low_battery}
          role="switch"
          aria-checked={info.prefs.notify_low_battery}
          aria-label={t("lowBattery")}
          onclick={() => setPrefs({ notify_low_battery: !info.prefs.notify_low_battery })}
        ></button>
      </div>
    </div>
    {#if info.platform === "linux"}
      <div class="row">
        <div>
          <div class="title">{t("udevTitle")}</div>
          <div class="faint">{t("udevHint")}</div>
        </div>
        <button
          class="btn"
          onclick={async () => {
            try {
              await api.installUdevRules();
              toast(t("udevInstalled"), "ok", 5000);
            } catch (e) {
              toast(errorText(e), "error");
            }
          }}>{t("installRules")}</button
        >
      </div>
    {/if}
  </div>

  <div class="card about">
    <div class="title">{t("about")}</div>
    <p class="faint">{t("aboutBody", { version: info.version })}</p>
    {#if info.config_path}<p class="faint path">{t("settingsFile", { path: info.config_path })}</p>{/if}
    {#if info.dry_run}<p class="warn">{t("dryRun")}</p>{/if}
    {#if info.hid_error}<p class="warn">{t("hidError", { error: info.hid_error })}</p>{/if}
  </div>
</section>

<style>
  .panel {
    max-width: 760px;
    margin: 0 auto;
    padding: 24px 28px 40px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  h1 {
    margin: 0 0 4px;
    font-size: 22px;
    font-weight: 650;
  }
  .card {
    background: var(--panel);
    border: 1px solid var(--line-soft);
    border-radius: var(--radius);
    padding: 4px 20px;
    box-shadow: var(--shadow);
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 14px 0;
    border-top: 1px solid var(--line-soft);
    font-size: 13px;
  }
  .row:first-child {
    border-top: none;
  }
  .title {
    font-weight: 600;
    font-size: 14px;
  }
  .inline {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .about {
    padding: 16px 20px;
  }
  .about p {
    margin: 6px 0 0;
    font-size: 13px;
  }
  .path {
    font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
    font-size: 12px !important;
    user-select: text;
  }
  .warn {
    color: var(--warn);
  }
</style>
