//! Map solids: axis-aligned boxes and ramps (066).
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

/// Axis-aligned box. Centre + half-extents (metres, Y-up).
#[derive(Clone, Copy, Debug)]
pub struct MapBox {
    pub center: Vec3,
    pub half: Vec3,
}

impl MapBox {
    pub fn min_y(self) -> f32 {
        self.center.y - self.half.y
    }

    pub fn max_y(self) -> f32 {
        self.center.y + self.half.y
    }

    fn contains_xz(self, x: f32, z: f32) -> bool {
        (x - self.center.x).abs() <= self.half.x && (z - self.center.z).abs() <= self.half.z
    }

    fn hits_circle_xz(self, x: f32, z: f32, radius: f32) -> bool {
        let dx = (x - self.center.x).abs() - self.half.x;
        let dz = (z - self.center.z).abs() - self.half.z;
        let cx = dx.max(0.0);
        let cz = dz.max(0.0);
        cx * cx + cz * cz <= radius * radius
    }

    /// Slab-based segment test. Returns true when the segment `from`→`to`
    /// passes through this box (endpoints inside also count).
    pub fn intersects_segment(self, from: Vec3, to: Vec3) -> bool {
        let min = self.center - self.half;
        let max = self.center + self.half;
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

/// Ramp footprint on XZ: height rises along local +Z from 0 to `height`.
#[derive(Clone, Copy, Debug)]
pub struct MapRamp {
    pub center_x: f32,
    pub center_z: f32,
    pub half_x: f32,
    pub half_z: f32,
    pub height: f32,
    pub yaw: f32,
}

impl MapRamp {
    fn local_xz(self, x: f32, z: f32) -> (f32, f32) {
        let dx = x - self.center_x;
        let dz = z - self.center_z;
        let (s, c) = self.yaw.sin_cos();
        (c * dx + s * dz, -s * dx + c * dz)
    }

    /// Surface height when `(x, z)` is on the footprint.
    pub fn surface_y(self, x: f32, z: f32) -> Option<f32> {
        let (lx, lz) = self.local_xz(x, z);
        if lx.abs() > self.half_x || lz.abs() > self.half_z {
            return None;
        }
        let span = (2.0 * self.half_z).max(1e-6);
        let t = ((lz + self.half_z) / span).clamp(0.0, 1.0);
        Some(t * self.height)
    }

    fn hits_circle_xz(self, x: f32, z: f32, radius: f32) -> Option<f32> {
        // Expand footprint by radius in local space (axis-aligned inflate).
        let (lx, lz) = self.local_xz(x, z);
        if lx.abs() > self.half_x + radius || lz.abs() > self.half_z + radius {
            return None;
        }
        let span = (2.0 * self.half_z).max(1e-6);
        let t = ((lz + self.half_z) / span).clamp(0.0, 1.0);
        Some(t * self.height)
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

    /// Highest standable surface under `(x, z)` (ground or solid top).
    pub fn support_y(&self, x: f32, z: f32) -> f32 {
        let mut y = 0.0_f32;
        for b in &self.boxes {
            if b.contains_xz(x, z) {
                y = y.max(b.max_y());
            }
        }
        for r in &self.ramps {
            if let Some(sy) = r.surface_y(x, z) {
                y = y.max(sy);
            }
        }
        y
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
        assert_eq!(MapWorld::empty().support_y(1.0, 2.0), 0.0);
    }

    #[test]
    fn box_support_and_inside() {
        let world = MapWorld {
            boxes: vec![MapBox {
                center: Vec3::new(0.0, 0.5, 0.0),
                half: Vec3::new(1.0, 0.5, 1.0),
            }],
            ramps: vec![],
        };
        assert!((world.support_y(0.0, 0.0) - 1.0).abs() < 1e-5);
        assert!(world.inside_solid(0.0, 0.2, 0.0));
        assert!(!world.inside_solid(0.0, 1.0, 0.0));
        assert!(!world.inside_solid(3.0, 0.2, 0.0));
    }

    #[test]
    fn ramp_rises_along_local_z() {
        let ramp = MapRamp {
            center_x: 0.0,
            center_z: 0.0,
            half_x: 1.0,
            half_z: 2.0,
            height: 1.0,
            yaw: 0.0,
        };
        let world = MapWorld {
            boxes: vec![],
            ramps: vec![ramp],
        };
        assert!(world.support_y(0.0, -2.0).abs() < 1e-4);
        assert!((world.support_y(0.0, 2.0) - 1.0).abs() < 1e-4);
        assert!((world.support_y(0.0, 0.0) - 0.5).abs() < 1e-4);
    }
}
