use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Paid {
    #[default]
    No,
    /// member says they transferred
    Sent,
    /// payer confirmed receipt
    Confirmed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub id: u64,
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub token: String,
    pub is_host: bool,
    #[serde(default)]
    pub paid: Paid,
}

impl Member {
    pub fn new(id: u64, name: String, is_host: bool) -> Self {
        Member { id, name, token: String::new(), is_host, paid: Paid::No }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: u64,
    pub name: String,
    /// unit price in satang
    pub price: i64,
    pub qty: i64,
    pub shared: bool,
    pub claims: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: String,
    pub title: String,
    pub promptpay: String,
    pub service_bp: i64,
    pub vat_bp: i64,
    pub discount: i64,
    pub admin_token: String,
    pub members: Vec<Member>,
    pub items: Vec<Item>,
    pub created_ms: i64,
    pub next_id: u64,
    #[serde(default)]
    pub version: u64,
}

impl Room {
    pub fn new(title: String, promptpay: String, service_bp: i64, vat_bp: i64, discount: i64) -> Room {
        Room {
            id: String::new(),
            title,
            promptpay,
            service_bp,
            vat_bp,
            discount,
            admin_token: String::new(),
            members: vec![],
            items: vec![],
            created_ms: 0,
            next_id: 1,
            version: 0,
        }
    }

    pub fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn member_by_token(&self, token: &str) -> Option<&Member> {
        if token.is_empty() {
            return None;
        }
        self.members.iter().find(|m| m.token == token)
    }

    pub fn is_admin(&self, token: &str) -> bool {
        !token.is_empty() && token == self.admin_token
    }

    /// Toggle whether `member` shared `item`. Returns new state.
    pub fn toggle_claim(&mut self, item: u64, member: u64, on: bool) -> Option<bool> {
        let it = self.items.iter_mut().find(|i| i.id == item)?;
        it.claims.retain(|&c| c != member);
        if on {
            it.claims.push(member);
        }
        Some(on)
    }

    pub fn remove_member(&mut self, member: u64) {
        self.members.retain(|m| m.id != member);
        for it in &mut self.items {
            it.claims.retain(|&c| c != member);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_claim_is_idempotent() {
        let mut r = Room::new("x".into(), "".into(), 0, 0, 0);
        r.items.push(Item { id: 1, name: "a".into(), price: 1, qty: 1, shared: false, claims: vec![] });
        r.toggle_claim(1, 5, true);
        r.toggle_claim(1, 5, true);
        assert_eq!(r.items[0].claims, vec![5]);
        r.toggle_claim(1, 5, false);
        assert!(r.items[0].claims.is_empty());
        assert!(r.toggle_claim(9, 5, true).is_none());
    }

    #[test]
    fn tokens_never_serialized_for_public_view_when_cleared() {
        let mut m = Member::new(1, "a".into(), false);
        m.token = "secret".into();
        let mut pubm = m.clone();
        pubm.token.clear();
        assert!(!serde_json::to_string(&pubm).unwrap().contains("secret"));
    }
}
