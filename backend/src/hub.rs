//! Room registry: in-memory rooms + SQLite persistence + per-room broadcast.
use crate::money::{Breakdown, split};
use crate::promptpay;
use crate::room::{Item, Member, Paid, Room};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

pub const MAX_MEMBERS: usize = 40;
pub const MAX_ITEMS: usize = 200;
const TTL_MS: i64 = 30 * 24 * 3600 * 1000;

#[derive(Debug, PartialEq)]
pub enum HubError {
    NotFound,
    Forbidden,
    Bad(String),
    Conflict(String),
}
pub type HResult<T> = Result<T, HubError>;
fn bad<T>(m: &str) -> HResult<T> {
    Err(HubError::Bad(m.into()))
}

#[derive(Serialize, Clone, Debug)]
pub struct RoomView {
    pub id: String,
    pub title: String,
    pub promptpay_masked: String,
    pub service_bp: i64,
    pub vat_bp: i64,
    pub discount: i64,
    pub members: Vec<Member>,
    pub items: Vec<Item>,
    pub breakdown: Breakdown,
    pub version: u64,
}

#[derive(Serialize, Debug)]
pub struct Created {
    pub room_id: String,
    pub admin_token: String,
    pub member_id: u64,
    pub member_token: String,
}

#[derive(Serialize, Debug)]
pub struct Joined {
    pub member_id: u64,
    pub member_token: String,
}

#[derive(Deserialize, Debug, Default)]
pub struct NewRoom {
    pub title: String,
    pub promptpay: String,
    pub host_name: String,
    #[serde(default)]
    pub service_bp: i64,
    #[serde(default)]
    pub vat_bp: i64,
    #[serde(default)]
    pub discount: i64,
}

#[derive(Deserialize, Debug, Default)]
pub struct Settings {
    pub title: Option<String>,
    pub promptpay: Option<String>,
    pub service_bp: Option<i64>,
    pub vat_bp: Option<i64>,
    pub discount: Option<i64>,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct ItemIn {
    pub name: Option<String>,
    pub price: Option<i64>,
    pub qty: Option<i64>,
    pub shared: Option<bool>,
}

pub enum Op {
    Settings(Settings),
    AddItem(ItemIn),
    EditItem(u64, ItemIn),
    DeleteItem(u64),
    Claim { item: u64, member: u64, on: bool },
    Paid { member: u64, state: Paid },
    RemoveMember(u64),
}

struct Slot {
    room: Mutex<Room>,
    tx: broadcast::Sender<Arc<str>>,
}

pub struct Hub {
    db: Mutex<Connection>,
    rooms: Mutex<HashMap<String, Arc<Slot>>>,
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn token(len: usize) -> String {
    const ABC: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let mut buf = vec![0u8; len];
    getrandom(&mut buf);
    buf.iter().map(|b| ABC[*b as usize % ABC.len()] as char).collect()
}
fn getrandom(buf: &mut [u8]) {
    use rand::RngExt;
    let mut r = rand::rng();
    for b in buf {
        *b = r.random();
    }
}

fn clean_name(s: &str, max: usize) -> HResult<String> {
    let t: String = s.trim().chars().filter(|c| !c.is_control()).take(max).collect();
    if t.is_empty() { bad("กรุณาใส่ชื่อ") } else { Ok(t) }
}

fn check_rates(service_bp: i64, vat_bp: i64, discount: i64) -> HResult<()> {
    if !(0..=3000).contains(&service_bp) || !(0..=3000).contains(&vat_bp) {
        return bad("เปอร์เซ็นต์ต้องอยู่ระหว่าง 0–30%");
    }
    if !(0..=100_000_000).contains(&discount) {
        return bad("ส่วนลดไม่ถูกต้อง");
    }
    Ok(())
}

fn mask(pp: &str) -> String {
    let d: String = pp.chars().filter(|c| c.is_ascii_digit()).collect();
    if d.len() <= 4 {
        return d;
    }
    format!("{}{}", "•".repeat(d.len() - 4), &d[d.len() - 4..])
}

pub fn view(r: &Room) -> RoomView {
    let mut members = r.members.clone();
    for m in &mut members {
        m.token.clear();
    }
    RoomView {
        id: r.id.clone(),
        title: r.title.clone(),
        promptpay_masked: mask(&r.promptpay),
        service_bp: r.service_bp,
        vat_bp: r.vat_bp,
        discount: r.discount,
        members,
        items: r.items.clone(),
        breakdown: split(r),
        version: r.version,
    }
}

impl Hub {
    pub fn open(path: &Path) -> anyhow::Result<Hub> {
        let db = Connection::open(path)?;
        db.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS rooms(id TEXT PRIMARY KEY, json TEXT NOT NULL, updated_ms INTEGER NOT NULL);",
        )?;
        db.execute("DELETE FROM rooms WHERE updated_ms < ?1", params![now_ms() - TTL_MS])?;
        Ok(Hub { db: Mutex::new(db), rooms: Mutex::new(HashMap::new()) })
    }

