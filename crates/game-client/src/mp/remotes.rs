//! Remote player pose table (filled by snapshots in 024).

use std::collections::HashMap;

use game_net::{NetPlayerPose, PlayerId};

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

    pub fn get(&self, id: PlayerId) -> Option<&NetPlayerPose> {
        self.poses.get(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&PlayerId, &NetPlayerPose)> {
        self.poses.iter()
    }

    pub fn len(&self) -> usize {
        self.poses.len()
    }

    pub fn is_empty(&self) -> bool {
        self.poses.is_empty()
    }
}

impl Default for RemoteTable {
    fn default() -> Self {
        Self::new()
    }
}
