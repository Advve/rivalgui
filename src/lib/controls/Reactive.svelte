<script lang="ts">
  import Color from "./Color.svelte";
  import { t } from "../i18n/index.svelte";

  let { color, onchange }: { color: string | null; onchange: (v: string) => void } = $props();
  let lastColor = $state("#ffffff");
  $effect(() => {
    if (color) lastColor = color;
  });
</script>

<div class="reactive">
  <button
    class="switch"
    class:on={color !== null}
    role="switch"
    aria-checked={color !== null}
    aria-label={t("enableReactive")}
    onclick={() => onchange(color === null ? lastColor : "off")}
  ></button>
  {#if color !== null}
    <Color {color} onchange={(hex) => onchange(hex)} />
  {:else}
    <span class="faint">{t("off")}</span>
  {/if}
</div>

<style>
  .reactive {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
</style>
