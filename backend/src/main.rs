mod api;
mod hub;
mod money;
mod promptpay;
mod room;

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::services::{ServeDir, ServeFile};

fn web_dir() -> PathBuf {
    if let Ok(d) = std::env::var("HARNBILL_WEB") {
        return d.into();
    }
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("web")))
        .filter(|p| p.join("index.html").exists())
        .unwrap_or_else(|| PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../frontend/dist")))
}

/// Best-effort LAN address (no packets are sent; UDP connect only picks a route).
fn lan_ip() -> Option<std::net::IpAddr> {
    let s = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("8.8.8.8:80").ok()?;
    Some(s.local_addr().ok()?.ip())
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut port: u16 = 8080;
    let mut data = PathBuf::from("harnbill.db");
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--port" | "-p" => port = args.next().context("--port needs a value")?.parse()?,
            "--db" => data = args.next().context("--db needs a path")?.into(),
            _ => {
                println!("harnbill [--port 8080] [--db harnbill.db]");
                return Ok(());
            }
        }
    }
    let hub = Arc::new(hub::Hub::open(&data)?);
    let web = web_dir();
    let app = api::router(hub)
        .fallback_service(ServeDir::new(&web).not_found_service(ServeFile::new(web.join("index.html"))));
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    println!("หารบิล พร้อมแล้ว");
    println!("  เครื่องนี้:                http://localhost:{port}");
    if let Some(ip) = lan_ip() {
        println!("  มือถือ (Wi-Fi เดียวกัน):   http://{ip}:{port}");
    }
    axum::serve(listener, app).await?;
    Ok(())
}
