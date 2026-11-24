<script lang="ts">
  import Home from './lib/Home.svelte'
  import Room from './lib/Room.svelte'

  let hash = $state(location.hash)
  $effect(() => {
    const on = () => (hash = location.hash)
    addEventListener('hashchange', on)
    return () => removeEventListener('hashchange', on)
  })
  const go = (p: string) => { location.hash = p; window.scrollTo(0, 0) }
  const roomId = $derived(/^#\/r\/([A-Za-z0-9]{4,16})/.exec(hash)?.[1] ?? null)
</script>

{#if roomId}
  {#key roomId}<Room id={roomId} {go} />{/key}
{:else}
  <Home {go} />
{/if}
