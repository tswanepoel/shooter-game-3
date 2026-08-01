//! Kit meshes: load, hold pose, held-weapon attach, GPU upload.
//!
//! Characters, blasters, and map solids use lit matte shading: albedo × (ambient + key × N·L).
//! Debug batches (markers) stay unlit via a material flag.
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
pub use primitives::{
    box_face_group_prim, box_prim, cylinder_y_prim, merge_transformed_prims, ramp_prim,
    transform_vertex, BoxFaceGroup, CpuPrim, MeshVertex,
};
#[cfg(feature = "debug-tools")]
pub use upload::upload_held_pair;
pub use upload::{
    upload_batch, upload_solid_batch, upload_textured_solid_batch, SolidShading, SolidUvLayout,
};
