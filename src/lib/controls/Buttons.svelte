<script lang="ts">
  import type { ButtonTargets } from "../api";
  import { prettyKey } from "../format";
  import { t, td } from "../i18n/index.svelte";
  import type { Key } from "../i18n/en";

  let {
    mapping,
    targets,
    keyboardKeys,
    multimediaKeys,
    onchange,
  }: {
    mapping: { button: string; value: string }[];
    targets: ButtonTargets;
    keyboardKeys: string[];
    multimediaKeys: string[];
    onchange: (mapping: { button: string; value: string }[]) => void;
  } = $props();

  const specialLabels: Record<string, Key> = {
    disabled: "disabled",
    dpi: "dpiSwitch",
    scrollup: "scrollUp",
    scrolldown: "scrollDown",
  };
  const special = (s: string) => (specialLabels[s] ? t(specialLabels[s]) : s);
  const buttonNames = $derived(targets.buttons.map((b) => b.toLowerCase()));
  // Button names win over special actions with the same name (like in rivalcfg)
  const specials = $derived(targets.specials.filter((s) => !buttonNames.includes(s)));

  function label(button: string) {
    const m = button.match(/^button(\d+)$/i);
    if (m) {
      const n = Number(m[1]);
      const named: Key[] = ["leftClick", "rightClick", "middleClick", "back", "forward"];
      return named[n - 1] ? t(named[n - 1]) : t("buttonN", { n });
    }
    const lower = button.toLowerCase();
    if (specialLabels[lower]) return special(lower);
    return prettyKey(button.replace(/([a-z])([A-Z])/g, "$1 $2"));
  }

  function known(value: string) {
    const v = value.toLowerCase();
    return (
      buttonNames.includes(v) ||
      specials.includes(v) ||
      (targets.multimedia && multimediaKeys.some((k) => k.toLowerCase() === v)) ||
      (targets.keyboard && keyboardKeys.some((k) => k.toLowerCase() === v))
    );
  }

  function set(i: number, value: string) {
    const next = mapping.map((m) => ({ ...m }));
    next[i].value = value;
    onchange(next);
  }
</script>

<div class="buttons">
  {#each mapping as m, i (m.button)}
    <div class="row" class:changed={targets.defaults[m.button] !== undefined && targets.defaults[m.button] !== m.value}>
      <span class="name">{label(m.button)}<span class="faint id">{m.button}</span></span>
      <select class="input" value={m.value} onchange={(e) => set(i, e.currentTarget.value)}>
        {#if !known(m.value)}
          <option value={m.value}>{m.value}</option>
        {/if}
        <optgroup label={t("optMouse")}>
          {#each targets.buttons as b (b)}
            <option value={b.toLowerCase()}>{label(b)}</option>
          {/each}
        </optgroup>
        {#if specials.length}
          <optgroup label={t("optActions")}>
            {#each specials as s (s)}
              <option value={s}>{special(s)}</option>
            {/each}
          </optgroup>
        {/if}
        {#if targets.multimedia}
          <optgroup label={t("optMedia")}>
            {#each multimediaKeys as k (k)}
              <option value={k.toLowerCase()}>{td(k)}</option>
            {/each}
          </optgroup>
        {/if}
        {#if targets.keyboard}
          <optgroup label={t("optKeyboard")}>
            {#each keyboardKeys as k (k)}
              <option value={k.toLowerCase()}>{k}</option>
            {/each}
          </optgroup>
        {/if}
      </select>
    </div>
  {/each}
</div>

<style>
  .buttons {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 8px 16px;
    width: 100%;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 6px 8px 6px 12px;
    border-radius: var(--radius-sm);
    background: var(--bg-elev);
    border: 1px solid var(--line-soft);
  }
  .row.changed {
    border-color: var(--accent);
  }
  .name {
    display: flex;
    flex-direction: column;
    font-size: 13px;
    line-height: 1.2;
  }
  .id {
    font-size: 11px;
  }
  select {
    width: 180px;
  }
</style>
