//! Remote player pose table (snapshots) and kit keys for present GPUs.

use std::collections::HashMap;

use game_net::{NetPlayerPose, PlayerId};

/// Character + loadout letters that decide which meshes a remote body needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RemoteKitKey {
    pub character: u8,
    pub primary: Option<u8>,
    pub secondary: Option<u8>,
}

impl RemoteKitKey {
    pub fn from_pose(pose: &NetPlayerPose) -> Self {
        Self {
            character: pose.character,
            primary: pose.primary,
            secondary: pose.secondary,
        }
    }
}

pub struct RemoteTable {
    poses: HashMap<PlayerId, NetPlayerPose>,
}

impl RemoteTable {
    pub fn new() -> Self {
        Self {
            poses: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.poses.clear();
    }

    pub fn insert(&mut self, pose: NetPlayerPose) {
        self.poses.insert(pose.id, pose);
    }

    pub fn remove(&mut self, id: PlayerId) {
        self.poses.remove(&id);
    }

    pub fn count(&self) -> usize {
        self.poses.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &NetPlayerPose> {
        self.poses.values()
    }

    pub fn ids(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.poses.keys().copied()
    }
}

impl Default for RemoteTable {
    fn default() -> Self {
        Self::new()
    }
}
