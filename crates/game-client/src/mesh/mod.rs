//! Kit meshes: load, hold pose, held-weapon attach, GPU upload.
//!
//! Characters and blasters use lit matte shading: albedo × (ambient + key × N·L).
//! Solid debug batches (markers) stay unlit via a material flag.
//!
//! Held attach:
//! `held_blaster = kit_to_world · arm_right · H · inv(G) · S_blaster`

mod anim;
mod gltf;
mod gpu;
mod kit;
mod primitives;
mod shader;
mod upload;

pub use anim::{extract_clip, pose_character_kit, AnimClip};
pub use gltf::{extract_character_parts, extract_primitives, CharPart};
pub use gpu::{UnlitMeshGpu, UnlitMeshLayout};
pub use kit::{
    held_blaster_root, kit_to_world, letter_index, load_kenney_core, muzzle_locals,
    muzzle_world_points, weapon_grip,
};
#[cfg(feature = "debug-tools")]
pub use primitives::unit_sphere_prim;
pub use primitives::{transform_vertex, MeshVertex};
pub use upload::upload_batch;
#[cfg(feature = "debug-tools")]
pub use upload::{upload_held_pair, upload_solid_batch};
