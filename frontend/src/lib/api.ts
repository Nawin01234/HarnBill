export type Paid = 'no' | 'sent' | 'confirmed'
export interface Member { id: number; name: string; is_host: boolean; paid: Paid }
export interface Item { id: number; name: string; price: number; qty: number; shared: boolean; claims: number[] }
export interface Share { member_id: number; subtotal: number; discount: number; service: number; vat: number; total: number }
export interface Breakdown { shares: Share[]; subtotal: number; discount: number; service: number; vat: number; total: number; unclaimed: number; unclaimed_items: number[] }
export interface Room {
  id: string; title: string; promptpay_masked: string; service_bp: number; vat_bp: number; discount: number
  members: Member[]; items: Item[]; breakdown: Breakdown; version: number
}
export interface Identity { member_id: number; member_token: string; admin_token?: string }

const KEY = 'harnbill:rooms'
type Saved = Record<string, Identity & { title: string; at: number }>
function load(): Saved {
  try { return JSON.parse(localStorage.getItem(KEY) || '{}') } catch { return {} }
}
export const ident = {
  get: (room: string): Identity | null => load()[room] ?? null,
  set(room: string, id: Identity, title: string) {
    const all = load(); all[room] = { ...id, title, at: Date.now() }
    try { localStorage.setItem(KEY, JSON.stringify(all)) } catch {}
  },
  forget(room: string) {
    const all = load(); delete all[room]
    try { localStorage.setItem(KEY, JSON.stringify(all)) } catch {}
  },
  recent: () => Object.entries(load()).map(([id, v]) => ({ id, ...v })).sort((a, b) => b.at - a.at).slice(0, 6),
}

async function call<T>(method: string, url: string, body?: unknown, token?: string): Promise<T> {
  const r = await fetch(url, {
    method,
    headers: { 'content-type': 'application/json', ...(token ? { 'x-token': token } : {}) },
    body: body === undefined ? undefined : JSON.stringify(body),
  })
  const data = await r.json().catch(() => ({}))
  if (!r.ok) throw new Error(data.error ?? 'มีบางอย่างผิดพลาด ลองใหม่อีกครั้ง')
  return data as T
}

export const api = {
  create: (b: { title: string; promptpay: string; host_name: string; service_bp: number; vat_bp: number; discount: number }) =>
    call<{ room_id: string; admin_token: string; member_id: number; member_token: string }>('POST', '/api/rooms', b),
  room: (id: string) => call<Room>('GET', `/api/rooms/${id}`),
  join: (id: string, name: string) => call<{ member_id: number; member_token: string; room: Room }>('POST', `/api/rooms/${id}/join`, { name }),
  settings: (id: string, t: string, b: Partial<{ title: string; promptpay: string; service_bp: number; vat_bp: number; discount: number }>) =>
    call<Room>('PATCH', `/api/rooms/${id}`, b, t),
  addItem: (id: string, t: string, b: { name: string; price: number; qty: number; shared: boolean }) => call<Room>('POST', `/api/rooms/${id}/items`, b, t),
  editItem: (id: string, t: string, item: number, b: Partial<{ name: string; price: number; qty: number; shared: boolean }>) =>
    call<Room>('PATCH', `/api/rooms/${id}/items/${item}`, b, t),
  deleteItem: (id: string, t: string, item: number) => call<Room>('DELETE', `/api/rooms/${id}/items/${item}`, undefined, t),
  claim: (id: string, t: string, item: number, member: number, on: boolean) => call<Room>('POST', `/api/rooms/${id}/claim`, { item, member, on }, t),
  paid: (id: string, t: string, member: number, state: Paid) => call<Room>('POST', `/api/rooms/${id}/paid`, { member, state }, t),
  removeMember: (id: string, t: string, member: number) => call<Room>('DELETE', `/api/rooms/${id}/members/${member}`, undefined, t),
  qrUrl: (id: string, member: number, v: number) => `/api/rooms/${id}/qr/${member}.svg?v=${v}`,
  textQr: (text: string) => `/api/qr?text=${encodeURIComponent(text)}`,
  live(id: string, on: (r: Room) => void, state: (up: boolean) => void) {
    let ws: WebSocket | null = null, closed = false
    const go = () => {
      ws = new WebSocket(`${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}/api/rooms/${id}/live`)
      ws.onopen = () => state(true)
      ws.onmessage = (m) => on(JSON.parse(m.data))
      ws.onclose = () => { state(false); if (!closed) setTimeout(go, 1500) }
    }
    go()
    return () => { closed = true; ws?.close() }
  },
}

const nf = new Intl.NumberFormat('th-TH', { minimumFractionDigits: 2, maximumFractionDigits: 2 })
const nf0 = new Intl.NumberFormat('th-TH', { maximumFractionDigits: 2 })
export const baht = (satang: number) => `฿${nf.format(satang / 100)}`
export const bahtShort = (satang: number) => `฿${nf0.format(satang / 100)}`
/** "259", "259.5", "1,200" → satang; NaN-safe */
export const toSatang = (s: string) => {
  const n = Number(String(s).replace(/[,\s฿]/g, ''))
  return Number.isFinite(n) && n >= 0 ? Math.round(n * 100) : NaN
}
export const pct = (bp: number) => `${nf0.format(bp / 100)}%`

const COLORS = ['#047857', '#b45309', '#1d4ed8', '#be185d', '#7c3aed', '#0e7490', '#b91c1c', '#4d7c0f']
export const colorOf = (id: number) => COLORS[id % COLORS.length]
export const initial = (name: string) => [...name.trim()].find((c) => /[\p{L}\p{N}]/u.test(c))?.toUpperCase() ?? '?'
