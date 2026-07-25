//! Kit meshes: load, hold pose, held-weapon attach (037), GPU upload.
//!
//! Characters and blasters use lit matte shading (018): albedo × (ambient + key × N·L).
//! Solid debug batches (markers) stay unlit via a material flag.
//!
//! Held attach (feature 037):
//! `held_blaster = kit_to_world · arm_right · H · inv(G) · S_blaster`

mod anim;
mod gltf;
mod gpu;
mod kit;
mod primitives;
mod shader;
mod upload;

// Barrel re-exports for call sites (`crate::mesh::…`). Some symbols are only used
// outside this crate's current graph or via path inference; keep the full surface.
#[allow(unused_imports)]
pub use anim::{extract_clip, pose_character_kit, AnimChannel, AnimClip, AnimPath};
#[cfg(feature = "debug-tools")]
#[allow(unused_imports)]
pub use gltf::extract_character_hold;
#[allow(unused_imports)]
pub use gltf::{extract_character_parts, extract_primitives, CharPart};
#[allow(unused_imports)]
pub use gpu::{MeshBatch, UnlitMeshGpu, UnlitMeshLayout, UploadCtx};
#[allow(unused_imports)]
pub use kit::{
    hand_socket_hold, held_blaster_root, kit_to_world, letter_index, load_kenney_core,
    muzzle_locals, muzzle_world_points, primary_muzzle_offset, weapon_grip, BLASTER_MUZZLE_POINTS,
    BLASTER_RELATIVE_SCALE, BLASTER_UNITS_TO_M, CHAR_UNITS_TO_M, KENNEY_CORE_PACK,
};
#[cfg(feature = "debug-tools")]
#[allow(unused_imports)]
pub use primitives::unit_sphere_prim;
#[allow(unused_imports)]
pub use primitives::{transform_vertex, CpuPrim, MeshVertex};
#[allow(unused_imports)]
pub use upload::upload_batch;
#[cfg(feature = "debug-tools")]
#[allow(unused_imports)]
pub use upload::{upload_held_pair, upload_solid_batch};
