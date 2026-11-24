<script lang="ts">
  import { bahtShort, toSatang, type Item, type Member } from './api'
  import Avatar from './Avatar.svelte'
  import Icon from './Icon.svelte'

  let {
    item, members, me, admin, unclaimed,
    onclaim, onedit, ondelete,
  }: {
    item: Item; members: Member[]; me: number | null; admin: boolean; unclaimed: boolean
    onclaim: (member: number, on: boolean) => void
    onedit: (patch: Partial<{ name: string; price: number; qty: number; shared: boolean }>) => void
    ondelete: () => void
  } = $props()

  let open = $state(false)
  let name = $state('')
  let price = $state('')
  let qty = $state(1)

  const mine = $derived(me != null && (item.shared || item.claims.includes(me)))
  const who = $derived(item.shared ? members : members.filter((m) => item.claims.includes(m.id)))
  const each = $derived(who.length ? Math.ceil((item.price * item.qty) / who.length) : 0)

  function startEdit() {
    name = item.name; price = String(item.price / 100); qty = item.qty; open = !open
  }
  function save() {
    const p = toSatang(price)
    if (!name.trim() || Number.isNaN(p)) return
    onedit({ name, price: p, qty })
    open = false
  }
</script>

<li class="item" class:mine class:warn={unclaimed}>
  <div class="main">
    <button class="info" onclick={admin ? startEdit : undefined} disabled={!admin} aria-expanded={admin ? open : undefined}
      aria-label={admin ? `แก้ไข ${item.name}` : undefined}>
      <span class="name">{item.name}</span>
      <span class="meta num">
        {bahtShort(item.price)}{#if item.qty > 1}&nbsp;× {item.qty} = {bahtShort(item.price * item.qty)}{/if}
        {#if who.length > 1}<span class="dot">·</span>คนละ ~{bahtShort(each)}{/if}
      </span>
      <span class="who">
        {#if item.shared}
          <span class="tag">ทุกคนหาร</span>
        {:else if who.length === 0}
          <span class="tag warn">ยังไม่มีใครติ๊ก</span>
        {:else}
          <span class="stack">{#each who.slice(0, 6) as m (m.id)}<Avatar id={m.id} name={m.name} size={24} ring />{/each}</span>
          {#if who.length > 6}<span class="more">+{who.length - 6}</span>{/if}
        {/if}
      </span>
    </button>
    {#if me != null && !item.shared}
      <button class="tick" aria-pressed={mine} onclick={() => onclaim(me, !mine)} aria-label="{mine ? 'ยกเลิก' : 'ฉันกิน'} {item.name}">
        <span class="box">{#if mine}<Icon name="check" size={18} />{/if}</span>
        <span class="lbl">{mine ? 'กินแล้ว' : 'ฉันกิน'}</span>
      </button>
    {/if}
  </div>

  {#if admin && open}
    <div class="edit">
      <div class="row">
        <label class="field grow"><span class="sr-only">ชื่อรายการ</span><input class="input" bind:value={name} maxlength="60" /></label>
        <label class="field price"><span class="sr-only">ราคาต่อชิ้น</span><input class="input num" bind:value={price} inputmode="decimal" /></label>
      </div>
      <div class="row">
        <div class="stepper" role="group" aria-label="จำนวน">
          <button onclick={() => (qty = Math.max(1, qty - 1))} aria-label="ลด">−</button><span class="num">{qty}</span><button onclick={() => (qty = Math.min(999, qty + 1))} aria-label="เพิ่ม">+</button>
        </div>
        <button class="chip" aria-pressed={item.shared} onclick={() => onedit({ shared: !item.shared })}>หารทุกคน</button>
      </div>
      {#if !item.shared}
        <p class="hint">แตะชื่อเพื่อติ๊กแทนเพื่อนที่ไม่ได้เปิดลิงก์</p>
        <div class="chips">
          {#each members as m (m.id)}
            {@const on = item.claims.includes(m.id)}
            <button class="chip person" aria-pressed={on} onclick={() => onclaim(m.id, !on)}><Avatar id={m.id} name={m.name} size={22} />{m.name}</button>
          {/each}
        </div>
      {/if}
      <div class="row end">
        <button class="btn ghost small del" onclick={ondelete}><Icon name="trash" size={16} />ลบ</button>
        <button class="btn small" onclick={() => (open = false)}>ปิด</button>
        <button class="btn primary small" onclick={save}>บันทึก</button>
      </div>
    </div>
  {/if}
</li>

<style>
  .item { list-style: none; background: var(--card); border: 1.5px solid var(--line); border-radius: var(--r); transition: border-color 200ms, background 200ms; animation: in 260ms var(--ease); }
  @keyframes in { from { opacity: 0; transform: translateY(6px) } }
  .item.mine { border-color: color-mix(in srgb, var(--primary) 55%, var(--line)); background: color-mix(in srgb, var(--primary-soft) 60%, var(--card)); }
  .item.warn { border-color: color-mix(in srgb, var(--warn) 45%, var(--line)); }
  .main { display: flex; align-items: stretch; }
  .info { flex: 1; min-width: 0; display: grid; gap: 2px; text-align: left; background: none; border: 0; padding: 12px 14px; cursor: default; }
  .info:not(:disabled) { cursor: pointer; }
  .info:disabled { opacity: 1; color: inherit; }
  .name { font-weight: 600; font-size: 16px; overflow-wrap: anywhere; }
  .meta { color: var(--muted); font-size: 14px; }
  .dot { margin: 0 6px; }
  .who { display: flex; align-items: center; gap: 6px; margin-top: 4px; min-height: 24px; }
  .stack { display: flex; } .stack :global(.av + .av) { margin-left: -6px; }
  .more { font-size: 12px; color: var(--muted); }
  .tag { font-size: 12px; font-weight: 600; padding: 2px 10px; border-radius: 99px; background: var(--slate-100); color: var(--muted); }
  .tag.warn { background: var(--warn-soft); color: var(--warn); }
  .tick { flex: none; width: 84px; display: grid; place-items: center; align-content: center; gap: 4px; border: 0; border-left: 1.5px solid var(--line); background: none; border-radius: 0 var(--r) var(--r) 0; }
  .box { width: 30px; height: 30px; border-radius: 9px; border: 2px solid var(--slate-300); display: grid; place-items: center; transition: all 160ms var(--ease); }
  .tick[aria-pressed='true'] .box { background: var(--primary); border-color: var(--primary); color: var(--on-primary); transform: scale(1.06); }
  .lbl { font-size: 12px; font-weight: 600; color: var(--muted); }
  .tick[aria-pressed='true'] .lbl { color: var(--primary); }
  .edit { border-top: 1px dashed var(--line); padding: 12px 14px 14px; display: grid; gap: 10px; }
  .row { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
  .row.end { justify-content: flex-end; }
  .grow { flex: 1; min-width: 140px; }
  .price { width: 110px; }
  .hint { margin: 0; font-size: 13px; color: var(--muted); }
  .person { display: inline-flex; align-items: center; gap: 6px; padding-left: 6px; }
  .del { color: var(--owe); margin-right: auto; }
  .stepper { display: inline-flex; align-items: center; border: 1.5px solid var(--line); border-radius: 999px; overflow: hidden; }
  .stepper button { width: 40px; height: 40px; border: 0; background: none; font-size: 18px; }
  .stepper span { min-width: 28px; text-align: center; font-weight: 600; }
</style>
