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
    #[allow(dead_code)]
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

    pub fn remove(&mut self, id: PlayerId) {
        self.entries.remove(&id);
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
