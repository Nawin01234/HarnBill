use crate::hub::{Hub, HubError, ItemIn, NewRoom, Op, RoomView, Settings};
use crate::room::Paid;
use axum::{
    Json, Router,
    extract::{Path, Query, State, ws::{Message, WebSocket, WebSocketUpgrade}},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

pub type AppState = Arc<Hub>;

impl IntoResponse for HubError {
    fn into_response(self) -> Response {
        let (code, msg) = match self {
            HubError::NotFound => (StatusCode::NOT_FOUND, "ไม่พบห้องหรือรายการนี้".to_string()),
            HubError::Forbidden => (StatusCode::FORBIDDEN, "คุณไม่มีสิทธิ์ทำสิ่งนี้".to_string()),
            HubError::Bad(m) => (StatusCode::BAD_REQUEST, m),
            HubError::Conflict(m) => (StatusCode::CONFLICT, m),
        };
        (code, Json(json!({ "error": msg }))).into_response()
    }
}

type R<T> = Result<Json<T>, HubError>;

fn tok(h: &HeaderMap) -> String {
    h.get("x-token").and_then(|v| v.to_str().ok()).unwrap_or("").to_string()
}

pub fn router(hub: AppState) -> Router {
    Router::new()
        .route("/api/rooms", post(create))
        .route("/api/rooms/{id}", get(get_room).patch(settings))
        .route("/api/rooms/{id}/items", post(add_item))
        .route("/api/rooms/{id}/items/{item}", patch(edit_item).delete(delete_item))
        .route("/api/rooms/{id}/join", post(join))
        .route("/api/rooms/{id}/claim", post(claim))
        .route("/api/rooms/{id}/paid", post(paid))
        .route("/api/rooms/{id}/members/{mid}", delete(remove_member))
        .route("/api/rooms/{id}/qr/{member}", get(qr))
        .route("/api/rooms/{id}/live", get(live))
        .route("/api/qr", get(text_qr))
        .with_state(hub)
}

async fn create(State(h): State<AppState>, Json(n): Json<NewRoom>) -> R<crate::hub::Created> {
    Ok(Json(h.create(n)?))
}
async fn get_room(State(h): State<AppState>, Path(id): Path<String>) -> R<RoomView> {
    Ok(Json(h.get(&id)?))
}
async fn settings(State(h): State<AppState>, Path(id): Path<String>, hd: HeaderMap, Json(s): Json<Settings>) -> R<RoomView> {
    Ok(Json(h.apply(&id, &tok(&hd), Op::Settings(s))?))
}
async fn add_item(State(h): State<AppState>, Path(id): Path<String>, hd: HeaderMap, Json(i): Json<ItemIn>) -> R<RoomView> {
    Ok(Json(h.apply(&id, &tok(&hd), Op::AddItem(i))?))
}
async fn edit_item(State(h): State<AppState>, Path((id, item)): Path<(String, u64)>, hd: HeaderMap, Json(i): Json<ItemIn>) -> R<RoomView> {
    Ok(Json(h.apply(&id, &tok(&hd), Op::EditItem(item, i))?))
}
async fn delete_item(State(h): State<AppState>, Path((id, item)): Path<(String, u64)>, hd: HeaderMap) -> R<RoomView> {
    Ok(Json(h.apply(&id, &tok(&hd), Op::DeleteItem(item))?))
}
async fn remove_member(State(h): State<AppState>, Path((id, mid)): Path<(String, u64)>, hd: HeaderMap) -> R<RoomView> {
    Ok(Json(h.apply(&id, &tok(&hd), Op::RemoveMember(mid))?))
}

#[derive(Deserialize)]
struct JoinIn { name: String }
async fn join(State(h): State<AppState>, Path(id): Path<String>, Json(j): Json<JoinIn>) -> Result<Json<serde_json::Value>, HubError> {
    let (joined, room) = h.join(&id, &j.name)?;
    Ok(Json(json!({ "member_id": joined.member_id, "member_token": joined.member_token, "room": room })))
}

#[derive(Deserialize)]
struct ClaimIn { item: u64, member: u64, on: bool }
async fn claim(State(h): State<AppState>, Path(id): Path<String>, hd: HeaderMap, Json(c): Json<ClaimIn>) -> R<RoomView> {
    Ok(Json(h.apply(&id, &tok(&hd), Op::Claim { item: c.item, member: c.member, on: c.on })?))
}

#[derive(Deserialize)]
struct PaidIn { member: u64, state: Paid }
async fn paid(State(h): State<AppState>, Path(id): Path<String>, hd: HeaderMap, Json(p): Json<PaidIn>) -> R<RoomView> {
    Ok(Json(h.apply(&id, &tok(&hd), Op::Paid { member: p.member, state: p.state })?))
}

fn svg(body: String) -> Response {
    ([(header::CONTENT_TYPE, "image/svg+xml"), (header::CACHE_CONTROL, "no-store")], body).into_response()
}

async fn qr(State(h): State<AppState>, Path((id, member)): Path<(String, String)>) -> Result<Response, HubError> {
    let mid: u64 = member.trim_end_matches(".svg").parse().map_err(|_| HubError::NotFound)?;
    Ok(svg(h.qr_for(&id, mid)?.0))
}

#[derive(Deserialize)]
struct TextQ { text: String }
async fn text_qr(Query(q): Query<TextQ>) -> Result<Response, HubError> {
    if q.text.len() > 512 {
        return Err(HubError::Bad("ข้อความยาวเกินไป".into()));
    }
    Ok(svg(crate::promptpay::qr_svg(&q.text).map_err(|e| HubError::Bad(e.to_string()))?))
}

async fn live(State(h): State<AppState>, Path(id): Path<String>, ws: WebSocketUpgrade) -> Result<Response, HubError> {
    let rx = h.subscribe(&id)?;
    let first = serde_json::to_string(&h.get(&id)?).unwrap_or_default();
    Ok(ws.on_upgrade(move |sock| pump(sock, rx, first)))
}

async fn pump(mut sock: WebSocket, mut rx: tokio::sync::broadcast::Receiver<Arc<str>>, first: String) {
    if sock.send(Message::Text(first.into())).await.is_err() {
        return;
    }
    loop {
        tokio::select! {
            m = rx.recv() => match m {
                Ok(s) => if sock.send(Message::Text(s.to_string().into())).await.is_err() { break },
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            },
            m = sock.recv() => if !matches!(m, Some(Ok(_))) { break },
        }
    }
}
