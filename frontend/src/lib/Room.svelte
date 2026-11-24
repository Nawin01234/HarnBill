<script lang="ts">
  import { onMount, untrack } from 'svelte'
  import { api, ident, baht, bahtShort, toSatang, pct, type Room, type Identity, type Paid } from './api'
  import Avatar from './Avatar.svelte'
  import Icon from './Icon.svelte'
  import Sheet from './Sheet.svelte'
  import ItemRow from './ItemRow.svelte'

  let { id, go }: { id: string; go: (p: string) => void } = $props()

  let room = $state<Room | null>(null)
  let me = $state<Identity | null>(ident.get(untrack(() => id)))
  let up = $state(false)
  let loadErr = $state('')
  let toast = $state('')

  let joinName = $state('')
  let joinErr = $state('')
  let joining = $state(false)

  let nName = $state('')
  let nPrice = $state('')
  let nQty = $state(1)
  let nShared = $state(false)
  let addErr = $state('')
  let nameEl: HTMLInputElement | undefined = $state()

  let payOpen = $state(false)
  let inviteOpen = $state(false)
  let setOpen = $state(false)
  let sTitle = $state(''), sPP = $state(''), sService = $state(0), sVat = $state(0), sDiscount = $state('')
  let setErr = $state('')

  const admin = $derived(!!me?.admin_token)
  const token = $derived(me?.admin_token || me?.member_token || '')
  const myId = $derived(me?.member_id ?? null)
  const meMember = $derived(room?.members.find((m) => m.id === myId) ?? null)
  const host = $derived(room?.members.find((m) => m.is_host) ?? null)
  const share = (mid: number) => room?.breakdown.shares.find((s) => s.member_id === mid)
  const myShare = $derived(myId != null ? share(myId) : undefined)
  const owedToHost = $derived(room ? room.breakdown.shares.filter((s) => s.member_id !== host?.id).reduce((a, s) => a + s.total, 0) : 0)
  const received = $derived(room ? room.members.filter((m) => !m.is_host && m.paid === 'confirmed').reduce((a, m) => a + (share(m.id)?.total ?? 0), 0) : 0)
  const link = $derived(`${location.origin}${location.pathname}#/r/${id}`)

  function accept(r: Room) {
    if (!room || r.version >= room.version) room = r
  }

  onMount(() => {
    api.room(id).then(accept).catch((e) => (loadErr = e.message))
    return api.live(id, accept, (s) => (up = s))
  })

  // member got removed by host → forget identity
  $effect(() => {
    if (room && me && !room.members.some((m) => m.id === me!.member_id)) { ident.forget(id); me = null }
  })

  function say(m: string) { toast = m; setTimeout(() => toast === m && (toast = ''), 2600) }
  async function run(p: Promise<Room>) {
    try { accept(await p) } catch (e) { say((e as Error).message) }
  }

  async function join(e: SubmitEvent) {
    e.preventDefault()
    joinErr = ''
    if (!joinName.trim()) return (joinErr = 'ใส่ชื่อเล่นก่อนนะ')
    joining = true
    try {
      const j = await api.join(id, joinName)
      me = { member_id: j.member_id, member_token: j.member_token }
      ident.set(id, me, j.room.title)
      accept(j.room)
    } catch (x) { joinErr = (x as Error).message } finally { joining = false }
  }

  async function addItem(e: SubmitEvent) {
    e.preventDefault()
    addErr = ''
    const p = toSatang(nPrice)
    if (!nName.trim()) return (addErr = 'ใส่ชื่อรายการ')
    if (Number.isNaN(p) || p <= 0) return (addErr = 'ใส่ราคาเป็นตัวเลข')
    try {
      accept(await api.addItem(id, token, { name: nName, price: p, qty: nQty, shared: nShared }))
      nName = ''; nPrice = ''; nQty = 1; nShared = false
      nameEl?.focus()
    } catch (x) { addErr = (x as Error).message }
  }

  function openSettings() {
    if (!room) return
    sTitle = room.title; sPP = ''; sService = room.service_bp; sVat = room.vat_bp; sDiscount = room.discount ? String(room.discount / 100) : ''
    setErr = ''; setOpen = true
  }
  async function saveSettings() {
    const d = sDiscount.trim() ? toSatang(sDiscount) : 0
    if (Number.isNaN(d)) return (setErr = 'ส่วนลดต้องเป็นตัวเลข')
    try {
      accept(await api.settings(id, token, { title: sTitle, service_bp: sService, vat_bp: sVat, discount: d, ...(sPP.trim() ? { promptpay: sPP } : {}) }))
      setOpen = false
    } catch (x) { setErr = (x as Error).message }
  }

  async function setPaid(mid: number, state: Paid) { await run(api.paid(id, token, mid, state)) }

  async function copy() {
    try { await navigator.clipboard.writeText(link); say('คัดลอกลิงก์แล้ว') } catch { say(link) }
  }
  async function nativeShare() {
    if (navigator.share) { try { await navigator.share({ title: room?.title, text: `มาหารบิล "${room?.title}" กัน ติ๊กที่กินแล้วสแกนจ่ายได้เลย`, url: link }) } catch {} }
    else copy()
  }
  const lineUrl = $derived(`https://line.me/R/share?text=${encodeURIComponent(`มาหารบิล "${room?.title ?? ''}" กัน ติ๊กที่กินแล้วสแกนจ่ายได้เลย\n${link}`)}`)

  const paidLabel: Record<Paid, string> = { no: 'ยังไม่จ่าย', sent: 'แจ้งโอนแล้ว', confirmed: 'ได้รับแล้ว' }
