<script lang="ts">
  let {
    range,
    value,
    unit = "",
    onchange,
  }: { range: [number, number, number]; value: number; unit?: string; onchange: (v: number) => void } = $props();

  const min = $derived(range[0]);
  const max = $derived(range[1]);
  const step = $derived(range[2]);
  let local = $state(0);
  $effect(() => {
    local = value;
  });
  const fill = $derived(max > min ? ((local - min) / (max - min)) * 100 : 0);

  function snap(v: number) {
    if (!Number.isFinite(v)) return min;
    const clamped = Math.min(max, Math.max(min, v));
    return min + Math.round((clamped - min) / step) * step;
  }
</script>

<div class="range">
  <input type="range" {min} {max} {step} bind:value={local} style:--fill="{fill}%" oninput={() => onchange(local)} />
  <label class="num">
    <input
      class="input"
      type="number"
      {min}
      {max}
      {step}
      value={local}
      onchange={(e) => {
        local = snap(e.currentTarget.valueAsNumber);
        e.currentTarget.value = String(local);
        onchange(local);
      }}
    />
    {#if unit}<span class="faint">{unit}</span>{/if}
  </label>
</div>

<style>
  .range {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
  }
  .num {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
  }
</style>
