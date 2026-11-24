//! Thai PromptPay QR (EMVCo merchant-presented) payload generation.
use anyhow::{Result, bail};

/// CRC-16/CCITT-FALSE (poly 0x1021, init 0xFFFF).
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &b in data {
        crc ^= (b as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
        }
    }
    crc
}

/// Returns (sub-tag, value) for the PromptPay target.
/// Mobile 0XXXXXXXXX → ("01", "0066XXXXXXXXX"); 13-digit ID → ("02", id); 15-digit e-wallet → ("03", id).
pub fn normalize_id(raw: &str) -> Result<(&'static str, String)> {
    let d: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    match d.len() {
        10 if d.starts_with('0') => Ok(("01", format!("0066{}", &d[1..]))),
        11 if d.starts_with("66") => Ok(("01", format!("00{d}"))),
        13 => Ok(("02", d)),
        15 => Ok(("03", d)),
        _ => bail!("พร้อมเพย์ต้องเป็นเบอร์มือถือ 10 หลัก หรือเลขบัตรประชาชน 13 หลัก"),
    }
}

fn tlv(tag: &str, value: &str) -> String {
    format!("{tag}{:02}{value}", value.len())
}

/// Build payload for `satang` (0 = let payer type the amount).
pub fn payload(id: &str, satang: i64) -> Result<String> {
    let (sub, target) = normalize_id(id)?;
    let merchant = tlv("00", "A000000677010111") + &tlv(sub, &target);
    let mut p = String::new();
    p += &tlv("00", "01");
    p += &tlv("01", if satang > 0 { "12" } else { "11" });
    p += &tlv("29", &merchant);
    p += &tlv("58", "TH");
    p += &tlv("53", "764");
    if satang > 0 {
        p += &tlv("54", &format!("{}.{:02}", satang / 100, satang % 100));
    }
    p += "6304";
    let crc = crc16(p.as_bytes());
    Ok(format!("{p}{crc:04X}"))
}

pub fn qr_svg(data: &str) -> Result<String> {
    let code = qrcode::QrCode::with_error_correction_level(data.as_bytes(), qrcode::EcLevel::M)?;
    Ok(code
        .render::<qrcode::render::svg::Color>()
        .min_dimensions(240, 240)
        .quiet_zone(true)
        .dark_color(qrcode::render::svg::Color("#0b1b33"))
        .light_color(qrcode::render::svg::Color("#ffffff"))
        .build())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_check_value() {
        assert_eq!(crc16(b"123456789"), 0x29B1);
    }

    #[test]
    fn normalizes_ids() {
        assert_eq!(normalize_id("081-234-5678").unwrap(), ("01", "0066812345678".into()));
        assert_eq!(normalize_id("1234567890123").unwrap().0, "02");
        assert!(normalize_id("12345").is_err());
        assert!(normalize_id("1812345678").is_err());
    }

    #[test]
    fn payload_structure_and_crc() {
        let p = payload("0812345678", 42250).unwrap();
        assert!(p.starts_with("000201010212"));
        assert!(p.contains("29370016A000000677010111011300668123456785802TH5303764"));
        assert!(p.contains("5406422.50"));
        let (body, crc) = p.split_at(p.len() - 4);
        assert_eq!(format!("{:04X}", crc16(body.as_bytes())), crc);
        // static QR (no amount)
        assert!(payload("0812345678", 0).unwrap().starts_with("000201010211"));
    }

    #[test]
    fn known_vector_matches_reference() {
        // CRCs cross-checked with Python's independent binascii.crc_hqx(data, 0xFFFF).
        assert_eq!(
            payload("0812345678", 0).unwrap(),
            "00020101021129370016A000000677010111011300668123456785802TH530376463045D82"
        );
        assert!(payload("0812345678", 42250).unwrap().ends_with("6304733A"));
    }

    #[test]
    fn svg_renders() {
        assert!(qr_svg("hello").unwrap().contains("<svg"));
    }
}
