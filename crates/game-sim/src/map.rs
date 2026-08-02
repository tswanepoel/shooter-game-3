//! Map solids: boxes (optional yaw) and ramps (066).
//!
//! Support height and blocking for figure soles. Solids sit on the floor
//! (no floating platforms). Infinite ground is y = 0.

use glam::Vec3;

/// Max auto step onto a higher support without jumping.
pub const STEP_UP_M: f32 = 0.4;
/// Max snap-down while remaining grounded; larger drops enter air.
pub const STEP_DOWN_M: f32 = 0.4;
/// XZ radius for wall overlap tests.
pub const FIGURE_RADIUS_M: f32 = 0.3;

/// Box solid. Centre + half-extents (metres, Y-up).
///
/// `yaw = 0` is axis-aligned on XZ. Non-zero yaw orients the XZ footprint
/// (same local frame as [`MapRamp`]) so angled props keep a tight collide.
#[derive(Clone, Copy, Debug)]
pub struct MapBox {
    pub center: Vec3,
    pub half: Vec3,
    pub yaw: f32,
}

impl MapBox {
    pub fn min_y(self) -> f32 {
        self.center.y - self.half.y
    }

    pub fn max_y(self) -> f32 {
        self.center.y + self.half.y
    }

    fn local_xz(self, x: f32, z: f32) -> (f32, f32) {
        let dx = x - self.center.x;
        let dz = z - self.center.z;
        let (s, c) = self.yaw.sin_cos();
        // Inverse of kit/world_from_local: (c·lx + s·lz, −s·lx + c·lz).
        (c * dx - s * dz, s * dx + c * dz)
    }

    fn contains_xz(self, x: f32, z: f32) -> bool {
        let (lx, lz) = self.local_xz(x, z);
        lx.abs() <= self.half.x && lz.abs() <= self.half.z
    }

    fn hits_circle_xz(self, x: f32, z: f32, radius: f32) -> bool {
        let (lx, lz) = self.local_xz(x, z);
        let dx = lx.abs() - self.half.x;
        let dz = lz.abs() - self.half.z;
        let cx = dx.max(0.0);
        let cz = dz.max(0.0);
        cx * cx + cz * cz <= radius * radius
    }

    /// World-axis bounds of the yawed footprint (for AA consumers e.g. foot patches).
    pub fn world_aabb_xz(self) -> (f32, f32, f32, f32) {
        let (s, c) = self.yaw.sin_cos();
        let hx = self.half.x;
        let hz = self.half.z;
        // Same world_from_local as kit roots: (c·lx + s·lz, −s·lx + c·lz).
        let corners = [
            (c * -hx + s * -hz, -s * -hx + c * -hz),
            (c * -hx + s * hz, -s * -hx + c * hz),
            (c * hx + s * -hz, -s * hx + c * -hz),
            (c * hx + s * hz, -s * hx + c * hz),
        ];
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_z = f32::INFINITY;
        let mut max_z = f32::NEG_INFINITY;
        for (x, z) in corners {
            min_x = min_x.min(self.center.x + x);
            max_x = max_x.max(self.center.x + x);
            min_z = min_z.min(self.center.z + z);
            max_z = max_z.max(self.center.z + z);
        }
        (min_x, max_x, min_z, max_z)
    }

    /// Slab-based segment test. Returns true when the segment `from`→`to`
    /// passes through this box (endpoints inside also count).
    pub fn intersects_segment(self, from: Vec3, to: Vec3) -> bool {
        let (s, c) = self.yaw.sin_cos();
        let to_local = |p: Vec3| {
            let dx = p.x - self.center.x;
            let dy = p.y - self.center.y;
            let dz = p.z - self.center.z;
            Vec3::new(c * dx - s * dz, dy, s * dx + c * dz)
        };
        let from = to_local(from);
        let to = to_local(to);
        let min = -self.half;
        let max = self.half;
        let d = to - from;
        let mut t_min = 0.0_f32;
        let mut t_max = 1.0_f32;
        for axis in 0..3 {
            let origin = from[axis];
            let dir = d[axis];
            let lo = min[axis];
            let hi = max[axis];
            if dir.abs() < 1e-8 {
                if origin < lo || origin > hi {
                    return false;
                }
            } else {
                let inv = 1.0 / dir;
                let t1 = (lo - origin) * inv;
                let t2 = (hi - origin) * inv;
                let (ta, tb) = if t1 < t2 { (t1, t2) } else { (t2, t1) };
                t_min = t_min.max(ta);
                t_max = t_max.min(tb);
                if t_min > t_max {
                    return false;
                }
            }
        }
        true
    }
}

