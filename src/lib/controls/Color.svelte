<script lang="ts">
  import { t } from "../i18n/index.svelte";

  let { color, onchange }: { color: string; onchange: (hex: string) => void } = $props();

  const swatches = ["#ff0000", "#ff6a00", "#ffd000", "#00ff40", "#00e5ff", "#0040ff", "#a000ff", "#ff00a0", "#ffffff"];
  let local = $state("#000000");
  $effect(() => {
    local = color;
  });
  function pick(hex: string) {
    local = hex;
    onchange(hex);
  }
</script>

<div class="color">
  <input type="color" bind:value={local} oninput={() => onchange(local)} title={t("pickColor")} />
  <input
    class="input hex"
    value={local}
    maxlength="7"
    spellcheck="false"
    onchange={(e) => {
      const v = e.currentTarget.value.trim();
      const hex = v.startsWith("#") ? v : `#${v}`;
      if (/^#[0-9a-f]{6}$/i.test(hex)) pick(hex.toLowerCase());
      else e.currentTarget.value = local;
    }}
  />
  <div class="swatches">
    {#each swatches as s (s)}
      <button
        class="swatch"
        class:active={s === local}
        style:background={s}
        title={s}
        aria-label={t("useColor", { color: s })}
        onclick={() => pick(s)}
      ></button>
    {/each}
  </div>
</div>

<style>
  .color {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .hex {
    width: 86px;
    font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
    font-size: 13px;
  }
  .swatches {
    display: flex;
    gap: 4px;
  }
  .swatch {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 2px solid var(--panel);
    box-shadow: 0 0 0 1px var(--line);
    padding: 0;
  }
  .swatch.active {
    box-shadow: 0 0 0 2px var(--accent);
  }
</style>
