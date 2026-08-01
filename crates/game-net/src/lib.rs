//! Directional postcard multiplayer wire.

use serde::{Deserialize, Serialize};

pub const TICK_HZ: u32 = 180;

pub const TICK_DURATION_SECS: f64 = 1.0 / TICK_HZ as f64;

/// Alpha wire; bumped when variants change (no distributed compat promise).
pub const PROTOCOL_VERSION: u16 = 18;

/// Largest accepted body of a length-prefixed reliable-stream frame.
pub const MAX_FRAME_BYTES: usize = 64 * 1024;

/// Max display-name length after trim (051).
pub const DISPLAY_NAME_MAX_CHARS: usize = 24;

/// Default body kit letter (051 / 052 — Kenney character-a).
pub const DEFAULT_CHARACTER: u8 = b'a';

/// Default map letter (064 — map-a).
pub const DEFAULT_MAP: u8 = b'a';

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

pub fn is_known_map(id: u8) -> bool {
    id == DEFAULT_MAP
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

/// Enough fields to rebuild present pose on a peer (facing + look offset + loco).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriveView {
    pub position: NetVec3,
    pub facing: f32,
    pub look_offset_yaw: f32,
    pub look_offset_pitch: f32,
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

/// Victim death dump of active-kind rounds into a room ammo drop (059).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetAmmoDump {
    pub ammo: u8,
    pub rounds: u16,
    pub position: NetVec3,
}

/// Pose snapshot for a corpse present (059).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetCorpseSpawn {
    pub corpse_id: u64,
    pub victim: PlayerId,
    pub character: u8,
    pub position: NetVec3,
    pub facing: f32,
}

/// Invisible ammo drop announce (059).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetAmmoDropSpawn {
    pub drop_id: u64,
    pub corpse_id: u64,
    pub position: NetVec3,
    pub ammo: u8,
    pub rounds: u16,
}

/// Victim / displace dump of a floor blaster (067).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetBlasterDump {
    pub letter: u8,
    pub mag: u16,
    pub position: NetVec3,
}

/// Visible blaster drop announce (067). `corpse_id` is 0 when not death-linked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetBlasterDropSpawn {
    pub drop_id: u64,
    pub corpse_id: u64,
    pub position: NetVec3,
    pub letter: u8,
    pub mag: u16,
}

/// Room match snapshot on roster (064).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchView {
    pub map: Option<u8>,
    pub started: bool,
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
    pub room_leader: bool,
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
    /// Victim dump of death ammo into a room drop (059).
    AmmoDump {
        tick: u64,
        dump: NetAmmoDump,
    },
    /// Living walk-over claim on a drop (059). `room` is reserve free slots for that kind.
    LootClaim {
        tick: u64,
        drop_id: u64,
        position: NetVec3,
        room: u16,
    },
    /// Victim or displace dump of a floor blaster (067).
    BlasterDump {
        tick: u64,
        dump: NetBlasterDump,
    },
    /// Living F claim on a blaster drop (067).
    BlasterClaim {
        tick: u64,
        drop_id: u64,
        position: NetVec3,
    },
    /// Reliable control stream only.
    SetRole {
        role: NetRole,
    },
    /// Reliable control stream only; rejected while living.
    SetCharacter {
        character: u8,
    },
    /// Reliable control stream only. Loadout applies on accept (053).
    Spawn {
        primary: Option<u8>,
        secondary: Option<u8>,
        active: NetActiveWeapon,
    },
    /// Room leader picks map before match start (064).
    PickMap {
        map: u8,
    },
    /// Room leader starts the match after map pick (064).
    StartMatch,
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
        facing: f32,
    },
    /// Sole membership / score / living / role / kit truth.
    Roster {
        tick: u64,
        match_view: MatchView,
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
    /// Relayed peer impact hit (043 / 080). `id` is the firer.
    /// Accepted claims only; never the sole death authority (080).
    PeerImpactHit {
        tick: u64,
        id: PlayerId,
        hit: NetImpactHit,
    },
    /// Server-owned death (080). Reliable stream. Sole death authority for clients.
    DeathAnnounce {
        tick: u64,
        victim: PlayerId,
        killer: PlayerId,
    },
    /// Room corpse present after accepted death (059).
    CorpseSpawn {
        tick: u64,
        corpse: NetCorpseSpawn,
    },
    /// Corpse lifetime ended (059). Linked drop ends with it.
    CorpseEnd {
        tick: u64,
        corpse_id: u64,
    },
    /// Room ammo drop after a victim dump (059).
    AmmoDropSpawn {
        tick: u64,
        drop: NetAmmoDropSpawn,
    },
    /// Elected loot grant into winner reserve (059).
    LootGrant {
        tick: u64,
        drop_id: u64,
        player_id: PlayerId,
        ammo: u8,
        rounds: u16,
    },
    /// Drop ended by timer or empty without a grant frame (059).
    AmmoDropEnd {
        tick: u64,
        drop_id: u64,
    },
    /// Room blaster drop after a dump (067).
    BlasterDropSpawn {
        tick: u64,
        drop: NetBlasterDropSpawn,
    },
    /// Elected blaster loot grant (067).
    BlasterGrant {
        tick: u64,
        drop_id: u64,
        player_id: PlayerId,
        letter: u8,
        mag: u16,
    },
    /// Blaster drop ended by timer or grant (067).
    BlasterDropEnd {
        tick: u64,
        drop_id: u64,
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

/// A peer announced a frame longer than [`MAX_FRAME_BYTES`]. The stream is
/// desynced past this point, so the caller must drop the connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameTooLarge(pub usize);

impl std::fmt::Display for FrameTooLarge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "frame of {} bytes exceeds {MAX_FRAME_BYTES}", self.0)
    }
}