    fn slot(&self, id: &str) -> HResult<Arc<Slot>> {
        if let Some(s) = self.rooms.lock().unwrap().get(id) {
            return Ok(s.clone());
        }
        let json: Option<String> = self
            .db
            .lock()
            .unwrap()
            .query_row("SELECT json FROM rooms WHERE id=?1", params![id], |r| r.get(0))
            .optional()
            .map_err(|e| HubError::Bad(e.to_string()))?;
        let room: Room = serde_json::from_str(&json.ok_or(HubError::NotFound)?).map_err(|_| HubError::NotFound)?;
        let slot = Arc::new(Slot { room: Mutex::new(room), tx: broadcast::channel(64).0 });
        Ok(self.rooms.lock().unwrap().entry(id.to_string()).or_insert(slot).clone())
    }

    fn save(&self, r: &Room) {
        if let Ok(json) = serde_json::to_string(r) {
            let _ = self.db.lock().unwrap().execute(
                "INSERT INTO rooms(id, json, updated_ms) VALUES(?1,?2,?3)
                 ON CONFLICT(id) DO UPDATE SET json=excluded.json, updated_ms=excluded.updated_ms",
                params![r.id, json, now_ms()],
            );
        }
    }

    fn publish(&self, slot: &Slot, r: &mut Room) -> RoomView {
        r.version += 1;
        self.save(r);
        let v = view(r);
        if let Ok(s) = serde_json::to_string(&v) {
            let _ = slot.tx.send(s.into());
        }
        v
    }

    pub fn create(&self, n: NewRoom) -> HResult<Created> {
        let title = clean_name(&n.title, 60)?;
        let host = clean_name(&n.host_name, 24)?;
        promptpay::normalize_id(&n.promptpay).map_err(|e| HubError::Bad(e.to_string()))?;
        check_rates(n.service_bp, n.vat_bp, n.discount)?;
        let mut r = Room::new(title, n.promptpay.trim().to_string(), n.service_bp, n.vat_bp, n.discount);
        r.id = token(8);
        r.admin_token = token(24);
        r.created_ms = now_ms();
        let mid = r.alloc_id();
        let mut m = Member::new(mid, host, true);
        m.token = token(24);
        let out = Created { room_id: r.id.clone(), admin_token: r.admin_token.clone(), member_id: mid, member_token: m.token.clone() };
        r.members.push(m);
        self.save(&r);
        let slot = Arc::new(Slot { room: Mutex::new(r), tx: broadcast::channel(64).0 });
        self.rooms.lock().unwrap().insert(out.room_id.clone(), slot);
        Ok(out)
    }

    pub fn get(&self, id: &str) -> HResult<RoomView> {
        let s = self.slot(id)?;
        let r = s.room.lock().unwrap();
        Ok(view(&r))
    }

    pub fn subscribe(&self, id: &str) -> HResult<broadcast::Receiver<Arc<str>>> {
        Ok(self.slot(id)?.tx.subscribe())
    }

    pub fn join(&self, id: &str, name: &str) -> HResult<(Joined, RoomView)> {
        let name = clean_name(name, 24)?;
        let s = self.slot(id)?;
        let mut r = s.room.lock().unwrap();
        if r.members.iter().any(|m| m.name.to_lowercase() == name.to_lowercase()) {
            return Err(HubError::Conflict("ชื่อนี้มีคนใช้แล้ว ลองเติมอะไรต่อท้ายนิดนึง".into()));
        }
        if r.members.len() >= MAX_MEMBERS {
            return bad("ห้องเต็มแล้ว");
        }
        let mid = r.alloc_id();
        let mut m = Member::new(mid, name, false);
        m.token = token(24);
        let j = Joined { member_id: mid, member_token: m.token.clone() };
        r.members.push(m);
        let v = self.publish(&s, &mut r);
        Ok((j, v))
    }

