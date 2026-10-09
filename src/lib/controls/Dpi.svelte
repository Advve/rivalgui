<script lang="ts">
  import Icon from "../Icon.svelte";
  import { t } from "../i18n/index.svelte";

  type Pair = [number, number];

  let {
    range,
    values,
    max,
    xy = false,
    onchange,
  }: {
    range: [number, number, number];
    /** One [x, y] pair per preset (x === y when the device has a single axis) */
    values: Pair[];
    max: number;
    /** The device accepts different X and Y DPI */
    xy?: boolean;
    onchange: (v: Pair[]) => void;
  } = $props();

  let local = $state<Pair[]>([]);
  let separate = $state(false);
  $effect(() => {
    local = values.map(([x, y]) => [x, y] as Pair);
    separate = xy && values.some(([x, y]) => x !== y);
  });
  const min = $derived(range[0]);
  const top = $derived(range[1]);
  const step = $derived(range[2]);
  const fill = (v: number) => `${((v - min) / (top - min)) * 100}%`;

  function snap(v: number) {
    if (!Number.isFinite(v)) return min;
    return min + Math.round((Math.min(top, Math.max(min, v)) - min) / step) * step;
  }
  function emit() {
    onchange(local.map(([x, y]) => [x, y] as Pair));
  }
  function update(i: number, axis: 0 | 1, v: number) {
    const s = snap(v);
    if (axis === 0 && !separate) local[i] = [s, s];
    else local[i][axis] = s;
    emit();
  }
  function add() {
    const last = local[local.length - 1]?.[0] ?? 800;
    const v = snap(Math.min(top, last * 2));
    local.push([v, v]);
    emit();
  }
  function remove(i: number) {
    local.splice(i, 1);
    emit();
  }
  function toggleSeparate() {
    separate = !separate;
    if (!separate) {
      local = local.map(([x]) => [x, x] as Pair);
      emit();
    }
  }
</script>

{#snippet axisInput(i: number, axis: 0 | 1, label: string)}
  <input
    type="range"
    {min}
    max={top}
    {step}
    value={local[i][axis]}
    style:--fill={fill(local[i][axis])}
    aria-label="{t('presetN', { n: i + 1 })} {label}"
    oninput={(e) => update(i, axis, e.currentTarget.valueAsNumber)}
  />
  <input
    class="input"
    type="number"
    {min}
    max={top}
    {step}
    value={local[i][axis]}
    onchange={(e) => {
      update(i, axis, e.currentTarget.valueAsNumber);
      e.currentTarget.value = String(local[i][axis]);
    }}
  />
{/snippet}

<div class="dpi">
  {#each local as _, i (i)}
    <div class="preset">
      <span class="idx">{i + 1}</span>
      <div class="axes">
        <div class="axis">
          {#if separate}<span class="tag">X</span>{/if}
          {@render axisInput(i, 0, separate ? "X" : "DPI")}
          <span class="faint unit">DPI</span>
        </div>
        {#if separate}
          <div class="axis">
            <span class="tag">Y</span>
            {@render axisInput(i, 1, "Y")}
            <span class="faint unit">DPI</span>
          </div>
        {/if}
      </div>
      <button class="btn ghost icon" title={t("removePreset")} disabled={local.length <= 1} onclick={() => remove(i)}
        ><Icon name="x" size={14} /></button
      >
    </div>
  {/each}
  <div class="footer">
    {#if local.length < max}
      <button class="btn ghost" onclick={add}><Icon name="plus" size={14} /> {t("addPreset")}</button>
    {/if}
    {#if xy}
      <label class="sep">
        <button class="switch" class:on={separate} role="switch" aria-checked={separate} aria-label={t("separateXY")} onclick={toggleSeparate}></button>
        {t("separateXY")}
      </label>
    {/if}
  </div>
</div>

<style>
  .dpi {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
  }
  .preset {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .axes {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .axis {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .idx {
    width: 22px;
    height: 22px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 6px;
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 12px;
    font-weight: 600;
  }
  .tag {
    width: 12px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-faint);
  }
  .unit {
    font-size: 12px;
  }
  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .sep {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text-dim);
    margin-left: auto;
  }
</style>
