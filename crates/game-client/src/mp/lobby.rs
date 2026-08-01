//! Join, role, character, loadout bench, spawn request, leave.

use std::cell::RefCell;
use std::rc::Rc;

use game_net::{is_known_character, normalize_display_name, ClientToServer, NetRole};
use game_sim::{prefer_armed_slot, ActiveWeapon, WeaponClass};
use js_sys::Reflect;
use wasm_bindgen::JsCast;

#[cfg(feature = "debug-tools")]
use super::cookie::load_display_name_cookie as load_cookie;
use super::phase::MpPhase;
use super::send::{send_reliable_locked, send_spawn_locked, SPAWN_RETRY_SECS};
use super::session::{join_session, js_error_string};
use super::shared::Shared;
#[cfg(feature = "debug-tools")]
use super::JOIN_ROOM_PREFILL;

/// Debug-console join with cookie name into the alpha prefill room.
#[cfg(feature = "debug-tools")]
pub(crate) fn begin_join(shared: &Rc<RefCell<Shared>>) {
    let name = load_cookie().unwrap_or_else(|| JOIN_ROOM_PREFILL.into());
    begin_join_with(shared, JOIN_ROOM_PREFILL, &name);
}

pub(crate) fn begin_join_with(shared: &Rc<RefCell<Shared>>, room_code: &str, display_name: &str) {
    let mut s = shared.borrow_mut();
    if !s.phase.can_go(MpPhase::Connecting) {
        return;
    }
    let name = match normalize_display_name(display_name) {
        Ok(n) => n,
        Err(reason) => {
            s.last_error = Some(format!("mp: {reason}"));
            return;
        }
    };
    s.phase = MpPhase::Connecting;
    s.clock.clear();
    s.player_id = None;
    s.display_name = None;
    s.remotes.clear();
    s.roster.clear();
    s.last_error = None;
    s.spawn_requested = false;
    s.join_room = room_code.to_string();
    s.join_name = name;
    drop(s);

    let shared = Rc::clone(shared);
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(e) = join_session(Rc::clone(&shared)).await {
            let msg = js_error_string(&e);
            let mut s = shared.borrow_mut();
            s.reset_session();
            s.last_error = Some(format!("mp: join failed: {msg}"));
        }
    });
}

pub(crate) fn choose_play(shared: &RefCell<Shared>) {
    let mut s = shared.borrow_mut();
    if !s.phase.can_go(MpPhase::Character) {
        return;
    }
    s.role = NetRole::Player;
    s.phase = MpPhase::Character;
    s.spawn_requested = false;
    send_reliable_locked(
        &s,
        &ClientToServer::SetRole {
            role: NetRole::Player,
        },
    );
}

pub(crate) fn choose_spectate(shared: &RefCell<Shared>) {
    let mut s = shared.borrow_mut();
    if !s.phase.can_go(MpPhase::Spectating) {
        return;
    }
    s.role = NetRole::Spectator;
    s.phase = MpPhase::Spectating;
    s.spawn_requested = false;
    send_reliable_locked(
        &s,
        &ClientToServer::SetRole {
            role: NetRole::Spectator,
        },
    );
}

/// UI back only — does not resend role.
pub(crate) fn back_to_role(shared: &RefCell<Shared>) {
    let mut s = shared.borrow_mut();
    if !s.phase.can_go(MpPhase::Role) {
        return;
    }
    s.phase = MpPhase::Role;
    s.spawn_requested = false;
}

/// Commit kit and advance to loadout bench (`SetCharacter` only). Character stays frozen after.
pub(crate) fn confirm_character(shared: &RefCell<Shared>, character: u8) -> Option<u8> {
    if !is_known_character(character) {
        return None;
    }
    let mut s = shared.borrow_mut();
    if s.phase != MpPhase::Character || !s.phase.can_go(MpPhase::Ready) {
        return None;
    }
    s.character = character;
    s.role = NetRole::Player;
    s.phase = MpPhase::Ready;
    s.spawn_requested = false;
    s.staged_primary = None;
    s.staged_secondary = None;
    s.staged_active = ActiveWeapon::Primary;
    send_reliable_locked(&s, &ClientToServer::SetCharacter { character });
    Some(character)
}

/// Stage primary (any known letter or empty). Bench only; cancels in-flight Spawn.
pub(crate) fn stage_primary(shared: &RefCell<Shared>, letter: Option<u8>) -> bool {
    if let Some(l) = letter {
        if WeaponClass::from_letter(l).is_none() {
            return false;
        }
    }
    let mut s = shared.borrow_mut();
    if s.phase != MpPhase::Ready {
        return false;
    }
    s.staged_primary = letter;
    s.staged_active = prefer_armed_slot(s.staged_primary, s.staged_secondary, s.staged_active);
    s.spawn_requested = false;
    s.spawn_retry_accum = 0.0;
    true
}

/// Stage secondary (launcher/pistol or empty). Illegal class rejected.
pub(crate) fn stage_secondary(shared: &RefCell<Shared>, letter: Option<u8>) -> bool {
    if let Some(l) = letter {
        match WeaponClass::from_letter(l) {
            Some(c) if c.allowed_in_secondary() => {}
            _ => return false,
        }
    }
    let mut s = shared.borrow_mut();
    if s.phase != MpPhase::Ready {
        return false;
    }
    s.staged_secondary = letter;
    s.staged_active = prefer_armed_slot(s.staged_primary, s.staged_secondary, s.staged_active);
    s.spawn_requested = false;
    s.spawn_retry_accum = 0.0;
    true
}

pub(crate) fn stage_active(shared: &RefCell<Shared>, active: ActiveWeapon) {
    let mut s = shared.borrow_mut();
    if s.phase != MpPhase::Ready {
        return;
    }
    s.staged_active = prefer_armed_slot(s.staged_primary, s.staged_secondary, active);
    s.spawn_requested = false;
    s.spawn_retry_accum = 0.0;
}

/// Death accepted → loadout bench. Staged loadout stays the picker choice (067);
/// living slots from the dead life (incl. floor pickup) do not carry over.
pub(crate) fn return_to_bench_after_death(shared: &RefCell<Shared>) {
    let mut s = shared.borrow_mut();
    if s.phase != MpPhase::Living || !s.phase.can_go(MpPhase::Ready) {
        return;
    }
    s.phase = MpPhase::Ready;
    s.spawn_requested = false;
    s.spawn_retry_accum = 0.0;
}

pub(crate) fn request_spawn(shared: &RefCell<Shared>) {
    let mut s = shared.borrow_mut();
    if s.phase != MpPhase::Ready {
        return;
    }
    s.spawn_requested = true;
    s.spawn_retry_accum = SPAWN_RETRY_SECS;
    send_spawn_locked(&s);
}

pub(crate) fn leave(shared: &RefCell<Shared>) {
    let mut s = shared.borrow_mut();
    if let Some(t) = s.transport.take() {
        if let Ok(close) = Reflect::get(&t, &"close".into()) {
            if let Ok(f) = close.dyn_into::<js_sys::Function>() {
                let _ = f.call0(&t);
            }
        }
    }
    s.reset_session();
}