/// Ramp footprint on XZ: height rises along local +Z from `base_y` to `base_y + height`.
#[derive(Clone, Copy, Debug)]
pub struct MapRamp {
    pub center_x: f32,
    pub center_z: f32,
    pub half_x: f32,
    pub half_z: f32,
    /// Rise along local +Z (metres). Surface = `base_y` … `base_y + height`.
    pub height: f32,
    /// Support height at the low end (local −Z). `0` = floor-seated yard ramp.
    pub base_y: f32,
    pub yaw: f32,
}

impl MapRamp {
    fn local_xz(self, x: f32, z: f32) -> (f32, f32) {
        let dx = x - self.center_x;
        let dz = z - self.center_z;
        let (s, c) = self.yaw.sin_cos();
        // Inverse of world_from_local (same basis as [`MapBox`]).
        (c * dx - s * dz, s * dx + c * dz)
    }

    fn rise_t(self, lz: f32) -> f32 {
        let span = (2.0 * self.half_z).max(1e-6);
        ((lz + self.half_z) / span).clamp(0.0, 1.0)
    }

    fn surface_at_t(self, t: f32) -> f32 {
        self.base_y + t * self.height
    }

    /// Surface height when `(x, z)` is on the footprint.
    pub fn surface_y(self, x: f32, z: f32) -> Option<f32> {
        let (lx, lz) = self.local_xz(x, z);
        if lx.abs() > self.half_x || lz.abs() > self.half_z {
            return None;
        }
        Some(self.surface_at_t(self.rise_t(lz)))
    }

    fn hits_circle_xz(self, x: f32, z: f32, radius: f32) -> Option<f32> {
        // Expand footprint by radius in local space (axis-aligned inflate).
        let (lx, lz) = self.local_xz(x, z);
        if lx.abs() > self.half_x + radius || lz.abs() > self.half_z + radius {
            return None;
        }
        Some(self.surface_at_t(self.rise_t(lz)))
    }
}

/// Collide / support set for one map. Empty = flat ground only.
#[derive(Clone, Debug, Default)]
pub struct MapWorld {
    pub boxes: Vec<MapBox>,
    pub ramps: Vec<MapRamp>,
}

impl MapWorld {
    pub fn empty() -> Self {
        Self::default()
    }

    /// Highest standable surface under `(x, z)` relative to `sole_y`.
    ///
    /// Prefers tops at or below `sole_y + `[`STEP_UP_M`] so a roof over a floor
    /// does not steal interior support. When nothing is within step-up reach,
    /// falls back to the absolute highest top (tall walls still block).
    pub fn support_y(&self, x: f32, z: f32, sole_y: f32) -> f32 {
        let reach = sole_y + STEP_UP_M;
        let mut best_reachable = 0.0_f32;
        let mut best_any = 0.0_f32;
        let mut found_reachable = false;
        for b in &self.boxes {
            if b.contains_xz(x, z) {
                let top = b.max_y();
                best_any = best_any.max(top);
                if top <= reach + 1e-4 {
                    best_reachable = best_reachable.max(top);
                    found_reachable = true;
                }
            }
        }
        for r in &self.ramps {
            if let Some(sy) = r.surface_y(x, z) {
                best_any = best_any.max(sy);
                if sy <= reach + 1e-4 {
                    best_reachable = best_reachable.max(sy);
                    found_reachable = true;
                }
            }
        }
        if found_reachable {
            best_reachable
        } else {
            best_any
        }
    }

    /// True when the segment `from`→`to` passes through any solid box.
    pub fn segment_hits_solid(&self, from: Vec3, to: Vec3) -> bool {
        self.boxes.iter().any(|b| b.intersects_segment(from, to))
    }

