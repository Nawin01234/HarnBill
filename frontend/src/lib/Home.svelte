<script lang="ts">
  import { api, ident } from './api'
  import Icon from './Icon.svelte'

  let { go }: { go: (path: string) => void } = $props()

  let title = $state('')
  let host = $state('')
  let pp = $state('')
  let service = $state(0)
  let vat = $state(0)
  let busy = $state(false)
  let err = $state('')
  const recent = ident.recent()

  const ppDigits = $derived(pp.replace(/\D/g, ''))
  const ppOk = $derived(/^0\d{9}$/.test(ppDigits) || /^\d{13}$/.test(ppDigits) || /^\d{15}$/.test(ppDigits))

  async function create(e: SubmitEvent) {
    e.preventDefault()
    err = ''
    if (!title.trim() || !host.trim()) return (err = 'กรอกชื่อมื้อและชื่อของคุณก่อนนะ')
    if (!ppOk) return (err = 'พร้อมเพย์ต้องเป็นเบอร์มือถือ 10 หลัก หรือเลขบัตรประชาชน 13 หลัก')
    busy = true
    try {
      const c = await api.create({ title, promptpay: pp, host_name: host, service_bp: service, vat_bp: vat, discount: 0 })
      ident.set(c.room_id, { member_id: c.member_id, member_token: c.member_token, admin_token: c.admin_token }, title)
      go(`/r/${c.room_id}`)
    } catch (x) { err = (x as Error).message } finally { busy = false }
  }
</script>

<main class="home">
  <section class="hero">
    <div class="logo" aria-hidden="true"><Icon name="receipt" size={26} /></div>
    <h1>หารบิล<br /><span>ติ๊กที่กิน สแกนจ่าย จบ</span></h1>
    <ol class="steps">
      <li><b>1</b>ใส่รายการอาหาร</li>
      <li><b>2</b>ส่งลิงก์ให้เพื่อน</li>
      <li><b>3</b>เพื่อนติ๊กแล้วสแกน QR จ่ายตามยอดตัวเอง</li>
    </ol>
  </section>

  <form class="card" onsubmit={create} novalidate>
    <h2>เปิดบิลใหม่</h2>
    <label class="field"><span>มื้อนี้ชื่ออะไร</span>
      <input class="input" bind:value={title} placeholder="เช่น หมูกระทะวันศุกร์" maxlength="60" autocomplete="off" /></label>
    <label class="field"><span>ชื่อคุณ (คนจ่ายก่อน)</span>
      <input class="input" bind:value={host} placeholder="เช่น นิว" maxlength="24" autocomplete="nickname" /></label>
    <label class="field"><span>พร้อมเพย์ที่ให้เพื่อนโอนเข้า</span>
      <input class="input num" bind:value={pp} inputmode="numeric" placeholder="08x-xxx-xxxx หรือเลขบัตร 13 หลัก" autocomplete="tel"
        aria-invalid={pp.length > 0 && !ppOk} />
      <small><Icon name="lock" size={13} /> เพื่อนจะเห็นแค่ 4 ตัวท้าย แต่ QR จะโอนเข้าบัญชีนี้</small></label>

    <div class="field"><span>เซอร์วิสชาร์จ</span>
      <div class="chips" role="group" aria-label="เซอร์วิสชาร์จ">
        {#each [[0, 'ไม่มี'], [1000, '10%']] as [v, l]}
          <button type="button" class="chip" aria-pressed={service === v} onclick={() => (service = v as number)}>{l}</button>
        {/each}
      </div></div>
    <div class="field"><span>VAT</span>
      <div class="chips" role="group" aria-label="VAT">
        <button type="button" class="chip" aria-pressed={vat === 0} onclick={() => (vat = 0)}>รวมในราคาแล้ว / ไม่มี</button>
        <button type="button" class="chip" aria-pressed={vat === 700} onclick={() => (vat = 700)}>บวกเพิ่ม 7%</button>
      </div></div>

    {#if err}<p class="err" role="alert">{err}</p>{/if}
    <button class="btn primary block" disabled={busy}>{busy ? 'กำลังเปิดบิล…' : 'เปิดบิล'}</button>
    <p class="fine">ไม่ต้องสมัคร ไม่ต้องลงแอป · ห้องจะถูกลบอัตโนมัติใน 30 วัน</p>
  </form>

  {#if recent.length}
    <section class="recent">
      <h2>บิลล่าสุดของคุณ</h2>
      {#each recent as r}
        <a href="#/r/{r.id}" class="rrow">
          <Icon name="receipt" size={18} /><span>{r.title}</span>
          {#if r.admin_token}<em>คุณเป็นคนจ่าย</em>{/if}
        </a>
      {/each}
    </section>
  {/if}
</main>

<style>
  .home { max-width: 520px; margin: 0 auto; padding: 28px 16px calc(40px + var(--safe-b)); display: grid; gap: 20px; }
  .hero { text-align: center; padding: 8px 4px 0; }
  .logo { width: 56px; height: 56px; margin: 0 auto 14px; border-radius: 18px; display: grid; place-items: center; background: var(--primary); color: var(--on-primary); }
  h1 { margin: 0; font-size: 34px; line-height: 1.15; letter-spacing: -.01em; }
  h1 span { font-size: 20px; color: var(--muted); font-weight: 500; }
  .steps { list-style: none; padding: 0; margin: 20px 0 0; display: grid; gap: 8px; text-align: left; }
  .steps li { display: flex; align-items: center; gap: 12px; padding: 10px 14px; background: var(--card); border: 1px solid var(--line); border-radius: var(--r-sm); font-size: 15px; }
  .steps b { flex: none; width: 26px; height: 26px; border-radius: 50%; display: grid; place-items: center; background: var(--primary-soft); color: var(--primary); font-size: 13px; }
  .card { background: var(--card); border: 1px solid var(--line); border-radius: 20px; padding: 20px; display: grid; gap: 16px; }
  .card h2, .recent h2 { margin: 0; font-size: 19px; }
  .field small { display: flex; align-items: center; gap: 4px; }
  .input[aria-invalid='true'] { border-color: var(--owe); }
  .fine { margin: -4px 0 0; text-align: center; font-size: 13px; color: var(--muted); }
  .recent { display: grid; gap: 8px; }
  .rrow { display: flex; align-items: center; gap: 10px; min-height: 52px; padding: 0 14px; background: var(--card); border: 1px solid var(--line); border-radius: var(--r-sm); color: inherit; text-decoration: none; }
  .rrow span { flex: 1; font-weight: 500; }
  .rrow em { font-style: normal; font-size: 12px; color: var(--primary); background: var(--primary-soft); padding: 2px 8px; border-radius: 99px; }
</style>