</script>

{#if loadErr}
  <main class="center"><h1>ไม่พบบิลนี้</h1><p>{loadErr}</p><button class="btn primary" onclick={() => go('/')}>เปิดบิลใหม่</button></main>
{:else if !room}
  <main class="center" aria-busy="true"><div class="spinner"></div></main>
{:else}
  <header class="top">
    <button class="icon" onclick={() => go('/')} aria-label="กลับหน้าแรก"><Icon name="back" /></button>
    <div class="t">
      <h1>{room.title}</h1>
      <p><span class="live" class:off={!up}></span>{room.members.length} คน · {room.items.length} รายการ</p>
    </div>
    {#if admin}<button class="icon" onclick={openSettings} aria-label="ตั้งค่าบิล"><Icon name="settings" /></button>{/if}
    <button class="btn primary small" onclick={() => (inviteOpen = true)}><Icon name="share" size={18} />ชวนเพื่อน</button>
  </header>

  <main class="room">
    {#if !me}
      <form class="card join" onsubmit={join}>
        <h2>คุณคือใครในโต๊ะนี้?</h2>
        <p class="muted">ใส่ชื่อเล่นแล้วติ๊กสิ่งที่คุณกิน ระบบจะคิดยอดและสร้าง QR ให้</p>
        {#if room.members.length}
          <div class="people">{#each room.members as m (m.id)}<span class="pchip"><Avatar id={m.id} name={m.name} size={22} />{m.name}</span>{/each}</div>
        {/if}
        <div class="row">
          <label class="field grow"><span class="sr-only">ชื่อเล่นของคุณ</span>
            <input class="input" bind:value={joinName} placeholder="ชื่อเล่นของคุณ" maxlength="24" autocomplete="nickname" /></label>
          <button class="btn primary" disabled={joining}>เข้าร่วม</button>
        </div>
        {#if joinErr}<p class="err" role="alert">{joinErr}</p>{/if}
      </form>
    {/if}

    <section aria-label="รายการอาหาร">
      <div class="sec-h">
        <h2>รายการ</h2>
        {#if me && !admin}<span class="muted small">แตะ “ฉันกิน” ที่รายการของคุณ</span>{/if}
      </div>

      {#if room.breakdown.unclaimed > 0}
        <div class="banner warn" role="status">ยังมี {room.breakdown.unclaimed_items.length} รายการ ({bahtShort(room.breakdown.unclaimed)}) ที่ยังไม่มีใครติ๊ก ยอดยังไม่ครบบิล</div>
      {/if}

      <ul class="items">
        {#each room.items as it (it.id)}
          <ItemRow item={it} members={room.members} me={myId} {admin}
            unclaimed={room.breakdown.unclaimed_items.includes(it.id)}
            onclaim={(mid, on) => run(api.claim(id, token, it.id, mid, on))}
            onedit={(p) => run(api.editItem(id, token, it.id, p))}
            ondelete={() => run(api.deleteItem(id, token, it.id))} />
        {:else}
          <li class="empty">{admin ? 'เริ่มใส่รายการอาหารด้านล่างได้เลย' : 'รอคนจ่ายใส่รายการอาหารสักครู่…'}</li>
        {/each}
      </ul>

      {#if admin}
        <form class="card add" onsubmit={addItem}>
          <div class="row">
            <label class="field grow"><span class="sr-only">ชื่อรายการ</span>
              <input class="input" bind:this={nameEl} bind:value={nName} placeholder="ชื่อรายการ เช่น หมูสามชั้น" maxlength="60" autocomplete="off" /></label>
            <label class="field price"><span class="sr-only">ราคาต่อชิ้น (บาท)</span>
              <input class="input num" bind:value={nPrice} placeholder="ราคา ฿" inputmode="decimal" /></label>
          </div>
          <div class="row">
            <div class="stepper" role="group" aria-label="จำนวน">
              <button type="button" onclick={() => (nQty = Math.max(1, nQty - 1))} aria-label="ลดจำนวน">−</button>
              <span class="num" aria-live="polite">×{nQty}</span>
              <button type="button" onclick={() => (nQty = Math.min(999, nQty + 1))} aria-label="เพิ่มจำนวน">+</button>
            </div>
            <button type="button" class="chip" aria-pressed={nShared} onclick={() => (nShared = !nShared)}>หารทุกคน</button>
            <button class="btn primary small add-btn"><Icon name="plus" size={18} />เพิ่ม</button>
          </div>
          {#if addErr}<p class="err" role="alert">{addErr}</p>{/if}
        </form>
      {/if}
    </section>

    <section aria-label="สรุปยอดทุกคน">
      <div class="sec-h"><h2>สรุปยอด</h2><span class="muted small num">ทั้งบิล {baht(room.breakdown.total)}</span></div>
      <div class="card sum">
        {#each room.members as m (m.id)}
          {@const s = share(m.id)}
          <div class="prow" class:you={m.id === myId}>
            <Avatar id={m.id} name={m.name} size={36} />
            <div class="pinfo">
              <span class="pname">{m.name}{#if m.id === myId}<em>คุณ</em>{/if}</span>
              <span class="pstat {m.is_host ? 'host' : m.paid}">{m.is_host ? 'คนจ่ายบิล' : paidLabel[m.paid]}</span>
            </div>
            <span class="pamt num">{baht(s?.total ?? 0)}</span>
            {#if admin && !m.is_host}
              {#if m.paid !== 'confirmed'}
                <button class="btn small confirm" onclick={() => setPaid(m.id, 'confirmed')} aria-label="ยืนยันว่าได้รับเงินจาก {m.name}"><Icon name="check" size={16} /></button>
              {:else}
                <button class="btn small ghost" onclick={() => setPaid(m.id, 'no')} aria-label="ยกเลิกการยืนยันของ {m.name}"><Icon name="x" size={16} /></button>
              {/if}
            {/if}
          </div>
        {/each}
        <dl class="totals num">
          <div><dt>รวมรายการที่ติ๊ก</dt><dd>{baht(room.breakdown.subtotal)}</dd></div>
          {#if room.breakdown.discount}<div><dt>ส่วนลด</dt><dd>−{baht(room.breakdown.discount)}</dd></div>{/if}
          {#if room.service_bp}<div><dt>เซอร์วิสชาร์จ {pct(room.service_bp)}</dt><dd>{baht(room.breakdown.service)}</dd></div>{/if}
          {#if room.vat_bp}<div><dt>VAT {pct(room.vat_bp)}</dt><dd>{baht(room.breakdown.vat)}</dd></div>{/if}
          <div class="grand"><dt>รวมทั้งหมด</dt><dd>{baht(room.breakdown.total)}</dd></div>
        </dl>
      </div>
    </section>
  </main>

  {#if me && meMember}
    <footer class="bar">
      {#if meMember.is_host}
        <div class="bar-t"><span class="muted small">ได้คืนแล้ว</span><strong class="num">{bahtShort(received)} <span class="muted">/ {bahtShort(owedToHost)}</span></strong></div>
        <div class="progress" aria-hidden="true"><i style:width="{owedToHost ? (received / owedToHost) * 100 : 0}%"></i></div>
      {:else}
        <div class="bar-t"><span class="muted small">ยอดของคุณ</span><strong class="num">{baht(myShare?.total ?? 0)}</strong></div>
        <button class="btn primary" disabled={!myShare?.total} onclick={() => (payOpen = true)}><Icon name="qr" size={18} />
          {meMember.paid === 'confirmed' ? 'จ่ายครบแล้ว' : 'จ่ายด้วยพร้อมเพย์'}</button>
      {/if}
    </footer>
  {/if}

  <Sheet bind:open={payOpen} title="โอนให้ {host?.name ?? 'คนจ่าย'}">
    {#if myShare && myId != null}
      <div class="pay">
        <img class="qr" src={api.qrUrl(id, myId, room.version)} alt="QR พร้อมเพย์ ยอด {baht(myShare.total)}" width="240" height="240" />
        <p class="amt num">{baht(myShare.total)}</p>
        <p class="muted small">พร้อมเพย์ {room.promptpay_masked} · สแกนด้วยแอปธนาคารใดก็ได้</p>
        <dl class="totals num">
          <div><dt>ค่าอาหารของคุณ</dt><dd>{baht(myShare.subtotal)}</dd></div>
          {#if myShare.discount}<div><dt>ส่วนลด (เฉลี่ยตามสัดส่วน)</dt><dd>−{baht(myShare.discount)}</dd></div>{/if}
          {#if myShare.service}<div><dt>เซอร์วิสชาร์จ</dt><dd>{baht(myShare.service)}</dd></div>{/if}
          {#if myShare.vat}<div><dt>VAT</dt><dd>{baht(myShare.vat)}</dd></div>{/if}
        </dl>
        <div class="pay-actions">
          <a class="btn" href={api.qrUrl(id, myId, room.version)} download="harnbill-{room.title}.svg"><Icon name="download" size={18} />บันทึก QR</a>
          {#if meMember?.paid === 'no'}
            <button class="btn primary" onclick={async () => { await setPaid(myId!, 'sent'); payOpen = false; say('แจ้ง ' + (host?.name ?? '') + ' แล้วว่าคุณโอนแล้ว') }}><Icon name="check" size={18} />ฉันโอนแล้ว</button>
          {:else}
            <span class="pstat {meMember?.paid}">{meMember ? paidLabel[meMember.paid] : ''}</span>
          {/if}
        </div>
        <p class="muted small">ถ้ายอดเปลี่ยน (เช่น มีคนติ๊กเพิ่ม) QR จะอัปเดตให้อัตโนมัติ</p>
      </div>
    {/if}
  </Sheet>

  <Sheet bind:open={inviteOpen} title="ชวนเพื่อนเข้าบิล">
    <div class="pay">
      <img class="qr" src={api.textQr(link)} alt="QR ลิงก์เข้าบิล" width="220" height="220" />
      <p class="muted small">ให้เพื่อนที่โต๊ะสแกนด้วยกล้องมือถือ หรือส่งลิงก์ลงกลุ่ม</p>
      <div class="linkbox num">{link}</div>
      <div class="pay-actions three">
        <button class="btn" onclick={copy}><Icon name="copy" size={18} />คัดลอก</button>
        <a class="btn line" href={lineUrl} target="_blank" rel="noopener">LINE</a>
        <button class="btn primary" onclick={nativeShare}><Icon name="share" size={18} />แชร์</button>
      </div>
      {#if /localhost|127\.0\.0\.1/.test(location.host)}
        <p class="banner warn small"><Icon name="wifi" size={16} /> คุณเปิดผ่าน localhost เพื่อนจะเข้าไม่ได้ ให้เปิดผ่านลิงก์ “มือถือ (Wi-Fi เดียวกัน)” ที่แสดงตอนรันเซิร์ฟเวอร์แทน</p>
      {/if}
    </div>
  </Sheet>

  <Sheet bind:open={setOpen} title="ตั้งค่าบิล">
    <div class="settings">
      <label class="field"><span>ชื่อมื้อ</span><input class="input" bind:value={sTitle} maxlength="60" /></label>
      <label class="field"><span>เปลี่ยนพร้อมเพย์</span><input class="input num" bind:value={sPP} inputmode="numeric" placeholder="ปัจจุบัน {room.promptpay_masked} (เว้นว่าง = ไม่เปลี่ยน)" /></label>
      <div class="field"><span>เซอร์วิสชาร์จ</span><div class="chips">
        {#each [0, 500, 1000] as v}<button type="button" class="chip" aria-pressed={sService === v} onclick={() => (sService = v)}>{v ? pct(v) : 'ไม่มี'}</button>{/each}
      </div></div>
      <div class="field"><span>VAT</span><div class="chips">
        <button type="button" class="chip" aria-pressed={sVat === 0} onclick={() => (sVat = 0)}>รวมแล้ว / ไม่มี</button>
        <button type="button" class="chip" aria-pressed={sVat === 700} onclick={() => (sVat = 700)}>บวกเพิ่ม 7%</button>
      </div></div>
      <label class="field"><span>ส่วนลดทั้งบิล (บาท)</span><input class="input num" bind:value={sDiscount} inputmode="decimal" placeholder="0" />
        <small>เฉลี่ยให้แต่ละคนตามสัดส่วนที่กิน</small></label>
      {#if setErr}<p class="err" role="alert">{setErr}</p>{/if}
      <button class="btn primary block" onclick={saveSettings}>บันทึก</button>
    </div>
  </Sheet>
{/if}

{#if toast}<div class="toast" role="status">{toast}</div>{/if}

<style>
  .center { min-height: 80dvh; display: grid; place-content: center; justify-items: center; gap: 8px; text-align: center; padding: 24px; }
  .spinner { width: 36px; height: 36px; border-radius: 50%; border: 4px solid var(--line); border-top-color: var(--primary); animation: s .8s linear infinite; }
  @keyframes s { to { transform: rotate(360deg) } }
  .top { position: sticky; top: 0; z-index: 10; display: flex; align-items: center; gap: 8px; padding: 10px 12px; background: color-mix(in srgb, var(--bg) 88%, transparent); backdrop-filter: blur(10px); border-bottom: 1px solid var(--line); }
  .icon { width: 44px; height: 44px; flex: none; border-radius: 50%; border: 0; background: none; display: grid; place-items: center; }
  .icon:hover { background: var(--slate-100); }
  .t { flex: 1; min-width: 0; }
  .t h1 { margin: 0; font-size: 18px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .t p { margin: 0; font-size: 13px; color: var(--muted); display: flex; align-items: center; gap: 6px; }
  .live { width: 8px; height: 8px; border-radius: 50%; background: var(--primary); box-shadow: 0 0 0 3px var(--primary-soft); }
  .live.off { background: var(--slate-300); box-shadow: none; }

  .room { max-width: 640px; margin: 0 auto; padding: 16px 16px calc(120px + var(--safe-b)); display: grid; gap: 24px; }
  .card { background: var(--card); border: 1px solid var(--line); border-radius: 18px; padding: 16px; }
  .sec-h { display: flex; align-items: baseline; justify-content: space-between; gap: 8px; margin-bottom: 10px; }
  .sec-h h2, .join h2 { margin: 0; font-size: 19px; }
  .muted { color: var(--muted); } .small { font-size: 13px; }
  .row { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
  .grow { flex: 1; min-width: 150px; }
  .price { width: 120px; }
  .join { display: grid; gap: 12px; border: 2px solid var(--primary); }
  .join p { margin: 0; }
  .people { display: flex; flex-wrap: wrap; gap: 6px; }
  .pchip { display: inline-flex; align-items: center; gap: 6px; padding: 3px 10px 3px 4px; border-radius: 99px; background: var(--slate-100); font-size: 14px; }
  .items { padding: 0; margin: 0; display: grid; gap: 10px; }
  .empty { list-style: none; text-align: center; padding: 28px 12px; color: var(--muted); border: 1.5px dashed var(--line); border-radius: var(--r); }
  .add { margin-top: 12px; display: grid; gap: 10px; border-style: dashed; }
  .add-btn { margin-left: auto; }
  .stepper { display: inline-flex; align-items: center; border: 1.5px solid var(--line); border-radius: 999px; overflow: hidden; background: var(--card); }
  .stepper button { width: 40px; height: 40px; border: 0; background: none; font-size: 18px; }
  .stepper span { min-width: 34px; text-align: center; font-weight: 600; }
  .banner { display: flex; align-items: center; gap: 8px; padding: 10px 14px; border-radius: var(--r-sm); font-size: 14px; margin: 0 0 10px; }
  .banner.warn { background: var(--warn-soft); color: var(--warn); }

  .sum { display: grid; gap: 4px; padding: 8px 12px; }
  .prow { display: flex; align-items: center; gap: 12px; padding: 8px 4px; border-radius: 12px; }
  .prow.you { background: var(--primary-soft); padding-inline: 8px; }
  .pinfo { flex: 1; min-width: 0; display: grid; }
  .pname { font-weight: 600; display: flex; align-items: center; gap: 6px; }
  .pname em { font-style: normal; font-size: 11px; color: var(--primary); border: 1px solid currentColor; border-radius: 99px; padding: 0 6px; }
  .pstat { font-size: 13px; color: var(--owe); }
  .pstat.sent { color: var(--warn); } .pstat.confirmed { color: var(--primary); } .pstat.host { color: var(--muted); }
  .pamt { font-weight: 700; }
  .confirm { width: 40px; padding: 0; color: var(--primary); }
  .totals { margin: 8px 4px 4px; padding-top: 10px; border-top: 1px dashed var(--line); display: grid; gap: 4px; font-size: 14px; }
  .totals div { display: flex; justify-content: space-between; }
  .totals dt { color: var(--muted); } .totals dd { margin: 0; }
  .totals .grand { font-weight: 700; font-size: 16px; } .totals .grand dt { color: var(--fg); }

  .bar { position: fixed; left: 0; right: 0; bottom: 0; z-index: 20; display: flex; align-items: center; gap: 12px; flex-wrap: wrap;
    padding: 12px 16px calc(12px + var(--safe-b)); background: var(--card); border-top: 1px solid var(--line); box-shadow: 0 -8px 30px rgba(15,23,42,.08); }
  .bar > * { max-width: 640px; }
  .bar-t { flex: 1; display: grid; line-height: 1.2; }
  .bar-t strong { font-size: 24px; }
  .progress { flex-basis: 100%; height: 6px; border-radius: 9px; background: var(--slate-100); overflow: hidden; }
  .progress i { display: block; height: 100%; background: var(--primary); transition: width 400ms var(--ease); }

  .pay { display: grid; justify-items: center; gap: 10px; text-align: center; }
  .qr { width: 240px; height: 240px; border-radius: 16px; border: 1px solid var(--line); background: #fff; padding: 6px; }
  .amt { margin: 0; font-size: 32px; font-weight: 700; }
  .pay p { margin: 0; }
  .pay .totals { width: 100%; text-align: left; }
  .pay-actions { display: flex; gap: 8px; width: 100%; align-items: center; justify-content: center; }
  .pay-actions > * { flex: 1; text-decoration: none; }
  .linkbox { width: 100%; padding: 10px 12px; border-radius: var(--r-sm); background: var(--slate-100); font-size: 13px; overflow-wrap: anywhere; }
  .line { background: #06c755; border-color: #06c755; color: #fff; }
  .settings { display: grid; gap: 14px; }

  .toast { position: fixed; z-index: 60; left: 50%; bottom: calc(100px + var(--safe-b)); transform: translateX(-50%); max-width: calc(100vw - 32px);
    background: var(--slate-900); color: #fff; padding: 10px 16px; border-radius: 12px; font-size: 14px; animation: t 200ms var(--ease); }
  @keyframes t { from { opacity: 0; transform: translate(-50%, 8px) } }
</style>
