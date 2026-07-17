//! Integration tests for the game-sim crate

use game_sim::{Entity, GameState};

#[test]
fn test_game_state_update() {
    let mut state = GameState::new();

    // Add some entities
    state.add_entity(Entity::new([0.0, 0.0, 0.0].into()));
    state.add_entity(Entity::new([1.0, 1.0, 1.0].into()));

    // Update the state
    state.update(1.0);

    // Check that entities were updated
    assert_eq!(state.entities.len(), 2);
}