impl std::error::Error for FrameTooLarge {}

fn encode_frame(body: Vec<u8>) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + body.len());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

/// QUIC streams carry bytes, not messages: one `read` may return several frames
/// or half of one. Pop the first complete frame and leave the tail in `buf`.
///
/// A frame whose body fails to decode is skipped rather than fatal — the length
/// prefix lets us resync on the next frame.
fn take_frame<T>(
    buf: &mut Vec<u8>,
    decode: fn(&[u8]) -> Result<T, postcard::Error>,
) -> Result<Option<T>, FrameTooLarge> {
    loop {
        if buf.len() < 4 {
            return Ok(None);
        }
        let len = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
        if len > MAX_FRAME_BYTES {
            return Err(FrameTooLarge(len));
        }
        if buf.len() < 4 + len {
            return Ok(None);
        }
        let body: Vec<u8> = buf.drain(..4 + len).skip(4).collect();
        if let Ok(msg) = decode(&body) {
            return Ok(Some(msg));
        }
    }
}

fn drain_frames<T>(
    buf: &mut Vec<u8>,
    decode: fn(&[u8]) -> Result<T, postcard::Error>,
) -> Result<Vec<T>, FrameTooLarge> {
    let mut out = Vec::new();
    while let Some(msg) = take_frame(buf, decode)? {
        out.push(msg);
    }
    Ok(out)
}

/// `u32` LE length + postcard body (reliable stream, including the handshake).
pub fn encode_c2s_frame(msg: &ClientToServer) -> Result<Vec<u8>, postcard::Error> {
    Ok(encode_frame(encode_c2s(msg)?))
}

/// See [`drain_frames`].
pub fn drain_c2s_frames(buf: &mut Vec<u8>) -> Result<Vec<ClientToServer>, FrameTooLarge> {
    drain_frames(buf, decode_c2s)
}

/// Pop one frame, for the handshake, where the bytes behind it belong to the
/// reader that takes over afterwards. See [`take_frame`].
pub fn take_c2s_frame(buf: &mut Vec<u8>) -> Result<Option<ClientToServer>, FrameTooLarge> {
    take_frame(buf, decode_c2s)
}

/// `u32` LE length + postcard body (reliable stream, including the handshake).
pub fn encode_s2c_frame(msg: &ServerToClient) -> Result<Vec<u8>, postcard::Error> {
    Ok(encode_frame(encode_s2c(msg)?))
}

/// See [`drain_frames`].
pub fn drain_s2c_frames(buf: &mut Vec<u8>) -> Result<Vec<ServerToClient>, FrameTooLarge> {
    drain_frames(buf, decode_s2c)
}

