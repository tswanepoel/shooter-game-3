//! Directional postcard multiplayer wire.

use serde::{Deserialize, Serialize};

pub const TICK_HZ: u32 = 180;

pub const TICK_DURATION_SECS: f64 = 1.0 / TICK_HZ as f64;

/// v9: role + character on roster; SetRole / SetCharacter (052).
pub const PROTOCOL_VERSION: u16 = 9;

/// Default room code (051 MVP). Client pre-fills this; server accepts only this value.
pub const DEFAULT_ROOM_CODE: &str = "dev";

/// Max display-name length after trim (051).
pub const DISPLAY_NAME_MAX_CHARS: usize = 24;

/// Default body kit letter (051 / 052 — Kenney character-a).
pub const DEFAULT_CHARACTER: u8 = b'a';

/// Inclusive kit letter range in the cooked character pack (`character-a` … `character-r`).
pub const CHARACTER_FIRST: u8 = b'a';
pub const CHARACTER_LAST: u8 = b'r';

pub type PlayerId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetRole {
    Player,
    Spectator,
}

pub fn is_known_character(id: u8) -> bool {
    (CHARACTER_FIRST..=CHARACTER_LAST).contains(&id)
}

pub fn character_catalog() -> impl Iterator<Item = u8> {
    CHARACTER_FIRST..=CHARACTER_LAST
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NetVec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl NetVec3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetLocomotion {
    Stand,
    Walk,
    Sprint,
    Stopping,
    Air,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetActiveWeapon {
    Primary,
    Secondary,
}

/// Enough fields to rebuild present pose on a peer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriveView {
    pub position: NetVec3,
    pub ocular_yaw: f32,
    pub ocular_pitch: f32,
    pub character: u8,
    pub primary: Option<u8>,
    pub secondary: Option<u8>,
    pub active: NetActiveWeapon,
    pub locomotion: NetLocomotion,
    pub walk_phase: f32,
    pub velocity_y: f32,
    /// Emote wheel slot when active (039); `None` when idle.
    pub emote: Option<u8>,
    /// Seconds since emote commit (039).
    pub emote_age_s: f32,
}

/// One claimed projectile spawn (038). Server relays; peers present motion + FX.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetProjectileSpawn {
    pub id: u64,
    pub weapon: u8,
    pub origin: NetVec3,
    pub velocity: NetVec3,
    /// Kit muzzle index (flash).
    pub muzzle_index: u8,
}

