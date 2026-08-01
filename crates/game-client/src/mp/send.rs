//! Reliable/datagram C2S helpers while Shared is borrowed.

use game_net::{encode_c2s_frame, ClientToServer, NetActiveWeapon, DEFAULT_MAP};
use game_sim::ActiveWeapon;
use js_sys::Uint8Array;

use super::shared::Shared;

/// Resend Spawn while Ready after user confirm until YouSpawned.
pub(crate) const SPAWN_RETRY_SECS: f32 = 0.5;

pub(crate) fn send_spawn_locked(s: &Shared) {
    send_reliable_locked(
        s,
        &ClientToServer::Spawn {
            primary: s.staged_primary,
            secondary: s.staged_secondary,
            active: match s.staged_active {
                ActiveWeapon::Primary => NetActiveWeapon::Primary,
                ActiveWeapon::Secondary => NetActiveWeapon::Secondary,
            },
        },
    );
}

pub(crate) fn send_pick_map_locked(s: &Shared) {
    send_reliable_locked(s, &ClientToServer::PickMap { map: DEFAULT_MAP });
}

pub(crate) fn send_start_match_locked(s: &Shared) {
    send_reliable_locked(s, &ClientToServer::StartMatch);
}

pub(crate) fn send_reliable_locked(s: &Shared, msg: &ClientToServer) {
    let Ok(payload) = encode_c2s_frame(msg) else {
        return;
    };
    let Some(w) = s.reliable_writer.as_ref() else {
        return;
    };
    let arr = Uint8Array::from(payload.as_slice());
    let _ = w.write_with_chunk(&arr);
}
