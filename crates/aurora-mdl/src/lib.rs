//! Defensive, Apache-2.0 NWN1 MDL reader and deterministic GLB exporter.
//!
//! The binary layout is implemented independently from the CC0 `NWN1MDL.bt`
//! format description published by xoreos-docs. No GPL implementation is linked
//! or copied into this crate.

mod glb;
mod parser;

pub use glb::{GLB_CACHE_SCHEMA_VERSION, GlbArtifact, export_glb};
pub use parser::{
    AnimationEvent, AnimationTrack, MdlAnimation, MdlDiagnostic, MdlError, MdlFormat, MdlMaterial,
    MdlMesh, MdlModel, MdlNode, MdlNodeKind, MdlSkin, TrackPath, parse_mdl,
};
