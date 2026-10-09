<script lang="ts">
  import { api, errorText, type Json, type SettingView } from "./api";
  import { buttonsValue, dpiValue, dpiXyValue, gradientValue } from "./format";
  import { toast } from "./toast.svelte";
  import Icon from "./Icon.svelte";
  import { t, td } from "./i18n/index.svelte";
  import Choice from "./controls/Choice.svelte";
  import Range from "./controls/Range.svelte";
  import Dpi from "./controls/Dpi.svelte";
  import Color from "./controls/Color.svelte";
  import Reactive from "./controls/Reactive.svelte";
  import Gradient from "./controls/Gradient.svelte";
  import Buttons from "./controls/Buttons.svelte";

  let {
    setting,
    deviceKey,
    online,
    keyboardKeys,
    multimediaKeys,
    onupdated,
  }: {
    setting: SettingView;
    deviceKey: string;
    online: boolean;
    keyboardKeys: string[];
    multimediaKeys: string[];
    onupdated: (view: SettingView) => void;
  } = $props();

  // Remounts the control to drop local edits (after an error)
  let version = $state(0);
  let busy = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let pending: Json | undefined;

  const unit = $derived.by(() => {
    const m = setting.description.match(/\((minutes|seconds|ms|dpi|hz|%)/i);
    if (!m) return "";
    return { minutes: "min", seconds: "s" }[m[1].toLowerCase()] ?? m[1];
  });
  const wide = $derived(
    ["multidpi_range", "multidpi_range_choice", "multidpi_range_choice_xy", "rgbgradient", "rgbgradientv2", "buttons"].includes(
      setting.value_type,
    ),
  );
  const description = $derived(td(setting.description.replace(/\s*\([^)]*\)\s*$/, "")));
  const label = $derived(td(setting.label));

  async function flush() {
    if (pending === undefined) return;
    const value = pending;
    pending = undefined;
    busy = true;
    try {
      const view = await api.setSetting(deviceKey, setting.name, value);
      // Skip stale echoes while the user is still dragging
      if (pending === undefined) onupdated(view);
    } catch (e) {
      toast(`${label}: ${errorText(e)}`, "error", 5000);
      version++;
    } finally {
      busy = false;
    }
  }

  /** Sends a value; rapid changes (sliders, color pickers) are coalesced. */
  function send(value: Json, delay = 180) {
    pending = value;
    clearTimeout(timer);
    timer = setTimeout(flush, delay);
  }

  async function reset() {
    try {
      onupdated(await api.resetSetting(deviceKey, setting.name));
      version++;
    } catch (e) {
      toast(errorText(e), "error");
    }
  }

  async function run() {
    busy = true;
    try {
      await api.runAction(deviceKey, setting.name);
      toast(t("done", { label }), "ok", 2000);
    } catch (e) {
      toast(errorText(e), "error");
    } finally {
      busy = false;
    }
  }
</script>

<div class="row" class:wide>
  <div class="head">
    <div class="title">
      {label}
      {#if busy}<span class="dot" title={t("sending")}></span>{/if}
    </div>
    <div class="desc faint">{description}</div>
  </div>
  <div class="control">
    {#key version}
      {#if setting.ui.kind === "choice"}
        <Choice choices={setting.choices ?? []} value={setting.ui.value} {unit} onchange={(v) => send(v, 0)} />
      {:else if setting.ui.kind === "range" && setting.range}
        <Range range={setting.range} value={setting.ui.value} {unit} onchange={(v) => send(v)} />
      {:else if setting.ui.kind === "dpi" && setting.range}
        <Dpi
          range={setting.range}
          values={setting.ui.values.map((v) => [v, v] as [number, number])}
          max={setting.max_presets ?? 1}
          onchange={(v) => send(dpiValue(v.map(([x]) => x)))}
        />
      {:else if setting.ui.kind === "dpi_xy" && setting.range}
        <Dpi
          range={setting.range}
          values={setting.ui.values}
          max={setting.max_presets ?? 1}
          xy
          onchange={(v) => send(dpiXyValue(v))}
        />
      {:else if setting.ui.kind === "color"}
        <Color color={setting.ui.color} onchange={(hex) => send(hex)} />
      {:else if setting.ui.kind === "reactive"}
        <Reactive color={setting.ui.color} onchange={(v) => send(v)} />
      {:else if setting.ui.kind === "gradient"}
        <Gradient
          isGradient={setting.ui.is_gradient}
          duration={setting.ui.duration}
          stops={setting.ui.stops}
          maxStops={setting.max_stops ?? 14}
          onchange={(g, d, s) => send(gradientValue(g, d, s), 250)}
        />
      {:else if setting.ui.kind === "buttons" && setting.button_targets}
        <Buttons
          mapping={setting.ui.mapping}
          targets={setting.button_targets}
          {keyboardKeys}
          {multimediaKeys}
          onchange={(m) => send(buttonsValue(m), 0)}
        />
      {:else if setting.ui.kind === "action"}
        <button class="btn" disabled={!online || busy} onclick={run}><Icon name="play" size={14} /> {t("apply")}</button>
      {:else if setting.ui.kind === "invalid"}
        <span class="error">{t("invalidStored", { error: setting.ui.error })}</span>
      {/if}
    {/key}
  </div>
  <div class="reset">
    {#if setting.is_set && setting.ui.kind !== "action"}
      <button class="btn ghost icon" title={t("resetToDefault")} onclick={reset}><Icon name="reset" size={14} /></button>
    {/if}
  </div>
</div>

<style>
  /* Label left, control right; the control wraps below when space runs out */
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px 16px;
    padding: 14px 0;
    border-top: 1px solid var(--line-soft);
  }
  .row:first-child {
    border-top: none;
  }
  .head {
    flex: 1 1 200px;
    min-width: 0;
  }
  .control {
    flex: 0 1 auto;
    max-width: 100%;
    justify-content: flex-end;
  }
  .reset {
    width: 32px;
    flex: none;
  }
  .row.wide .head {
    flex-basis: calc(100% - 48px);
  }
  .row.wide .control {
    flex: 1 1 calc(100% - 48px);
    justify-content: flex-start;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
  }
  .desc {
    font-size: 12px;
    margin-top: 1px;
  }
  .control {
    display: flex;
    align-items: center;
    min-width: 0;
  }
  .control :global(.seg) {
    max-width: 100%;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse 0.8s infinite alternate;
  }
  @keyframes pulse {
    to {
      opacity: 0.25;
    }
  }
  .error {
    color: var(--bad);
    font-size: 12px;
  }
</style>
