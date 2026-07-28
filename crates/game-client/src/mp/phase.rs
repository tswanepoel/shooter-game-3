//! Client product phase and camera intent (host-testable).

use game_sim::ActiveWeapon;

/// Client product phase (UI/play gates). Server role/living is separate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpPhase {
    Lobby,
    Connecting,
    Role,
    Character,
    /// Loadout + Spawn bench (053). First entry and post-death.
    Ready,
    Spectating,
    Living,
}

impl MpPhase {
    pub fn in_room(self) -> bool {
        matches!(
            self,
            Self::Role | Self::Character | Self::Ready | Self::Spectating | Self::Living
        )
    }

    pub fn blocks_play(self) -> bool {
        !matches!(self, Self::Living)
    }

    /// Product Gate / Panel chrome: soft pointer + UI keys while the session is active (061).
    pub fn forces_free_cursor(self) -> bool {
        self.surface_kind()
            .is_some_and(ProductSurfaceKind::arms_soft_pointer)
    }

    pub fn surface_kind(self) -> Option<ProductSurfaceKind> {
        ProductSurfaceKind::from_phase(self)
    }

    pub fn is_spectating(self) -> bool {
        self == Self::Spectating
    }

    pub fn can_go(self, to: Self) -> bool {
        use MpPhase::*;
        matches!(
            (self, to),
            (Lobby, Connecting)
                | (Connecting, Role)
                | (Connecting, Lobby)
                | (Role, Character)
                | (Role, Spectating)
                | (Role, Lobby)
                | (Character, Ready)
                | (Character, Role)
                | (Character, Spectating)
                | (Character, Lobby)
                | (Ready, Living)
                | (Ready, Spectating)
                | (Ready, Role)
                | (Ready, Lobby)
                | (Spectating, Character)
                | (Spectating, Role)
                | (Spectating, Lobby)
                | (Living, Ready)
                | (Living, Spectating)
                | (Living, Lobby)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductSurfaceKind {
    Gate,
    Panel,
}

impl ProductSurfaceKind {
    pub fn from_phase(phase: MpPhase) -> Option<Self> {
        match phase {
            MpPhase::Lobby | MpPhase::Connecting | MpPhase::Role | MpPhase::Character => {
                Some(Self::Gate)
            }
            MpPhase::Ready => Some(Self::Panel),
            MpPhase::Spectating | MpPhase::Living => None,
        }
    }

    pub fn arms_soft_pointer(self) -> bool {
        matches!(self, Self::Gate | Self::Panel)
    }
}

/// Per-frame camera intent (product phase + optional debug F8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CamIntent {
    Overview,
    ProductFly,
    DebugFly,
    Mounted,
}

impl CamIntent {
    pub fn derive(phase: MpPhase, debug_fly_wanted: bool) -> Self {
        match phase {
            MpPhase::Spectating => Self::ProductFly,
            MpPhase::Living if debug_fly_wanted => Self::DebugFly,
            MpPhase::Living => Self::Mounted,
            _ => Self::Overview,
        }
    }

    pub fn is_fly(self) -> bool {
        matches!(self, Self::ProductFly | Self::DebugFly)
    }
}

#[derive(Debug, Clone)]
pub struct PendingSpawn {
    pub position: glam::Vec3,
    pub facing: f32,
    pub primary: Option<u8>,
    pub secondary: Option<u8>,
    pub active: ActiveWeapon,
}

/// Staged bench loadout (empty until player chooses; no defaults).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StagedLoadout {
    pub primary: Option<u8>,
    pub secondary: Option<u8>,
    pub active: ActiveWeapon,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_graph_core_paths() {
        assert!(MpPhase::Lobby.can_go(MpPhase::Connecting));
        assert!(MpPhase::Connecting.can_go(MpPhase::Role));
        assert!(MpPhase::Role.can_go(MpPhase::Character));
        assert!(MpPhase::Role.can_go(MpPhase::Spectating));
        assert!(MpPhase::Character.can_go(MpPhase::Ready));
        assert!(MpPhase::Ready.can_go(MpPhase::Living));
        assert!(MpPhase::Spectating.can_go(MpPhase::Character));
        assert!(MpPhase::Living.can_go(MpPhase::Spectating));
        assert!(MpPhase::Living.can_go(MpPhase::Ready));
        assert!(!MpPhase::Lobby.can_go(MpPhase::Living));
        assert!(!MpPhase::Spectating.can_go(MpPhase::Living));
        assert!(!MpPhase::Ready.can_go(MpPhase::Character));
    }

    #[test]
    fn surface_kind_maps_phases() {
        assert_eq!(
            MpPhase::Lobby.surface_kind(),
            Some(ProductSurfaceKind::Gate)
        );
        assert_eq!(
            MpPhase::Ready.surface_kind(),
            Some(ProductSurfaceKind::Panel)
        );
        assert!(MpPhase::Living.surface_kind().is_none());
        assert!(ProductSurfaceKind::Gate.arms_soft_pointer());
        assert!(!MpPhase::Spectating.forces_free_cursor());
    }

    #[test]
    fn cam_intent_derives() {
        assert_eq!(CamIntent::derive(MpPhase::Role, false), CamIntent::Overview);
        assert_eq!(
            CamIntent::derive(MpPhase::Spectating, true),
            CamIntent::ProductFly
        );
        assert_eq!(
            CamIntent::derive(MpPhase::Living, false),
            CamIntent::Mounted
        );
        assert_eq!(
            CamIntent::derive(MpPhase::Living, true),
            CamIntent::DebugFly
        );
    }
}
