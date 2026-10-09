<script lang="ts">
  import Icon from "./Icon.svelte";
  import { profileName, t } from "./i18n/index.svelte";

  let {
    profiles,
    active,
    onactivate,
    oncreate,
    onrename,
    ondelete,
  }: {
    profiles: string[];
    active: string;
    onactivate: (name: string) => void;
    oncreate: (name: string, copy: boolean) => void;
    onrename: (name: string) => void;
    ondelete: () => void;
  } = $props();

  let editing = $state<null | "new" | "copy" | "rename">(null);
  let draft = $state("");
  let input = $state<HTMLInputElement>();

  function start(mode: "new" | "copy" | "rename") {
    editing = mode;
    draft = mode === "rename" ? active : mode === "copy" ? t("copySuffix", { name: active }) : "";
    queueMicrotask(() => input?.select());
  }

  function commit() {
    const name = draft.trim();
    if (!name) return;
    if (editing === "rename") onrename(name);
    else oncreate(name, editing === "copy");
    editing = null;
  }
</script>

<div class="bar">
  <span class="label faint">{t("profile")}</span>
  {#if editing}
    <form
      class="edit"
      onsubmit={(e) => {
        e.preventDefault();
        commit();
      }}
    >
      <input
        bind:this={input}
        class="input"
        bind:value={draft}
        maxlength="40"
        placeholder={t("profileName")}
        onkeydown={(e) => e.key === "Escape" && (editing = null)}
      />
      <button class="btn primary" type="submit" disabled={!draft.trim()}>
        {editing === "rename" ? t("rename") : t("create")}
      </button>
      <button class="btn ghost" type="button" onclick={() => (editing = null)}>{t("cancel")}</button>
    </form>
  {:else}
    <div class="tabs" role="tablist">
      {#each profiles as name (name)}
        <button role="tab" aria-selected={name === active} class:active={name === active} onclick={() => onactivate(name)}>
          {profileName(name)}
        </button>
      {/each}
    </div>
    <div class="actions">
      <button class="btn ghost icon" title={t("newProfile")} onclick={() => start("new")}><Icon name="plus" /></button>
      <button class="btn ghost icon" title={t("duplicateProfile")} onclick={() => start("copy")}><Icon name="copy" /></button>
      <button class="btn ghost icon" title={t("renameProfile")} onclick={() => start("rename")}><Icon name="edit" /></button>
      <button class="btn ghost icon danger" title={t("deleteProfile")} disabled={profiles.length <= 1} onclick={ondelete}
        ><Icon name="trash" /></button
      >
    </div>
  {/if}
</div>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 40px;
  }
  .label {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .tabs {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
    flex: 1;
    min-width: 0;
  }
  .tabs button {
    height: 30px;
    padding: 0 14px;
    border-radius: 15px;
    border: 1px solid var(--line);
    background: transparent;
    color: var(--text-dim);
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tabs button:hover {
    color: var(--text);
  }
  .tabs button.active {
    border-color: transparent;
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }
  .actions {
    display: flex;
    gap: 2px;
  }
  .edit {
    display: flex;
    gap: 6px;
    flex: 1;
  }
  .edit .input {
    width: 220px;
  }
</style>
