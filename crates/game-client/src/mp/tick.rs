//! Per-frame probe / drive / spawn-retry while in room.

use std::cell::RefCell;

use game_net::{encode_c2s, ClientToServer, TICK_HZ};
use game_sim::SelfState;
use js_sys::Uint8Array;

use super::drive::state_to_drive;
use super::phase::MpPhase;
use super::send::{
    send_pick_map_locked, send_spawn_locked, send_start_match_locked, SPAWN_RETRY_SECS,
};
use super::shared::{client_now_secs, Shared};

pub(crate) fn on_frame(shared: &RefCell<Shared>, dt: f32, self_state: &SelfState) {
    let mut s = shared.borrow_mut();
    if !s.phase.in_room() {
        return;
    }
    s.join_secs += dt;
    s.probe_accum += dt;
    s.drive_accum += dt;

    let dgram = s.dgram_writer.clone();
    let living = s.phase == MpPhase::Living;
    let ready = s.phase == MpPhase::Ready;

    let mut send_spawn = false;
    let mut send_pick_map = false;
    let mut send_start_match = false;
    if ready && s.spawn_requested {
        s.spawn_retry_accum += dt;
        if s.spawn_retry_accum >= SPAWN_RETRY_SECS {
            s.spawn_retry_accum = 0.0;
            send_spawn = true;
        }
    }

    if s.room_leader {
        if s.match_view.map.is_none() && !s.pick_map_sent {
            send_pick_map = true;
            s.pick_map_sent = true;
        } else if s.match_view.map.is_some() && !s.match_view.started && !s.start_match_sent {
            send_start_match = true;
            s.start_match_sent = true;
        }
    }

    let probe_interval = if s.join_secs < 1.0 { 0.05 } else { 0.2 };
    let send_probe = s.probe_accum >= probe_interval;
    if send_probe {
        s.probe_accum = 0.0;
    }

    let drive_interval = 1.0 / TICK_HZ as f32;
    let send_drive = living && s.drive_accum >= drive_interval;
    let drive_payload = if send_drive {
        s.drive_accum = 0.0;
        let tick = s.clock.estimated_tick(client_now_secs()).unwrap_or(0);
        let drive = state_to_drive(self_state);
        encode_c2s(&ClientToServer::DriveSample { tick, drive }).ok()
    } else {
        None
    };

    if send_spawn {
        send_spawn_locked(&s);
    }
    if send_pick_map {
        send_pick_map_locked(&s);
    }
    if send_start_match {
        send_start_match_locked(&s);
    }
    drop(s);

    let Some(writer) = dgram else {
        return;
    };

    if send_probe {
        let t1 = client_now_secs();
        if let Ok(payload) = encode_c2s(&ClientToServer::ClockProbe { t1 }) {
            let arr = Uint8Array::from(payload.as_slice());
            let _ = writer.write_with_chunk(&arr);
        }
    }

    if let Some(payload) = drive_payload {
        let arr = Uint8Array::from(payload.as_slice());
        let _ = writer.write_with_chunk(&arr);
    }
}
