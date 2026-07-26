use std::collections::HashMap;

use game_net::{DriveView, PlayerId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RemoteKitKey {
    pub character: u8,
    pub primary: Option<u8>,
    pub secondary: Option<u8>,
}

impl RemoteKitKey {
    pub fn from_drive(drive: &DriveView) -> Self {
        Self {
            character: drive.character,
            primary: drive.primary,
            secondary: drive.secondary,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RemoteSample {
    /// Server tick of this drive sample (present / adaptive delay).
    pub tick: u64,
    pub drive: DriveView,
}

pub struct RemoteTable {
    entries: HashMap<PlayerId, Option<RemoteSample>>,
}

impl RemoteTable {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn note_joined(&mut self, id: PlayerId) {
        self.entries.entry(id).or_insert(None);
    }

    pub fn retain(&mut self, mut f: impl FnMut(PlayerId) -> bool) {
        self.entries.retain(|&id, _| f(id));
    }

    pub fn upsert_drive(&mut self, id: PlayerId, tick: u64, drive: DriveView) {
        self.entries.insert(id, Some(RemoteSample { tick, drive }));
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }

    pub fn ids(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.entries.keys().copied()
    }

    pub fn samples(&self) -> impl Iterator<Item = (PlayerId, &RemoteSample)> + '_ {
        self.entries
            .iter()
            .filter_map(|(&id, s)| s.as_ref().map(|sample| (id, sample)))
    }
}

impl Default for RemoteTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_net::DriveView;

    #[test]
    fn upsert_stores_tick() {
        let mut table = RemoteTable::new();
        let drive = DriveView {
            position: game_net::NetVec3::new(0.0, 0.0, 0.0),
            facing: 0.0,
            look_offset_yaw: 0.0,
            look_offset_pitch: 0.0,
            character: b'a',
            primary: None,
            secondary: None,
            active: game_net::NetActiveWeapon::Primary,
            locomotion: game_net::NetLocomotion::Stand,
            walk_phase: 0.0,
            velocity_y: 0.0,
            emote: None,
            emote_age_s: 0.0,
        };
        table.upsert_drive(7, 42, drive);
        let sample = table.samples().next().expect("sample");
        assert_eq!(sample.0, 7);
        assert_eq!(sample.1.tick, 42);
    }
}