    pub fn qr_for(&self, id: &str, member: u64) -> HResult<(String, i64)> {
        let s = self.slot(id)?;
        let r = s.room.lock().unwrap();
        let b = split(&r);
        let share = b.shares.iter().find(|x| x.member_id == member).ok_or(HubError::NotFound)?;
        let p = promptpay::payload(&r.promptpay, share.total).map_err(|e| HubError::Bad(e.to_string()))?;
        Ok((promptpay::qr_svg(&p).map_err(|e| HubError::Bad(e.to_string()))?, share.total))
    }

    pub fn apply(&self, id: &str, tok: &str, op: Op) -> HResult<RoomView> {
        let s = self.slot(id)?;
        let mut r = s.room.lock().unwrap();
        let admin = r.is_admin(tok);
        let me = r.member_by_token(tok).map(|m| m.id);
        let need_admin = |ok: bool| if ok { Ok(()) } else { Err(HubError::Forbidden) };

        match op {
            Op::Settings(st) => {
                need_admin(admin)?;
                if let Some(t) = st.title { r.title = clean_name(&t, 60)?; }
                if let Some(p) = st.promptpay {
                    promptpay::normalize_id(&p).map_err(|e| HubError::Bad(e.to_string()))?;
                    r.promptpay = p.trim().to_string();
                }
                let (sv, vt, dc) = (st.service_bp.unwrap_or(r.service_bp), st.vat_bp.unwrap_or(r.vat_bp), st.discount.unwrap_or(r.discount));
                check_rates(sv, vt, dc)?;
                (r.service_bp, r.vat_bp, r.discount) = (sv, vt, dc);
            }
            Op::AddItem(i) => {
                need_admin(admin)?;
                if r.items.len() >= MAX_ITEMS { return bad("รายการเยอะเกินไป"); }
                let name = clean_name(i.name.as_deref().unwrap_or(""), 60)?;
                let price = check_price(i.price.unwrap_or(0))?;
                let qty = check_qty(i.qty.unwrap_or(1))?;
                let iid = r.alloc_id();
                r.items.push(Item { id: iid, name, price, qty, shared: i.shared.unwrap_or(false), claims: vec![] });
            }
            Op::EditItem(iid, i) => {
                need_admin(admin)?;
                let it = r.items.iter_mut().find(|x| x.id == iid).ok_or(HubError::NotFound)?;
                if let Some(n) = i.name { it.name = clean_name(&n, 60)?; }
                if let Some(p) = i.price { it.price = check_price(p)?; }
                if let Some(q) = i.qty { it.qty = check_qty(q)?; }
                if let Some(sh) = i.shared { it.shared = sh; }
            }
            Op::DeleteItem(iid) => {
                need_admin(admin)?;
                let before = r.items.len();
                r.items.retain(|x| x.id != iid);
                if r.items.len() == before { return Err(HubError::NotFound); }
            }
            Op::Claim { item, member, on } => {
                need_admin(admin || me == Some(member))?;
                if !r.members.iter().any(|m| m.id == member) { return Err(HubError::NotFound); }
                r.toggle_claim(item, member, on).ok_or(HubError::NotFound)?;
            }
            Op::Paid { member, state } => {
                let own = me == Some(member);
                need_admin(admin || (own && state != Paid::Confirmed))?;
                let m = r.members.iter_mut().find(|m| m.id == member).ok_or(HubError::NotFound)?;
                if m.is_host { return bad("คนจ่ายไม่ต้องโอนให้ตัวเอง"); }
                m.paid = state;
            }
            Op::RemoveMember(mid) => {
                need_admin(admin || me == Some(mid))?;
                let m = r.members.iter().find(|m| m.id == mid).ok_or(HubError::NotFound)?;
                if m.is_host { return bad("ลบคนจ่ายออกไม่ได้"); }
                r.remove_member(mid);
            }
        }
        Ok(self.publish(&s, &mut r))
    }
}

fn check_price(p: i64) -> HResult<i64> {
    if (0..=100_000_000).contains(&p) { Ok(p) } else { bad("ราคาไม่ถูกต้อง") }
}
fn check_qty(q: i64) -> HResult<i64> {
    if (1..=999).contains(&q) { Ok(q) } else { bad("จำนวนต้องอยู่ระหว่าง 1–999") }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hub() -> (tempfile::TempDir, Hub) {
        let d = tempfile::tempdir().unwrap();
        let h = Hub::open(&d.path().join("t.db")).unwrap();
        (d, h)
    }
    fn mk(h: &Hub) -> Created {
        h.create(NewRoom { title: "หมูกระทะ".into(), promptpay: "0812345678".into(), host_name: "นิว".into(), ..Default::default() }).unwrap()
    }
    fn add(h: &Hub, c: &Created, price: i64) -> u64 {
        let v = h.apply(&c.room_id, &c.admin_token, Op::AddItem(ItemIn { name: Some("x".into()), price: Some(price), ..Default::default() })).unwrap();
        v.items.last().unwrap().id
    }

    #[test]
    fn invalid_promptpay_rejected() {
        let (_d, h) = hub();
        let e = h.create(NewRoom { title: "a".into(), promptpay: "123".into(), host_name: "b".into(), ..Default::default() });
        assert!(matches!(e, Err(HubError::Bad(_))));
    }

    #[test]
    fn non_admin_cannot_edit_items_or_settings() {
        let (_d, h) = hub();
        let c = mk(&h);
        let (j, _) = h.join(&c.room_id, "บอส").unwrap();
        let r = h.apply(&c.room_id, &j.member_token, Op::AddItem(ItemIn { name: Some("x".into()), price: Some(1), ..Default::default() }));
        assert_eq!(r.unwrap_err(), HubError::Forbidden);
        let r = h.apply(&c.room_id, "", Op::Settings(Settings { discount: Some(1), ..Default::default() }));
        assert_eq!(r.unwrap_err(), HubError::Forbidden);
    }

    #[test]
    fn member_claims_only_for_self_admin_for_anyone() {
        let (_d, h) = hub();
        let c = mk(&h);
        let it = add(&h, &c, 30000);
        let (a, _) = h.join(&c.room_id, "บอส").unwrap();
        let (b, _) = h.join(&c.room_id, "มายด์").unwrap();
        assert!(h.apply(&c.room_id, &a.member_token, Op::Claim { item: it, member: a.member_id, on: true }).is_ok());
        assert_eq!(
            h.apply(&c.room_id, &a.member_token, Op::Claim { item: it, member: b.member_id, on: true }).unwrap_err(),
            HubError::Forbidden
        );
        let v = h.apply(&c.room_id, &c.admin_token, Op::Claim { item: it, member: b.member_id, on: true }).unwrap();
        assert_eq!(v.breakdown.total, 30000);
        assert_eq!(v.breakdown.shares.iter().find(|s| s.member_id == b.member_id).unwrap().total, 15000);
    }

    #[test]
    fn join_dedupes_nickname_case_insensitive() {
        let (_d, h) = hub();
        let c = mk(&h);
        h.join(&c.room_id, "Boss").unwrap();
        assert!(matches!(h.join(&c.room_id, " boss ").unwrap_err(), HubError::Conflict(_)));
    }

    #[test]
    fn paid_flow_and_tokens_hidden() {
        let (_d, h) = hub();
        let c = mk(&h);
        let (a, _) = h.join(&c.room_id, "บอส").unwrap();
        let v = h.apply(&c.room_id, &a.member_token, Op::Paid { member: a.member_id, state: Paid::Sent }).unwrap();
        assert_eq!(v.members[1].paid, Paid::Sent);
        assert_eq!(h.apply(&c.room_id, &a.member_token, Op::Paid { member: a.member_id, state: Paid::Confirmed }).unwrap_err(), HubError::Forbidden);
        assert!(h.apply(&c.room_id, &c.admin_token, Op::Paid { member: a.member_id, state: Paid::Confirmed }).is_ok());
        let json = serde_json::to_string(&h.get(&c.room_id).unwrap()).unwrap();
        assert!(!json.contains(&a.member_token) && !json.contains(&c.admin_token) && !json.contains("0812345678"));
    }

    #[test]
    fn persists_across_restart() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("t.db");
        let id = { let h = Hub::open(&p).unwrap(); let c = mk(&h); add(&h, &c, 500); c.room_id };
        let h2 = Hub::open(&p).unwrap();
        assert_eq!(h2.get(&id).unwrap().items.len(), 1);
        assert_eq!(h2.get("nope").unwrap_err(), HubError::NotFound);
    }

    #[test]
    fn qr_amount_matches_share() {
        let (_d, h) = hub();
        let c = mk(&h);
        let it = add(&h, &c, 12345);
        let (a, _) = h.join(&c.room_id, "บอส").unwrap();
        h.apply(&c.room_id, &a.member_token, Op::Claim { item: it, member: a.member_id, on: true }).unwrap();
        let (svg, amt) = h.qr_for(&c.room_id, a.member_id).unwrap();
        assert_eq!(amt, 12345);
        assert!(svg.contains("<svg"));
    }
}