/// Firer-claimed impact. Ammo 0/1/2 = light/thick/grenade.
/// Part 0…5 = head, torso, arm-left, arm-right, leg-left, leg-right.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetImpactHit {
    pub projectile_id: u64,
    pub target: PlayerId,
    pub ammo: u8,
    pub speed: f32,
    pub part: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RosterEntry {
    pub id: PlayerId,
    pub display_name: String,
    pub score: u32,
    pub living: bool,
    pub role: NetRole,
    /// Last committed kit (kept while spectating).
    pub character: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientToServer {
    Hello {
        protocol: u16,
        room_code: String,
        display_name: String,
    },
    ClockProbe {
        t1: f64,
    },
    DriveSample {
        tick: u64,
        drive: DriveView,
    },
    /// Shooter-claimed projectile batch (038).
    ProjectileSpawn {
        tick: u64,
        projectiles: Vec<NetProjectileSpawn>,
    },
    /// Shooter-claimed impact hit (043). VFX projectiles are separate.
    ImpactHit {
        tick: u64,
        hit: NetImpactHit,
    },
    /// Reliable control stream only.
    SetRole {
        role: NetRole,
    },
    /// Reliable control stream only; rejected while living.
    SetCharacter {
        character: u8,
    },
    /// Reliable control stream only.
    Spawn,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerToClient {
    Welcome {
        protocol: u16,
        player_id: PlayerId,
        tick: u64,
        server_time_secs: f64,
    },
    Reject {
        reason: String,
    },
    ClockReply {
        t1: f64,
        t2: f64,
        t3: f64,
        tick: u64,
    },
    /// Local member may enter the map at this pose (051).
    YouSpawned {
        tick: u64,
        position: NetVec3,
        yaw: f32,
    },
    /// Sole membership / score / living / role / kit truth.
    Roster {
        tick: u64,
        entries: Vec<RosterEntry>,
    },
    PeerDrive {
        tick: u64,
        id: PlayerId,
        drive: DriveView,
    },
    /// Relayed peer projectile spawns (038).
    PeerProjectileSpawn {
        tick: u64,
        id: PlayerId,
        projectiles: Vec<NetProjectileSpawn>,
    },
    /// Relayed peer impact hit (043). `id` is the firer.
    PeerImpactHit {
        tick: u64,
        id: PlayerId,
        hit: NetImpactHit,
    },
}

/// Trim, length-cap, and reject empty display names (shared client/server rules).
pub fn normalize_display_name(raw: &str) -> Result<String, &'static str> {
    let trimmed: String = raw.trim().chars().take(DISPLAY_NAME_MAX_CHARS).collect();
    if trimmed.is_empty() {
        return Err("display name empty");
    }
    Ok(trimmed)
}

pub fn display_name_key(name: &str) -> String {
    name.to_lowercase()
}

pub fn encode_c2s(msg: &ClientToServer) -> Result<Vec<u8>, postcard::Error> {
    postcard::to_allocvec(msg)
}

pub fn decode_c2s(buf: &[u8]) -> Result<ClientToServer, postcard::Error> {
    postcard::from_bytes(buf)
}

pub fn encode_s2c(msg: &ServerToClient) -> Result<Vec<u8>, postcard::Error> {
    postcard::to_allocvec(msg)
}

pub fn decode_s2c(buf: &[u8]) -> Result<ServerToClient, postcard::Error> {
    postcard::from_bytes(buf)
}

/// `u32` LE length + postcard body (reliable stream after Welcome).
pub fn encode_s2c_frame(msg: &ServerToClient) -> Result<Vec<u8>, postcard::Error> {
    let body = encode_s2c(msg)?;
    let mut out = Vec::with_capacity(4 + body.len());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

/// Decode complete frames; leaves a partial tail in `buf`.
pub fn drain_s2c_frames(buf: &mut Vec<u8>) -> Vec<ServerToClient> {
    let mut out = Vec::new();
    loop {
        if buf.len() < 4 {
            break;
        }
        let len = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
        if buf.len() < 4 + len {
            break;
        }
        let body: Vec<u8> = buf.drain(..4 + len).skip(4).collect();
        if let Ok(msg) = decode_s2c(&body) {
            out.push(msg);
        }
    }
    out
}

/// `server ≈ local + offset` → floor to tick.
pub fn estimated_tick(local_secs: f64, offset_secs: f64) -> u64 {
    let server = local_secs + offset_secs;
    if server <= 0.0 {
        0
    } else {
        (server * TICK_HZ as f64).floor() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_drive() -> DriveView {
        DriveView {
            position: NetVec3::new(1.0, 0.0, 2.0),
            ocular_yaw: 0.5,
            ocular_pitch: -0.1,
            character: b'a',
            primary: Some(b'p'),
            secondary: Some(b'b'),
            active: NetActiveWeapon::Primary,
            locomotion: NetLocomotion::Walk,
            walk_phase: 0.25,
            velocity_y: 0.0,
            emote: Some(2),
            emote_age_s: 0.12,
        }
    }

    fn sample_roster_entry() -> RosterEntry {
        RosterEntry {
            id: 1,
            display_name: "Ace".into(),
            score: 2,
            living: true,
            role: NetRole::Player,
            character: b'a',
        }
    }

    #[test]
    fn hello_roundtrip() {
        let msg = ClientToServer::Hello {
            protocol: PROTOCOL_VERSION,
            room_code: DEFAULT_ROOM_CODE.into(),
            display_name: "Ace".into(),
        };
        let b = encode_c2s(&msg).unwrap();
        assert_eq!(decode_c2s(&b).unwrap(), msg);
    }

    #[test]
    fn welcome_roundtrip() {
        let msg = ServerToClient::Welcome {
            protocol: PROTOCOL_VERSION,
            player_id: 7,
            tick: 42,
            server_time_secs: 1.5,
        };
        let b = encode_s2c(&msg).unwrap();
        assert_eq!(decode_s2c(&b).unwrap(), msg);
    }

    #[test]
    fn reject_roundtrip() {
        let msg = ServerToClient::Reject {
            reason: "protocol mismatch".into(),
        };
        let b = encode_s2c(&msg).unwrap();
        assert_eq!(decode_s2c(&b).unwrap(), msg);
    }

    #[test]
    fn clock_probe_reply_roundtrip() {
        let probe = ClientToServer::ClockProbe { t1: 0.25 };
        let b = encode_c2s(&probe).unwrap();
        assert_eq!(decode_c2s(&b).unwrap(), probe);

        let reply = ServerToClient::ClockReply {
            t1: 0.1,
            t2: 0.2,
            t3: 0.21,
            tick: 7,
        };
        let b = encode_s2c(&reply).unwrap();
        assert_eq!(decode_s2c(&b).unwrap(), reply);
    }

    #[test]
    fn drive_sample_roundtrip() {
        let msg = ClientToServer::DriveSample {
            tick: 99,
            drive: sample_drive(),
        };
        let b = encode_c2s(&msg).unwrap();
        assert_eq!(decode_c2s(&b).unwrap(), msg);
    }

    #[test]
    fn peer_drive_roundtrip() {
        let msg = ServerToClient::PeerDrive {
            tick: 100,
            id: 3,
            drive: sample_drive(),
        };
        let b = encode_s2c(&msg).unwrap();
        assert_eq!(decode_s2c(&b).unwrap(), msg);
    }

    #[test]
    fn spawn_role_character_and_roster_roundtrip() {
        let spawn = ClientToServer::Spawn;
        let b = encode_c2s(&spawn).unwrap();
        assert_eq!(decode_c2s(&b).unwrap(), spawn);

        let set_role = ClientToServer::SetRole {
            role: NetRole::Spectator,
        };
        let b = encode_c2s(&set_role).unwrap();
        assert_eq!(decode_c2s(&b).unwrap(), set_role);

        let set_ch = ClientToServer::SetCharacter { character: b'c' };
        let b = encode_c2s(&set_ch).unwrap();
        assert_eq!(decode_c2s(&b).unwrap(), set_ch);

        let you = ServerToClient::YouSpawned {
            tick: 1,
            position: NetVec3::new(2.0, 0.0, -1.0),
            yaw: 0.5,
        };
        let b = encode_s2c(&you).unwrap();
        assert_eq!(decode_s2c(&b).unwrap(), you);

        let roster = ServerToClient::Roster {
            tick: 3,
            entries: vec![sample_roster_entry()],
        };
        let b = encode_s2c(&roster).unwrap();
        assert_eq!(decode_s2c(&b).unwrap(), roster);
    }

    #[test]
    fn known_character_letters() {
        assert!(is_known_character(b'a'));
        assert!(is_known_character(b'r'));
        assert!(!is_known_character(b's'));
        assert!(!is_known_character(b'A'));
        assert_eq!(character_catalog().count(), 18);
    }

    #[test]
    fn normalize_display_name_rules() {
        assert_eq!(normalize_display_name("  Ace  ").unwrap(), "Ace");
        assert!(normalize_display_name("   ").is_err());
        let long = "x".repeat(40);
        assert_eq!(normalize_display_name(&long).unwrap().chars().count(), 24);
        assert_eq!(display_name_key("Ace"), display_name_key("ace"));
    }

    #[test]
    fn projectile_spawn_roundtrip() {
        let spawns = vec![NetProjectileSpawn {
            id: 9,
            weapon: b'p',
            origin: NetVec3::new(0.0, 1.4, 0.5),
            velocity: NetVec3::new(0.0, 0.0, 400.0),
            muzzle_index: 0,
        }];
        let c2s = ClientToServer::ProjectileSpawn {
            tick: 12,
            projectiles: spawns.clone(),
        };
        let b = encode_c2s(&c2s).unwrap();
        assert_eq!(decode_c2s(&b).unwrap(), c2s);

        let s2c = ServerToClient::PeerProjectileSpawn {
            tick: 12,
            id: 3,
            projectiles: spawns,
        };
        let b = encode_s2c(&s2c).unwrap();
        assert_eq!(decode_s2c(&b).unwrap(), s2c);
    }

    #[test]
    fn impact_hit_roundtrip() {
        let hit = NetImpactHit {
            projectile_id: 42,
            target: 2,
            ammo: 0,
            speed: 380.0,
            part: 0,
        };
        let c2s = ClientToServer::ImpactHit {
            tick: 15,
            hit: hit.clone(),
        };
        let b = encode_c2s(&c2s).unwrap();
        assert_eq!(decode_c2s(&b).unwrap(), c2s);

        let s2c = ServerToClient::PeerImpactHit {
            tick: 15,
            id: 1,
            hit,
        };
        let b = encode_s2c(&s2c).unwrap();
        assert_eq!(decode_s2c(&b).unwrap(), s2c);
    }

    #[test]
    fn estimated_tick_basic() {
        assert_eq!(estimated_tick(1.0, 0.0), 180);
    }

    #[test]
    fn directional_roots_do_not_cross_decode() {
        let c2s = encode_c2s(&ClientToServer::Hello {
            protocol: 1,
            room_code: DEFAULT_ROOM_CODE.into(),
            display_name: "x".into(),
        })
        .unwrap();
        assert!(decode_s2c(&c2s).is_err());
        let s2c = encode_s2c(&ServerToClient::Reject { reason: "x".into() }).unwrap();
        assert!(decode_c2s(&s2c).is_err());
    }

    #[test]
    fn framed_s2c_roundtrip_batch() {
        let a = ServerToClient::YouSpawned {
            tick: 1,
            position: NetVec3::new(0.0, 0.0, 0.0),
            yaw: 0.0,
        };
        let b = ServerToClient::Roster {
            tick: 3,
            entries: vec![RosterEntry {
                id: 2,
                display_name: "A".into(),
                score: 0,
                living: false,
                role: NetRole::Spectator,
                character: b'b',
            }],
        };
        let mut buf = encode_s2c_frame(&a).unwrap();
        buf.extend(encode_s2c_frame(&b).unwrap());
        // partial tail
        buf.extend_from_slice(&[3, 0, 0, 0, 9]);
        let msgs = drain_s2c_frames(&mut buf);
        assert_eq!(msgs, vec![a, b]);
        assert_eq!(buf, vec![3, 0, 0, 0, 9]);
    }
}
