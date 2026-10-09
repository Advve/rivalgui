<script lang="ts">
  import type { Stop } from "../api";
  import { cssGradient } from "../format";
  import Icon from "../Icon.svelte";
  import { t } from "../i18n/index.svelte";
  import Color from "./Color.svelte";

  let {
    isGradient,
    duration,
    stops,
    maxStops,
    onchange,
  }: {
    isGradient: boolean;
    duration: number;
    stops: Stop[];
    maxStops: number;
    onchange: (isGradient: boolean, duration: number, stops: Stop[]) => void;
  } = $props();

  let mode = $state<"static" | "gradient">("static");
  let dur = $state(1000);
  let local = $state<Stop[]>([]);
  $effect(() => {
    mode = isGradient ? "gradient" : "static";
    dur = duration;
    local = stops.map((s) => ({ ...s }));
  });

  const preview = $derived(mode === "static" ? (local[0]?.color ?? "#000") : cssGradient(local));

  function emit() {
    onchange(mode === "gradient", dur, local.map((s) => ({ ...s })));
  }

  function setMode(m: "static" | "gradient") {
    mode = m;
    if (m === "gradient" && local.length < 2) {
      const first = local[0] ?? { pos: 0, color: "#ff0000" };
      local = [{ ...first, pos: 0 }, { pos: 33, color: "#00ff00" }, { pos: 66, color: "#0000ff" }];
    }
    emit();
  }

  function addStop() {
    const sorted = [...local].sort((a, b) => a.pos - b.pos);
    // Insert in the largest gap
    let best = { at: 50, size: 0 };
    for (let i = 0; i < sorted.length; i++) {
      const a = sorted[i].pos;
      const b = i + 1 < sorted.length ? sorted[i + 1].pos : 100;
      if (b - a > best.size) best = { at: Math.round((a + b) / 2), size: b - a };
    }
    local.push({ pos: best.at, color: "#ffffff" });
    local.sort((a, b) => a.pos - b.pos);
    emit();
  }
</script>

<div class="gradient">
  <div class="top">
    <div class="seg">
      <button class:active={mode === "static"} onclick={() => setMode("static")}>{t("static")}</button>
      <button class:active={mode === "gradient"} onclick={() => setMode("gradient")}>{t("gradient")}</button>
    </div>
    {#if mode === "gradient"}
      <label class="dur">
        <span class="faint">{t("cycle")}</span>
        <input
          class="input"
          type="number"
          min="100"
          max="30000"
          step="100"
          value={dur}
          onchange={(e) => {
            dur = Math.max(100, Math.min(30000, e.currentTarget.valueAsNumber || 1000));
            e.currentTarget.value = String(dur);
            emit();
          }}
        />
        <span class="faint">ms</span>
      </label>
    {/if}
  </div>

  <div class="preview" style:background={preview}></div>

  {#if mode === "static"}
    <Color
      color={local[0]?.color ?? "#ff0000"}
      onchange={(hex) => {
        local = [{ pos: 0, color: hex }];
        emit();
      }}
    />
  {:else}
    <div class="stops">
      {#each local as stop, i (i)}
        <div class="stop">
          <input
            type="color"
            value={stop.color}
            oninput={(e) => {
              local[i].color = e.currentTarget.value;
              emit();
            }}
          />
          <input
            type="range"
            min="0"
            max="100"
            value={stop.pos}
            style:--fill="{stop.pos}%"
            oninput={(e) => {
              local[i].pos = e.currentTarget.valueAsNumber;
              emit();
            }}
          />
          <span class="pos">{stop.pos}%</span>
          <button
            class="btn ghost icon"
            title={t("removeColor")}
            disabled={local.length <= 2}
            onclick={() => {
              local.splice(i, 1);
              emit();
            }}><Icon name="x" size={14} /></button
          >
        </div>
      {/each}
    </div>
    {#if local.length < maxStops}
      <button class="btn ghost add" onclick={addStop}><Icon name="plus" size={14} /> {t("addColor")}</button>
    {/if}
  {/if}
</div>

<style>
  .gradient {
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: 100%;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .seg {
    display: inline-flex;
    padding: 3px;
    gap: 2px;
    border-radius: var(--radius-sm);
    background: var(--bg-elev);
    border: 1px solid var(--line);
  }
  .seg button {
    border: none;
    background: transparent;
    color: var(--text-dim);
    padding: 4px 12px;
    border-radius: 6px;
    font-size: 13px;
  }
  .seg button.active {
    background: var(--accent-grad);
    color: #fff;
  }
  .dur {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
  }
  .preview {
    height: 14px;
    border-radius: 7px;
    border: 1px solid var(--line);
  }
  .stops {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .stop {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .pos {
    width: 40px;
    text-align: right;
    font-size: 12px;
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
  }
  .add {
    align-self: flex-start;
  }
</style>
