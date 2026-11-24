//! Exact bill splitting in satang. Every displayed number sums to the bill total.
use crate::room::Room;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Share {
    pub member_id: u64,
    pub subtotal: i64,
    pub discount: i64,
    pub service: i64,
    pub vat: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Breakdown {
    pub shares: Vec<Share>,
    pub subtotal: i64,
    pub discount: i64,
    pub service: i64,
    pub vat: i64,
    pub total: i64,
    /// sum of items nobody has ticked yet (not in anyone's total)
    pub unclaimed: i64,
    pub unclaimed_items: Vec<u64>,
}

/// Round a*bp/10000 half-up (non-negative inputs).
fn pct(a: i64, bp: i64) -> i64 {
    ((a as i128 * bp as i128 + 5000) / 10000) as i64
}

/// Split `amount` across `weights` proportionally, largest remainder; result sums to `amount`.
pub fn apportion(amount: i64, weights: &[i64]) -> Vec<i64> {
    let sum: i128 = weights.iter().map(|&w| w.max(0) as i128).sum();
    if sum == 0 || amount == 0 {
        return vec![0; weights.len()];
    }
    let mut out = Vec::with_capacity(weights.len());
    let mut rems = Vec::with_capacity(weights.len());
    for (i, &w) in weights.iter().enumerate() {
        let exact = amount as i128 * w.max(0) as i128;
        out.push((exact / sum) as i64);
        rems.push((exact % sum, i));
    }
    let mut left = amount - out.iter().sum::<i64>();
    rems.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for (_, i) in rems {
        if left <= 0 {
            break;
        }
        out[i] += 1;
        left -= 1;
    }
    out
}

pub fn split(room: &Room) -> Breakdown {
    let ids: Vec<u64> = room.members.iter().map(|m| m.id).collect();
    let mut sub = vec![0i64; ids.len()];
    let mut unclaimed = 0;
    let mut unclaimed_items = vec![];

    for item in &room.items {
        let cost = item.price * item.qty;
        if cost <= 0 {
            continue;
        }
        let who: Vec<usize> = if item.shared {
            (0..ids.len()).collect()
        } else {
            item.claims.iter().filter_map(|c| ids.iter().position(|id| id == c)).collect()
        };
        if who.is_empty() {
            unclaimed += cost;
            unclaimed_items.push(item.id);
            continue;
        }
        for (k, part) in apportion(cost, &vec![1; who.len()]).into_iter().enumerate() {
            sub[who[k]] += part;
        }
    }

    let subtotal: i64 = sub.iter().sum();
    let discount = room.discount.clamp(0, subtotal);
    let after = subtotal - discount;
    let service = pct(after, room.service_bp);
    let vat = pct(after + service, room.vat_bp);
    let total = after + service + vat;

    let d = apportion(discount, &sub);
    let s = apportion(service, &sub);
    let v = apportion(vat, &sub);
    let shares = ids
        .iter()
        .enumerate()
        .map(|(i, &member_id)| Share {
            member_id,
            subtotal: sub[i],
            discount: d[i],
            service: s[i],
            vat: v[i],
            total: sub[i] - d[i] + s[i] + v[i],
        })
        .collect();

    Breakdown { shares, subtotal, discount, service, vat, total, unclaimed, unclaimed_items }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room::{Item, Member, Room};

    fn room(n: usize) -> Room {
        let mut r = Room::new("t".into(), "0812345678".into(), 0, 0, 0);
        for i in 0..n {
            r.members.push(Member::new(i as u64 + 1, format!("m{i}"), i == 0));
        }
        r
    }
    fn item(id: u64, price: i64, qty: i64, claims: &[u64], shared: bool) -> Item {
        Item { id, name: format!("i{id}"), price, qty, shared, claims: claims.to_vec() }
    }

    #[test]
    fn apportion_sums_exactly() {
        assert_eq!(apportion(10000, &[1, 1, 1]), vec![3334, 3333, 3333]);
        assert_eq!(apportion(5, &[0, 0]), vec![0, 0]);
        assert_eq!(apportion(100, &[1, 3]), vec![25, 75]);
    }

    #[test]
    fn equal_split_remainder() {
        let mut r = room(3);
        r.items.push(item(1, 10000, 1, &[1, 2, 3], false));
        let b = split(&r);
        let t: Vec<i64> = b.shares.iter().map(|s| s.total).collect();
        assert_eq!(t, vec![3334, 3333, 3333]);
        assert_eq!(b.total, 10000);
    }

    #[test]
    fn shared_item_goes_to_everyone() {
        let mut r = room(4);
        r.items.push(item(1, 40000, 1, &[], true));
        r.items.push(item(2, 9000, 2, &[2], false));
        let b = split(&r);
        assert_eq!(b.shares[0].total, 10000);
        assert_eq!(b.shares[1].total, 28000);
        assert_eq!(b.total, 58000);
    }

    #[test]
    fn discount_then_service_then_vat() {
        let mut r = room(2);
        r.discount = 10000; // ฿100
        r.service_bp = 1000; // 10%
        r.vat_bp = 700; // 7%
        r.items.push(item(1, 60000, 1, &[1], false));
        r.items.push(item(2, 40000, 1, &[2], false));
        let b = split(&r);
        // (1000-100)=900 → +10% = 990 → +7% = 1059.30
        assert_eq!(b.total, 105930);
        assert_eq!(b.shares.iter().map(|s| s.total).sum::<i64>(), b.total);
        assert_eq!(b.shares[0].discount, 6000);
        assert_eq!(b.shares[0].total, 63558);
    }

    #[test]
    fn unclaimed_and_zero_items_excluded() {
        let mut r = room(2);
        r.items.push(item(1, 5000, 1, &[], false));
        r.items.push(item(2, 0, 1, &[1], false));
        r.items.push(item(3, 2000, 1, &[99], false)); // claim by a member who left
        let b = split(&r);
        assert_eq!(b.total, 0);
        assert_eq!(b.unclaimed, 7000);
        assert_eq!(b.unclaimed_items, vec![1, 3]);
    }

    #[test]
    fn totals_always_sum_fuzz() {
        let mut seed = 7u64;
        let mut rnd = |m: i64| { seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); ((seed >> 33) as i64).rem_euclid(m) };
        for _ in 0..300 {
            let n = 1 + rnd(7) as usize;
            let mut r = room(n);
            r.service_bp = rnd(1500);
            r.vat_bp = rnd(1000);
            r.discount = rnd(50000);
            for k in 0..rnd(10) as u64 {
                let claims: Vec<u64> = (1..=n as u64).filter(|_| rnd(2) == 0).collect();
                r.items.push(item(k, rnd(99999), 1 + rnd(4), &claims, rnd(5) == 0));
            }
            let b = split(&r);
            assert_eq!(b.shares.iter().map(|s| s.total).sum::<i64>(), b.total);
            assert!(b.shares.iter().all(|s| s.total >= 0));
        }
    }
}
