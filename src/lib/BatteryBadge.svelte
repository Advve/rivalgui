<script lang="ts">
  import type { DeviceStatus } from "./api";
  import Icon from "./Icon.svelte";
  import { t } from "./i18n/index.svelte";

  let { device }: { device: DeviceStatus } = $props();
  const level = $derived(device.battery?.level ?? null);
  const charging = $derived(device.battery?.charging === true);
  const color = $derived(
    charging ? "var(--info)" : level === null ? "var(--text-faint)" : level > 50 ? "var(--ok)" : level > 20 ? "var(--warn)" : "var(--bad)",
  );
</script>

{#if device.wireless}
  {#if level !== null}
    <span class="battery" style:--c={color} title={charging ? t("charging") : t("onBattery")}>
      <span class="cell"><span class="fill" style:width="{level}%"></span></span>
      {level}%
      {#if charging}<Icon name="zap" size={12} />{/if}
    </span>
  {:else}
    <span class="battery asleep" title={t("asleepTitle")}>
      <Icon name="moon" size={12} /> {t("asleep")}
    </span>
  {/if}
{/if}

<style>
  .battery {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    color: var(--c);
  }
  .asleep {
    color: var(--text-faint);
  }
  .cell {
    position: relative;
    width: 20px;
    height: 10px;
    border: 1.5px solid currentColor;
    border-radius: 3px;
    padding: 1px;
  }
  .cell::after {
    content: "";
    position: absolute;
    right: -4px;
    top: 2px;
    width: 2px;
    height: 3px;
    border-radius: 0 1px 1px 0;
    background: currentColor;
  }
  .fill {
    display: block;
    height: 100%;
    border-radius: 1px;
    background: currentColor;
  }
</style>
