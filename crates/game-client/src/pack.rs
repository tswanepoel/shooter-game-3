//! Cooked pack load (SGPK + manifest). See feature 010.

use std::collections::HashMap;

use serde::Deserialize;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;

const MAGIC: &[u8; 4] = b"SGPK";

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub packs: Vec<PackRecord>,
}

#[derive(Debug, Deserialize)]
pub struct PackRecord {
    pub id: String,
    pub url: String,
    pub hash: String,
    pub size: u64,
}

#[derive(Debug, Deserialize)]
struct PackHeader {
    assets: Vec<PackAssetMeta>,
}

#[derive(Debug, Deserialize)]
struct PackAssetMeta {
    id: String,
    offset: u64,
    size: u64,
}

/// Parsed SGPK v1: content-addressed blob of named assets (glb/png bytes as-is).
pub struct Pack {
    payload: Vec<u8>,
    index: HashMap<String, (usize, usize)>,
}

impl Pack {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 8 {
            return Err("pack too short".into());
        }
        if &bytes[0..4] != MAGIC {
            return Err("bad pack magic (expected SGPK)".into());
        }
        let header_len = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
        let header_start: usize = 8;
        let header_end = header_start
            .checked_add(header_len)
            .ok_or_else(|| "pack header length overflow".to_string())?;
        if header_end > bytes.len() {
            return Err("pack header truncated".into());
        }
        let header: PackHeader = serde_json::from_slice(&bytes[header_start..header_end])
            .map_err(|e| format!("pack header json: {e}"))?;

        let payload = bytes[header_end..].to_vec();
        let mut index = HashMap::with_capacity(header.assets.len());
        for a in header.assets {
            let start = a.offset as usize;
            let end = start
                .checked_add(a.size as usize)
                .ok_or_else(|| format!("asset {} size overflow", a.id))?;
            if end > payload.len() {
                return Err(format!("asset {} out of range", a.id));
            }
            if index.insert(a.id.clone(), (start, end)).is_some() {
                return Err(format!("duplicate asset id {}", a.id));
            }
        }

        Ok(Self { payload, index })
    }

    pub fn get(&self, id: &str) -> Result<&[u8], String> {
        let (start, end) = self
            .index
            .get(id)
            .copied()
            .ok_or_else(|| format!("pack missing asset {id}"))?;
        Ok(&self.payload[start..end])
    }
}

pub async fn fetch_bytes(url: &str) -> Result<Vec<u8>, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let resp_val = JsFuture::from(window.fetch_with_str(url)).await?;
    let resp: Response = resp_val
        .dyn_into()
        .map_err(|_| JsValue::from_str("fetch response type"))?;
    if !resp.ok() {
        return Err(JsValue::from_str(&format!(
            "fetch {url} failed: HTTP {}",
            resp.status()
        )));
    }
    let buf = JsFuture::from(resp.array_buffer()?).await?;
    let arr = js_sys::Uint8Array::new(&buf);
    let mut bytes = vec![0u8; arr.length() as usize];
    arr.copy_to(&mut bytes);
    Ok(bytes)
}

pub async fn load_pack(pack_id: &str) -> Result<Pack, JsValue> {
    let manifest_bytes = fetch_bytes("/manifest.json").await?;
    let manifest: Manifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|e| JsValue::from_str(&format!("manifest json: {e}")))?;
    let record = manifest
        .packs
        .iter()
        .find(|p| p.id == pack_id)
        .ok_or_else(|| JsValue::from_str(&format!("manifest missing pack {pack_id}")))?;

    let pack_bytes = fetch_bytes(&record.url).await?;
    if pack_bytes.len() as u64 != record.size {
        return Err(JsValue::from_str(&format!(
            "pack {} size mismatch: got {} expected {}",
            pack_id,
            pack_bytes.len(),
            record.size
        )));
    }

    // Optional integrity: sha-256 of pack bytes vs manifest hash.
    let hash = sha256_hex(&pack_bytes);
    if hash != record.hash {
        return Err(JsValue::from_str(&format!(
            "pack {} hash mismatch",
            pack_id
        )));
    }

    Pack::parse(&pack_bytes).map_err(|e| JsValue::from_str(&e))
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(data);
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}