/// Pop one frame, for the handshake, where the bytes behind it belong to the
/// reader that takes over afterwards. See [`take_frame`].
pub fn take_s2c_frame(buf: &mut Vec<u8>) -> Result<Option<ServerToClient>, FrameTooLarge> {
    take_frame(buf, decode_s2c)
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
            facing: 0.5,
            look_offset_yaw: 0.0,
            look_offset_pitch: -0.1,
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
            room_leader: true,
        }
    }

    #[test]
    fn hello_roundtrip() {
        let msg = ClientToServer::Hello {
            protocol: PROTOCOL_VERSION,
            room_code: "dev".into(),
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
        let spawn = ClientToServer::Spawn {
            primary: Some(b'p'),
            secondary: None,
            active: NetActiveWeapon::Primary,
        };
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
            facing: 0.5,
        };
        let b = encode_s2c(&you).unwrap();
        assert_eq!(decode_s2c(&b).unwrap(), you);

        let roster = ServerToClient::Roster {
            tick: 3,
            match_view: MatchView {
                map: Some(DEFAULT_MAP),
                started: true,
            },
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

        let death = ServerToClient::DeathAnnounce {
            tick: 16,
            victim: 2,
            killer: 1,
        };
        let b = encode_s2c(&death).unwrap();
        assert_eq!(decode_s2c(&b).unwrap(), death);
        let framed = encode_s2c_frame(&death).unwrap();
        let mut buf = framed;
        assert_eq!(drain_s2c_frames(&mut buf).unwrap(), vec![death]);
    }

    #[test]
    fn loot_wire_roundtrip() {
        let dump = NetAmmoDump {
            ammo: 0,
            rounds: 12,
            position: NetVec3::new(1.0, 0.0, 2.0),
        };
        let c2s = ClientToServer::AmmoDump {
            tick: 9,
            dump: dump.clone(),
        };
        assert_eq!(decode_c2s(&encode_c2s(&c2s).unwrap()).unwrap(), c2s);

        let claim = ClientToServer::LootClaim {
            tick: 10,
            drop_id: 7,
            position: NetVec3::new(1.0, 0.0, 2.0),
            room: 12,
        };
        assert_eq!(decode_c2s(&encode_c2s(&claim).unwrap()).unwrap(), claim);

        let corpse = ServerToClient::CorpseSpawn {
            tick: 11,
            corpse: NetCorpseSpawn {
                corpse_id: 3,
                victim: 2,
                character: b'a',
                position: NetVec3::new(1.0, 0.0, 2.0),
                facing: 0.5,
            },
        };
        assert_eq!(decode_s2c(&encode_s2c(&corpse).unwrap()).unwrap(), corpse);

        let drop = ServerToClient::AmmoDropSpawn {
            tick: 12,
            drop: NetAmmoDropSpawn {
                drop_id: 7,
                corpse_id: 3,
                position: NetVec3::new(1.0, 0.0, 2.0),
                ammo: 0,
                rounds: 12,
            },
        };
        assert_eq!(decode_s2c(&encode_s2c(&drop).unwrap()).unwrap(), drop);

        let grant = ServerToClient::LootGrant {
            tick: 13,
            drop_id: 7,
            player_id: 1,
            ammo: 0,
            rounds: 5,
        };
        assert_eq!(decode_s2c(&encode_s2c(&grant).unwrap()).unwrap(), grant);

        let end = ServerToClient::AmmoDropEnd {
            tick: 14,
            drop_id: 7,
        };
        assert_eq!(decode_s2c(&encode_s2c(&end).unwrap()).unwrap(), end);

        let corpse_end = ServerToClient::CorpseEnd {
            tick: 15,
            corpse_id: 3,
        };
        assert_eq!(
            decode_s2c(&encode_s2c(&corpse_end).unwrap()).unwrap(),
            corpse_end
        );

        let bdump = ClientToServer::BlasterDump {
            tick: 16,
            dump: NetBlasterDump {
                letter: b'b',
                mag: 4,
                position: NetVec3::new(1.0, 0.0, 2.0),
            },
        };
        assert_eq!(decode_c2s(&encode_c2s(&bdump).unwrap()).unwrap(), bdump);

        let bclaim = ClientToServer::BlasterClaim {
            tick: 17,
            drop_id: 8,
            position: NetVec3::new(1.0, 0.0, 2.0),
        };
        assert_eq!(decode_c2s(&encode_c2s(&bclaim).unwrap()).unwrap(), bclaim);

        let bspawn = ServerToClient::BlasterDropSpawn {
            tick: 18,
            drop: NetBlasterDropSpawn {
                drop_id: 8,
                corpse_id: 3,
                position: NetVec3::new(1.0, 0.0, 2.0),
                letter: b'b',
                mag: 4,
            },
        };
        assert_eq!(decode_s2c(&encode_s2c(&bspawn).unwrap()).unwrap(), bspawn);

        let bgrant = ServerToClient::BlasterGrant {
            tick: 19,
            drop_id: 8,
            player_id: 1,
            letter: b'b',
            mag: 4,
        };
        assert_eq!(decode_s2c(&encode_s2c(&bgrant).unwrap()).unwrap(), bgrant);

        let bend = ServerToClient::BlasterDropEnd {
            tick: 20,
            drop_id: 8,
        };
        assert_eq!(decode_s2c(&encode_s2c(&bend).unwrap()).unwrap(), bend);
    }

    #[test]
    fn estimated_tick_basic() {
        assert_eq!(estimated_tick(1.0, 0.0), 180);
    }

    #[test]
    fn directional_roots_do_not_cross_decode() {
        let c2s = encode_c2s(&ClientToServer::Hello {
            protocol: 1,
            room_code: "dev".into(),
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
            facing: 0.0,
        };
        let b = ServerToClient::Roster {
            tick: 3,
            match_view: MatchView {
                map: None,
                started: false,
            },
            entries: vec![RosterEntry {
                id: 2,
                display_name: "A".into(),
                score: 0,
                living: false,
                role: NetRole::Spectator,
                character: b'b',
                room_leader: false,
            }],
        };
        let mut buf = encode_s2c_frame(&a).unwrap();
        buf.extend(encode_s2c_frame(&b).unwrap());
        // partial tail
        buf.extend_from_slice(&[3, 0, 0, 0, 9]);
        let msgs = drain_s2c_frames(&mut buf).unwrap();
        assert_eq!(msgs, vec![a, b]);
        assert_eq!(buf, vec![3, 0, 0, 0, 9]);
    }

    /// Bare postcard silently keeps the first message and discards the rest, so
    /// coalesced control writes used to vanish without an error.
    #[test]
    fn unframed_c2s_pair_loses_the_tail() {
        let a = ClientToServer::SetCharacter { character: b'c' };
        let mut buf = encode_c2s(&a).unwrap();
        buf.extend(encode_c2s(&ClientToServer::StartMatch).unwrap());
        assert_eq!(decode_c2s(&buf).unwrap(), a);
    }

    #[test]
    fn framed_c2s_survives_coalesced_writes() {
        let msgs = [
            ClientToServer::SetRole {
                role: NetRole::Spectator,
            },
            ClientToServer::SetCharacter { character: b'c' },
            ClientToServer::PickMap { map: DEFAULT_MAP },
            ClientToServer::StartMatch,
        ];
        let mut buf = Vec::new();
        for m in &msgs {
            buf.extend(encode_c2s_frame(m).unwrap());
        }
        assert_eq!(drain_c2s_frames(&mut buf).unwrap(), msgs);
        assert!(buf.is_empty());
    }

    #[test]
    fn framed_c2s_survives_a_write_split_across_reads() {
        let spawn = ClientToServer::Spawn {
            primary: Some(b'p'),
            secondary: Some(b'b'),
            active: NetActiveWeapon::Primary,
        };
        let wire = encode_c2s_frame(&spawn).unwrap();
        // Every split point must recover the message once both halves arrive.
        for cut in 0..wire.len() {
            let mut buf = wire[..cut].to_vec();
            assert!(drain_c2s_frames(&mut buf).unwrap().is_empty());
            buf.extend_from_slice(&wire[cut..]);
            assert_eq!(drain_c2s_frames(&mut buf).unwrap(), vec![spawn.clone()]);
            assert!(buf.is_empty());
        }
    }

    #[test]
    fn oversized_frame_is_rejected() {
        let len = MAX_FRAME_BYTES + 1;
        let mut buf = (len as u32).to_le_bytes().to_vec();
        buf.extend_from_slice(&[0u8; 8]);
        assert_eq!(drain_c2s_frames(&mut buf), Err(FrameTooLarge(len)));
        assert_eq!(drain_s2c_frames(&mut buf), Err(FrameTooLarge(len)));
    }

    /// Whatever the peer coalesced behind the handshake must be left for the
    /// reader that takes over, not eaten with it.
    #[test]
    fn take_frame_leaves_the_rest_for_the_next_reader() {
        let hello = ClientToServer::Hello {
            protocol: PROTOCOL_VERSION,
            room_code: "dev".into(),
            display_name: "Ace".into(),
        };
        let follow = ClientToServer::SetCharacter { character: b'c' };
        let mut buf = encode_c2s_frame(&hello).unwrap();
        buf.extend(encode_c2s_frame(&follow).unwrap());

        assert_eq!(take_c2s_frame(&mut buf).unwrap(), Some(hello));
        assert_eq!(drain_c2s_frames(&mut buf).unwrap(), vec![follow]);
    }

    /// An undecodable body must not desync the stream: the next frame still lands.
    #[test]
    fn undecodable_frame_is_skipped() {
        let mut buf = encode_frame(vec![0xff, 0xff, 0xff]);
        let good = ClientToServer::StartMatch;
        buf.extend(encode_c2s_frame(&good).unwrap());
        assert_eq!(drain_c2s_frames(&mut buf).unwrap(), vec![good]);
    }
}
