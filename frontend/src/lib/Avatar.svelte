<script lang="ts">
  import { colorOf } from './api'
  let { id, name, size = 32, ring = false }: { id: number; name: string; size?: number; ring?: boolean } = $props()
  // skip Thai leading vowels so "เบียร์" → "บ"
  const ch = $derived([...name.trim()].find((c) => /[\p{L}\p{N}]/u.test(c) && !/[เแโใไ]/.test(c))?.toUpperCase() ?? '?')
</script>
<span class="av" class:ring style:--c={colorOf(id)} style:width="{size}px" style:height="{size}px" style:font-size="{size * 0.45}px" title={name} aria-hidden="true">{ch}</span>
<style>
  .av { display: inline-grid; place-items: center; flex: none; border-radius: 50%; background: var(--c); color: #fff; font-weight: 700; line-height: 1; }
  .ring { box-shadow: 0 0 0 2px var(--card); }
</style>
