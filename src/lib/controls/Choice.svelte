<script lang="ts">
  import type { Json } from "../api";
  import { choiceValue, prettyKey } from "../format";
  import { i18n, td } from "../i18n/index.svelte";

  let {
    choices,
    value,
    unit = "",
    onchange,
  }: { choices: string[]; value: string; unit?: string; onchange: (v: Json) => void } = $props();
  const segmented = $derived(choices.length <= 6 && choices.join("").length <= 42);
  const text = (c: string) =>
    unit && /^\d+$/.test(c) ? `${c} ${unit}` : i18n.lang === "en" ? prettyKey(c) : prettyKey(td(c));
</script>

{#if segmented}
  <div class="seg" role="radiogroup">
    {#each choices as choice (choice)}
      <button
        role="radio"
        aria-checked={choice === value}
        class:active={choice === value}
        onclick={() => onchange(choiceValue(choice))}>{text(choice)}</button
      >
    {/each}
  </div>
{:else}
  <select class="input" {value} onchange={(e) => onchange(choiceValue(e.currentTarget.value))}>
    {#each choices as choice (choice)}
      <option value={choice}>{text(choice)}</option>
    {/each}
  </select>
{/if}

<style>
  .seg {
    display: inline-flex;
    flex-wrap: wrap;
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
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }
  .seg button:hover {
    color: var(--text);
  }
  .seg button.active {
    background: var(--accent-grad);
    color: #fff;
  }
</style>
