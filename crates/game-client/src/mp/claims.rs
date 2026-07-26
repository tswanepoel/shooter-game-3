//! Living-phase projectile/hit claims and ammo wire mapping.

use std::cell::RefCell;

use game_net::{encode_c2s, ClientToServer, NetImpactHit, NetProjectileSpawn, NetVec3, PlayerId};
use game_sim::{weapon_def, AmmoKind, ImpactHit, Projectile};
use js_sys::Uint8Array;

use super::phase::MpPhase;
use super::shared::{client_now_secs, Shared};

pub(crate) fn claim_projectiles(shared: &RefCell<Shared>, projectiles: &[Projectile]) {
    if projectiles.is_empty() {
        return;
    }
    let s = shared.borrow();
    if s.phase != MpPhase::Living {
        return;
    }
    let Some(writer) = s.dgram_writer.as_ref() else {
        return;
    };
    let tick = s.clock.estimated_tick(client_now_secs()).unwrap_or(0);
    let spawns: Vec<NetProjectileSpawn> = projectiles
        .iter()
        .map(|p| NetProjectileSpawn {
            id: p.id,
            weapon: p.weapon,
            origin: NetVec3::new(p.origin.x, p.origin.y, p.origin.z),
            velocity: NetVec3::new(p.velocity.x, p.velocity.y, p.velocity.z),
            muzzle_index: p.muzzle_index,
        })
        .collect();
    let Ok(payload) = encode_c2s(&ClientToServer::ProjectileSpawn {
        tick,
        projectiles: spawns,
    }) else {
        return;
    };
    let arr = Uint8Array::from(payload.as_slice());
    let _ = writer.write_with_chunk(&arr);
}

pub(crate) fn claim_hits(shared: &RefCell<Shared>, hits: &[ImpactHit]) {
    if hits.is_empty() {
        return;
    }
    let s = shared.borrow();
    if s.phase != MpPhase::Living {
        return;
    }
    let Some(writer) = s.dgram_writer.as_ref() else {
        return;
    };
    let tick = s.clock.estimated_tick(client_now_secs()).unwrap_or(0);
    for h in hits {
        let Some(ammo) = ammo_kind_to_wire(h.ammo) else {
            continue;
        };
        let hit = NetImpactHit {
            projectile_id: h.projectile_id,
            target: h.target_id,
            ammo,
            speed: h.speed,
            part: h.part.to_wire(),
        };
        let Ok(payload) = encode_c2s(&ClientToServer::ImpactHit { tick, hit }) else {
            continue;
        };
        let arr = Uint8Array::from(payload.as_slice());
        let _ = writer.write_with_chunk(&arr);
    }
}

pub fn ammo_kind_from_wire(ammo: u8) -> Option<AmmoKind> {
    match ammo {
        0 => Some(AmmoKind::LightFoam),
        1 => Some(AmmoKind::ThickFoam),
        2 => Some(AmmoKind::Grenade),
        _ => None,
    }
}

fn ammo_kind_to_wire(ammo: AmmoKind) -> Option<u8> {
    Some(match ammo {
        AmmoKind::LightFoam => 0,
        AmmoKind::ThickFoam => 1,
        AmmoKind::Grenade => 2,
    })
}

pub fn net_spawn_to_projectile(owner: PlayerId, n: &NetProjectileSpawn) -> Option<Projectile> {
    let def = weapon_def(n.weapon)?;
    let origin = glam::Vec3::new(n.origin.x, n.origin.y, n.origin.z);
    let velocity = glam::Vec3::new(n.velocity.x, n.velocity.y, n.velocity.z);
    Some(Projectile {
        id: n.id,
        owner,
        weapon: n.weapon,
        ammo: def.ammo(),
        origin,
        position: origin,
        velocity,
        traveled: 0.0,
        max_range: def.max_range,
        muzzle_index: n.muzzle_index,
    })
}
