//! Display-name cookie pre-fill (051).

use game_net::normalize_display_name;
use wasm_bindgen::JsCast;

const NAME_COOKIE: &str = "sg_display_name";

pub fn load_display_name_cookie() -> Option<String> {
    let doc = web_sys::window()?.document()?;
    let html_doc: web_sys::HtmlDocument = doc.dyn_into().ok()?;
    let cookie = html_doc.cookie().ok()?;
    for part in cookie.split(';') {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix(&format!("{NAME_COOKIE}=")) {
            let s = urlencoding_decode(rest);
            if normalize_display_name(&s).is_ok() {
                return Some(s);
            }
        }
    }
    None
}

pub fn save_display_name_cookie(name: &str) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let Ok(html_doc) = doc.dyn_into::<web_sys::HtmlDocument>() else {
        return;
    };
    let encoded = urlencoding_encode(name);
    let _ = html_doc.set_cookie(&format!(
        "{NAME_COOKIE}={encoded}; path=/; max-age=31536000; SameSite=Lax"
    ));
}

fn urlencoding_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn urlencoding_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let h = || -> Option<u8> {
                let hi = (bytes[i + 1] as char).to_digit(16)?;
                let lo = (bytes[i + 2] as char).to_digit(16)?;
                Some((hi * 16 + lo) as u8)
            };
            if let Some(b) = h() {
                out.push(b);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            out.push(b' ');
        } else {
            out.push(bytes[i]);
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
