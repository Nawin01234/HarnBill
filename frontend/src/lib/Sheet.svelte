<script lang="ts">
  import type { Snippet } from 'svelte'
  import Icon from './Icon.svelte'
  let { open = $bindable(), title, children }: { open: boolean; title: string; children: Snippet } = $props()
  let el: HTMLDivElement | undefined = $state()
  $effect(() => { if (open) queueMicrotask(() => el?.focus()) })
</script>
{#if open}
  <div class="scrim" role="presentation" onclick={() => (open = false)}></div>
  <div class="sheet" role="dialog" aria-modal="true" aria-label={title} tabindex="-1" bind:this={el}
    onkeydown={(e) => e.key === 'Escape' && (open = false)}>
    <div class="grab" aria-hidden="true"></div>
    <header><h2>{title}</h2><button class="x" onclick={() => (open = false)} aria-label="ปิด"><Icon name="x" /></button></header>
    <div class="body">{@render children()}</div>
  </div>
{/if}
<style>
  .scrim { position: fixed; inset: 0; background: rgba(15,23,42,.45); z-index: 40; animation: f 180ms; }
  .sheet { position: fixed; z-index: 41; left: 50%; bottom: 0; transform: translateX(-50%); width: min(560px, 100vw);
    max-height: 92dvh; display: flex; flex-direction: column; background: var(--card); border-radius: 22px 22px 0 0;
    padding-bottom: var(--safe-b); animation: up 260ms var(--ease); outline: none; }
  @media (min-width: 720px) { .sheet { bottom: auto; top: 50%; transform: translate(-50%, -50%); border-radius: 22px; animation: pop 220ms var(--ease); } .grab { display: none; } }
  .grab { width: 40px; height: 5px; border-radius: 9px; background: var(--line); margin: 8px auto 0; }
  header { display: flex; align-items: center; justify-content: space-between; padding: 10px 12px 4px 20px; }
  h2 { margin: 0; font-size: 18px; }
  .x { width: 44px; height: 44px; border: 0; background: none; border-radius: 50%; display: grid; place-items: center; color: var(--muted); }
  .x:hover { background: var(--slate-100); }
  .body { overflow: auto; padding: 8px 20px 20px; }
  @keyframes f { from { opacity: 0 } }
  @keyframes up { from { transform: translate(-50%, 100%) } }
  @keyframes pop { from { opacity: 0; transform: translate(-50%, -46%) } }
</style>
