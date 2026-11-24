# HarnBill (หารบิล) — Design

## Intent
Splitting a restaurant bill among friends is a weekly chore in Thailand. The payer
opens a room, enters items, shares a link (LINE) or shows an invite QR at the
table. Friends tick what they ate, everyone sees their amount update live, and
each person gets a PromptPay QR for exactly their share. No install, no signup.

User feedback that shaped this: the previous project was "hard to use and too
narrow". So: zero setup, works from a phone, useful to anyone.

## Goals
- Create room (title, PromptPay ID, service %, VAT %, discount ฿) → admin token.
- Add/edit/remove items (name, price, qty). Items can be "shared by everyone".
- Join with nickname (no account); tick/untick items you shared.
- Live updates for everyone via WebSocket.
- Exact money math in satang: per-person amounts always sum to the bill total.
- Per-person PromptPay QR (EMVCo payload + CRC16, SVG rendered server-side).
- Mark "โอนแล้ว" (member) / confirm received (admin).
- Invite QR + copy link + LINE share.
- Mobile-first Thai UI.

## Non-goals
OCR of receipts, accounts/login, payment verification with banks, multi-currency.

## Anti-goals
- Any total that doesn't add up to the bill. - Anyone but the payer editing prices.

## Money rules (my call, standard Thai restaurant order)
1. Item cost split equally among its claimers (shared items: among all members).
2. Discount allocated proportionally to each person's subtotal.
3. Service charge % applied on (subtotal − discount).
4. VAT % applied on (after-discount + service).
5. Rounded to satang with largest-remainder so the sum equals the rounded bill total.
Unclaimed items are shown as a warning and excluded from people's totals.

## Approaches
1. **Rust axum + WebSocket rooms, Svelte SPA (chosen)** — server is the single source
   of truth; clients send REST mutations, server broadcasts full room state.
2. CRDT/peer-to-peer — no server needed but PromptPay math and authority get messy.
3. Plain static page with link-encoded state — no live ticking, defeats the point.

## Architecture
- `backend/` Rust: axum 0.8 (REST + WS), tokio broadcast per room, rusqlite (room JSON
  snapshots, 30-day expiry), `qrcode` for SVG, rand for ids/tokens.
  Modules: `money.rs` (split), `promptpay.rs` (payload + CRC), `room.rs` (model + ops),
  `db.rs`, `api.rs`, `main.rs`.
- Binds 0.0.0.0:8080 by default so phones on the same Wi-Fi can join; prints LAN URL.
- `frontend/` Svelte 5 + Vite + TS, served by the Rust binary.
- Tokens: admin token (creator) and member token (per nickname) kept in localStorage.