    /// Sole sample is inside a solid volume (below its top).
    pub fn inside_solid(&self, x: f32, y: f32, z: f32) -> bool {
        let r = FIGURE_RADIUS_M;
        for b in &self.boxes {
            if b.hits_circle_xz(x, z, r) && y >= b.min_y() - 1e-4 && y < b.max_y() - 1e-3 {
                return true;
            }
        }
        for ramp in &self.ramps {
            if let Some(surface) = ramp.hits_circle_xz(x, z, r) {
                if y >= -1e-4 && y < surface - 1e-3 {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_support_is_zero() {
        assert_eq!(MapWorld::empty().support_y(1.0, 2.0, 0.0), 0.0);
    }

    #[test]
    fn box_support_and_inside() {
        let world = MapWorld {
            boxes: vec![MapBox {
                center: Vec3::new(0.0, 0.5, 0.0),
                half: Vec3::new(1.0, 0.5, 1.0),
                yaw: 0.0,
            }],
            ramps: vec![],
        };
        assert!((world.support_y(0.0, 0.0, 0.0) - 1.0).abs() < 1e-5);
        assert!(world.inside_solid(0.0, 0.2, 0.0));
        assert!(!world.inside_solid(0.0, 1.0, 0.0));
        assert!(!world.inside_solid(3.0, 0.2, 0.0));
    }

    #[test]
    fn stacked_shell_support_uses_sole_reach() {
        // Floor under roof: from the floor, roof must not win.
        let world = MapWorld {
            boxes: vec![
                MapBox {
                    center: Vec3::new(0.0, 0.05, 0.0),
                    half: Vec3::new(1.0, 0.05, 2.0),
                    yaw: 0.0,
                },
                MapBox {
                    center: Vec3::new(0.0, 2.35, 0.0),
                    half: Vec3::new(1.0, 0.05, 2.0),
                    yaw: 0.0,
                },
            ],
            ramps: vec![],
        };
        assert!((world.support_y(0.0, 0.0, 0.05) - 0.1).abs() < 1e-4);
        assert!((world.support_y(0.0, 0.0, 2.4) - 2.4).abs() < 1e-4);
        assert!((world.support_y(0.0, 0.0, f32::MAX) - 2.4).abs() < 1e-4);
    }

    #[test]
    fn yawed_box_matches_local_footprint() {
        // Long thin box along local +Z, yawed 45° — world AABB would fill the diamond.
        let yaw = std::f32::consts::FRAC_PI_4;
        let box_ = MapBox {
            center: Vec3::new(0.0, 0.5, 0.0),
            half: Vec3::new(0.5, 0.5, 2.0),
            yaw,
        };
        let world = MapWorld {
            boxes: vec![box_],
            ramps: vec![],
        };
        let (s, c) = yaw.sin_cos();
        // Point along local +Z in world (world_from_local).
        let on_x = s * 1.5;
        let on_z = c * 1.5;
        assert!((world.support_y(on_x, on_z, 0.0) - 1.0).abs() < 1e-4);
        // Off-axis corners a world AABB would cover; OBB does not.
        assert!(world.support_y(1.5, 0.0, 0.0).abs() < 1e-4);
        assert!(world.support_y(0.0, 1.5, 0.0).abs() < 1e-4);
    }

    #[test]
    fn elevated_ramp_rises_from_base_y() {
        let ramp = MapRamp {
            center_x: 0.0,
            center_z: 0.0,
            half_x: 1.0,
            half_z: 2.0,
            height: 1.0,
            base_y: 2.5,
            yaw: 0.0,
        };
        let world = MapWorld {
            boxes: vec![],
            ramps: vec![ramp],
        };
        assert!((world.support_y(0.0, -2.0, 2.5) - 2.5).abs() < 1e-4);
        assert!((world.support_y(0.0, 2.0, 2.5) - 3.5).abs() < 1e-4);
        assert!((world.support_y(0.0, 0.0, 2.5) - 3.0).abs() < 1e-4);
    }

    #[test]
    fn ramp_rises_along_local_z() {
        let ramp = MapRamp {
            center_x: 0.0,
            center_z: 0.0,
            half_x: 1.0,
            half_z: 2.0,
            height: 1.0,
            base_y: 0.0,
            yaw: 0.0,
        };
        let world = MapWorld {
            boxes: vec![],
            ramps: vec![ramp],
        };
        assert!(world.support_y(0.0, -2.0, 0.0).abs() < 1e-4);
        assert!((world.support_y(0.0, 2.0, 0.0) - 1.0).abs() < 1e-4);
        assert!((world.support_y(0.0, 0.0, 0.0) - 0.5).abs() < 1e-4);
    }
}
