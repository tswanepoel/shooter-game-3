//! Game simulation logic crate.
//!
//! Shared ground-truth world data and pure sim rules.
//! World space: 1 unit = 1 metre, Y-up, XZ ground plane.

mod config;
mod fire;
mod self_state;
mod weapons;

pub use config::*;
pub use fire::*;
pub use self_state::*;
pub use weapons::*;

use glam::Vec3;

/// A basic game entity with position and velocity.
#[derive(Debug, Clone)]
pub struct Entity {
    pub position: Vec3,
    pub velocity: Vec3,
}

impl Entity {
    /// Creates a new entity at the given position.
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
        }
    }

    /// Updates the entity's position based on its velocity.
    pub fn update(&mut self, delta_time: f32) {
        self.position += self.velocity * delta_time;
    }
}

/// Basic game state.
#[derive(Debug, Clone)]
pub struct GameState {
    pub entities: Vec<Entity>,
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameState {
    /// Creates a new game state.
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }

    /// Adds an entity to the game state.
    pub fn add_entity(&mut self, entity: Entity) {
        self.entities.push(entity);
    }

    /// Updates all entities in the game state.
    pub fn update(&mut self, delta_time: f32) {
        for entity in &mut self.entities {
            entity.update(delta_time);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_creation() {
        let position = Vec3::new(1.0, 2.0, 3.0);
        let entity = Entity::new(position);
        assert_eq!(entity.position, position);
        assert_eq!(entity.velocity, Vec3::ZERO);
    }

    #[test]
    fn test_entity_update() {
        let mut entity = Entity::new(Vec3::ZERO);
        entity.velocity = Vec3::new(1.0, 1.0, 1.0);
        entity.update(1.0);
        assert_eq!(entity.position, Vec3::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn test_game_state_creation() {
        let state = GameState::new();
        assert!(state.entities.is_empty());
    }

    #[test]
    fn test_game_state_add_entity() {
        let mut state = GameState::new();
        let entity = Entity::new(Vec3::new(1.0, 2.0, 3.0));
        state.add_entity(entity);
        assert_eq!(state.entities.len(), 1);
    }
}
