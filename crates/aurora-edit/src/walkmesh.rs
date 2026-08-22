use aurora_core::{AppError, AppResult, ErrorSeverity};
use aurora_mdl::{MdlFormat, parse_mdl};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::{edit_error, sha256_bytes};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WalkmeshDraft {
    pub vertices: Vec<[f32; 3]>,
    pub faces: Vec<[u32; 3]>,
    #[serde(default)]
    pub surface_ids: Vec<i32>,
    #[serde(default)]
    pub variants: Vec<WalkmeshVariantDraft>,
    #[serde(default)]
    pub hooks: Vec<WalkmeshHookDraft>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WalkmeshVariantDraft {
    pub name: String,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub vertices: Vec<[f32; 3]>,
    pub faces: Vec<[u32; 3]>,
    #[serde(default)]
    pub surface_ids: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WalkmeshHookDraft {
    pub name: String,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WalkmeshKind {
    Wok,
    Pwk,
    Dwk,
}

impl WalkmeshKind {
    pub fn resource_type(self) -> u16 {
        match self {
            Self::Wok => 2016,
            Self::Dwk => 2052,
            Self::Pwk => 2053,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WalkmeshOperation {
    SplitFace {
        face_index: usize,
    },
    RemoveFace {
        face_index: usize,
    },
    WeldVertices {
        tolerance: f32,
    },
    ExtrudeFace {
        face_index: usize,
        distance: f32,
    },
    MoveVertex {
        vertex_index: usize,
        position: [f32; 3],
    },
    SetSurface {
        face_index: usize,
        surface_id: i32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WalkmeshDocument {
    pub resref: String,
    pub kind: WalkmeshKind,
    pub source_format: String,
    pub draft: WalkmeshDraft,
    pub source_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WalkmeshValidation {
    pub valid: bool,
    pub diagnostics: Vec<String>,
}

pub fn validate_walkmesh(draft: &WalkmeshDraft) -> WalkmeshValidation {
    validate_walkmesh_document(draft, false)
}

pub fn validate_walkmesh_for_kind(draft: &WalkmeshDraft, kind: WalkmeshKind) -> WalkmeshValidation {
    let hook_only = !matches!(kind, WalkmeshKind::Wok)
        && draft.vertices.is_empty()
        && draft.faces.is_empty()
        && !draft.hooks.is_empty();
    validate_walkmesh_document(draft, hook_only)
}

fn validate_walkmesh_document(
    draft: &WalkmeshDraft,
    allow_empty_primary: bool,
) -> WalkmeshValidation {
    let mut diagnostics = Vec::new();
    validate_walkmesh_geometry(
        "principal",
        &draft.vertices,
        &draft.faces,
        &draft.surface_ids,
        allow_empty_primary,
        &mut diagnostics,
    );
    for variant in &draft.variants {
        if variant.name.trim().is_empty() || variant.name.len() > 63 {
            diagnostics.push("Une variante possède un nom vide ou trop long.".to_owned());
        }
        if !variant.position.iter().all(|value| value.is_finite())
            || !variant.rotation.iter().all(|value| value.is_finite())
        {
            diagnostics.push(format!(
                "Variante {} avec transformation non finie.",
                variant.name
            ));
        }
        validate_walkmesh_geometry(
            &variant.name,
            &variant.vertices,
            &variant.faces,
            &variant.surface_ids,
            false,
            &mut diagnostics,
        );
    }
    for hook in &draft.hooks {
        if hook.name.trim().is_empty() || hook.name.len() > 63 {
            diagnostics.push("Un point d'usage possède un nom vide ou trop long.".to_owned());
        }
        if !hook.position.iter().all(|value| value.is_finite())
            || !hook.rotation.iter().all(|value| value.is_finite())
        {
            diagnostics.push(format!("Point d'usage {} non fini.", hook.name));
        }
    }
    WalkmeshValidation {
        valid: diagnostics.is_empty(),
        diagnostics,
    }
}

fn validate_walkmesh_geometry(
    label: &str,
    vertices: &[[f32; 3]],
    faces: &[[u32; 3]],
    surface_ids: &[i32],
    allow_empty: bool,
    diagnostics: &mut Vec<String>,
) {
    if (vertices.is_empty() || faces.is_empty())
        && !(allow_empty && vertices.is_empty() && faces.is_empty())
    {
        diagnostics.push(format!("Le walkmesh {label} ne contient pas de géométrie."));
    }
    if vertices.len() > 1_000_000 || faces.len() > 2_000_000 {
        diagnostics.push(format!(
            "Le walkmesh {label} dépasse les limites de sécurité."
        ));
    }
    for (index, vertex) in vertices.iter().enumerate() {
        if !vertex.iter().all(|value| value.is_finite()) {
            diagnostics.push(format!("Sommet {index} de {label} non fini."));
        }
    }
    let mut unique_faces = std::collections::BTreeSet::new();
    let mut edges = BTreeMap::<(u32, u32), Vec<(usize, bool)>>::new();
    for (index, face) in faces.iter().enumerate() {
        if face[0] == face[1] || face[1] == face[2] || face[0] == face[2] {
            diagnostics.push(format!("Face {index} de {label} dégénérée."));
        }
        if face.iter().any(|vertex| *vertex as usize >= vertices.len()) {
            diagnostics.push(format!(
                "Face {index} de {label} référence un sommet absent."
            ));
            continue;
        }
        let mut canonical = *face;
        canonical.sort_unstable();
        if !unique_faces.insert(canonical) {
            diagnostics.push(format!("Face {index} de {label} dupliquée."));
        }
        let [a, b, c] = face.map(|vertex| vertices[vertex as usize]);
        let cross = triangle_cross(a, b, c);
        if vector_length_squared(cross) <= 1.0e-12 {
            diagnostics.push(format!(
                "Face {index} de {label} de surface géométrique nulle."
            ));
        }
        for (from, to) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            let edge = if from < to { (from, to) } else { (to, from) };
            edges.entry(edge).or_default().push((index, from < to));
        }
    }
    for (edge, owners) in edges {
        if owners.len() > 2 {
            diagnostics.push(format!(
                "Arête {}-{} non-manifold dans {label} ({} faces).",
                edge.0,
                edge.1,
                owners.len()
            ));
        } else if owners.len() == 2 && owners[0].1 == owners[1].1 {
            diagnostics.push(format!(
                "Orientation incohérente autour de l'arête {}-{} dans {label}.",
                edge.0, edge.1
            ));
        }
    }
    if !surface_ids.is_empty() && surface_ids.len() != faces.len() {
        diagnostics.push(format!(
            "Le nombre d'identifiants de surface de {label} doit correspondre au nombre de faces."
        ));
    }
}

/// Imports standalone ASCII WOK/PWK/DWK resources and regular MDL AABB data.
/// Every geometry state and use/door hook remains explicit in the draft.
pub fn inspect_walkmesh(
    resref: &str,
    kind: WalkmeshKind,
    bytes: &[u8],
) -> AppResult<WalkmeshDocument> {
    validate_walkmesh_resref(resref)?;
    let model = parse_mdl(bytes).map_err(|error| {
        Box::new(
            AppError::new(
                "EDIT_WALKMESH_PARSE_FAILED",
                "Le walkmesh n'a pas pu etre lu.",
                error.to_string(),
                ErrorSeverity::Error,
            )
            .with_resource(format!("{resref}.{}", walkmesh_extension(kind)))
            .with_import_stage("walkmesh"),
        )
    })?;
    let mesh_nodes = model
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| {
            node.mesh.as_ref().is_some_and(|mesh| match kind {
                WalkmeshKind::Wok => mesh.walkmesh,
                WalkmeshKind::Pwk | WalkmeshKind::Dwk => true,
            })
        })
        .collect::<Vec<_>>();
    let primary = mesh_nodes
        .iter()
        .position(|(_, node)| match kind {
            WalkmeshKind::Wok => node.mesh.as_ref().is_some_and(|mesh| mesh.walkmesh),
            WalkmeshKind::Pwk => node.name.to_ascii_lowercase().contains("nowalk"),
            WalkmeshKind::Dwk => node.name.to_ascii_lowercase().contains("wg_closed"),
        })
        .unwrap_or_default();
    let mut draft = WalkmeshDraft::default();
    for (position, (_, node)) in mesh_nodes.iter().enumerate() {
        let mesh = node.mesh.as_ref().expect("mesh node filtered above");
        let geometry = walkmesh_geometry_from_mesh(mesh);
        if position == primary {
            draft.vertices = geometry
                .0
                .into_iter()
                .map(|vertex| transform_point(vertex, node.translation, node.rotation))
                .collect();
            draft.faces = geometry.1;
            draft.surface_ids = geometry.2;
        } else {
            draft.variants.push(WalkmeshVariantDraft {
                name: node.name.clone(),
                position: node.translation,
                rotation: node.rotation,
                vertices: geometry.0,
                faces: geometry.1,
                surface_ids: geometry.2,
            });
        }
    }
    draft.hooks = model
        .nodes
        .iter()
        .filter(|node| node.mesh.is_none() && node.kinds.contains(&aurora_mdl::MdlNodeKind::Dummy))
        .map(|node| WalkmeshHookDraft {
            name: node.name.clone(),
            position: node.translation,
            rotation: node.rotation,
        })
        .collect();
    if draft.faces.is_empty() && (matches!(kind, WalkmeshKind::Wok) || draft.hooks.is_empty()) {
        return Err(edit_error(
            "EDIT_WALKMESH_EMPTY",
            format!(
                "{resref}.{} contains no walkmesh face",
                walkmesh_extension(kind)
            ),
        ));
    }
    Ok(WalkmeshDocument {
        resref: resref.to_owned(),
        kind,
        source_format: match model.format {
            MdlFormat::Ascii => "ascii",
            MdlFormat::Binary => "binary",
        }
        .to_owned(),
        draft,
        source_sha256: sha256_bytes(bytes),
    })
}

fn walkmesh_geometry_from_mesh(
    mesh: &aurora_mdl::MdlMesh,
) -> (Vec<[f32; 3]>, Vec<[u32; 3]>, Vec<i32>) {
    let faces = mesh
        .indices
        .chunks_exact(3)
        .map(|face| [face[0], face[1], face[2]])
        .collect::<Vec<_>>();
    let surfaces = (0..faces.len())
        .map(|index| mesh.surface_ids.get(index).copied().unwrap_or_default())
        .collect();
    (mesh.positions.clone(), faces, surfaces)
}

/// Serializes the standalone ASCII grammars used by NWN for WOK, PWK and DWK.
/// WOK resources include a deterministic AABB tree; PWK/DWK keep every
/// geometry state and interaction hook.
pub fn serialize_walkmesh_ascii(
    resref: &str,
    kind: WalkmeshKind,
    draft: &WalkmeshDraft,
) -> AppResult<Vec<u8>> {
    validate_walkmesh_resref(resref)?;
    let validation = validate_walkmesh_for_kind(draft, kind);
    if !validation.valid {
        return Err(edit_error(
            "EDIT_WALKMESH_INVALID",
            validation.diagnostics.join(" "),
        ));
    }
    validate_walkmesh_kind(kind, draft)?;
    let mut output = String::from("# Generated by OpenNever Forge (Apache-2.0)\n");
    match kind {
        WalkmeshKind::Wok => {
            output.push_str("#NWmax WALKMESH  ASCII\n");
            output.push_str(&format!("beginwalkmeshgeom {resref}\n"));
            output.push_str(&format!("node aabb {resref}\n  parent {resref}\n"));
            output.push_str("  position 0 0 0\n  orientation 1 0 0 0\n  render 0\n");
            output.push_str("  multimaterial 20\n");
            for surface in WALKMESH_SURFACES {
                output.push_str(&format!("    {surface}\n"));
            }
            write_walkmesh_geometry(
                &mut output,
                &draft.vertices,
                &draft.faces,
                &draft.surface_ids,
                true,
            );
            write_walkmesh_aabb_tree(&mut output, &draft.vertices, &draft.faces);
            output.push_str("endnode\n");
            output.push_str(&format!("endwalkmeshgeom {resref}\n"));
        }
        WalkmeshKind::Pwk => {
            output.push_str("#NWmax PWKMESH  ASCII\n");
            let parent = format!("{resref}_pwk");
            if !draft.faces.is_empty() {
                write_walkmesh_trimesh(
                    &mut output,
                    "NoWalk",
                    &parent,
                    WalkmeshTrimesh {
                        position: [0.0, 0.0, 0.0],
                        rotation: [0.0, 0.0, 0.0, 1.0],
                        vertices: &draft.vertices,
                        faces: &draft.faces,
                        surfaces: &draft.surface_ids,
                    },
                );
            }
            for variant in &draft.variants {
                write_walkmesh_trimesh(
                    &mut output,
                    &variant.name,
                    &parent,
                    WalkmeshTrimesh {
                        position: variant.position,
                        rotation: variant.rotation,
                        vertices: &variant.vertices,
                        faces: &variant.faces,
                        surfaces: &variant.surface_ids,
                    },
                );
            }
            let hooks = if draft.hooks.is_empty() {
                default_pwk_hooks(resref, &draft.vertices)
            } else {
                draft.hooks.clone()
            };
            for hook in &hooks {
                write_walkmesh_hook(&mut output, hook, &parent);
            }
        }
        WalkmeshKind::Dwk => {
            output.push_str("#NWmax DWKMESH  ASCII\n");
            let parent = format!("{resref}_DWK");
            let hooks = if draft.hooks.is_empty() {
                default_dwk_hooks(resref, &draft.vertices)
            } else {
                draft.hooks.clone()
            };
            for hook in &hooks {
                write_walkmesh_hook(&mut output, hook, &parent);
            }
            if !draft.faces.is_empty() {
                write_walkmesh_trimesh(
                    &mut output,
                    &format!("{resref}_DWK_wg_closed"),
                    &parent,
                    WalkmeshTrimesh {
                        position: [0.0, 0.0, 0.0],
                        rotation: [0.0, 0.0, 0.0, 1.0],
                        vertices: &draft.vertices,
                        faces: &draft.faces,
                        surfaces: &draft.surface_ids,
                    },
                );
                let variants = dwk_variants(resref, draft);
                for variant in &variants {
                    write_walkmesh_trimesh(
                        &mut output,
                        &variant.name,
                        &parent,
                        WalkmeshTrimesh {
                            position: variant.position,
                            rotation: variant.rotation,
                            vertices: &variant.vertices,
                            faces: &variant.faces,
                            surfaces: &variant.surface_ids,
                        },
                    );
                }
            }
        }
    }
    Ok(output.into_bytes())
}

pub fn split_walkmesh_face(draft: &mut WalkmeshDraft, face_index: usize) -> AppResult<()> {
    let face = *draft.faces.get(face_index).ok_or_else(|| {
        edit_error(
            "EDIT_WALKMESH_FACE_NOT_FOUND",
            format!("face {face_index} does not exist"),
        )
    })?;
    if face
        .iter()
        .any(|vertex| *vertex as usize >= draft.vertices.len())
    {
        return Err(edit_error(
            "EDIT_WALKMESH_FACE_INVALID",
            format!("face {face_index} references a missing vertex"),
        ));
    }
    let [a, b, c] = face.map(|vertex| draft.vertices[vertex as usize]);
    let centroid = [
        (a[0] + b[0] + c[0]) / 3.0,
        (a[1] + b[1] + c[1]) / 3.0,
        (a[2] + b[2] + c[2]) / 3.0,
    ];
    let center = draft.vertices.len() as u32;
    draft.vertices.push(centroid);
    draft.faces[face_index] = [face[0], face[1], center];
    draft.faces.push([face[1], face[2], center]);
    draft.faces.push([face[2], face[0], center]);
    if !draft.surface_ids.is_empty() {
        let surface = draft.surface_ids[face_index];
        draft.surface_ids.push(surface);
        draft.surface_ids.push(surface);
    }
    Ok(())
}

pub fn apply_walkmesh_operation(
    draft: &mut WalkmeshDraft,
    operation: &WalkmeshOperation,
) -> AppResult<WalkmeshValidation> {
    match operation {
        WalkmeshOperation::SplitFace { face_index } => split_walkmesh_face(draft, *face_index)?,
        WalkmeshOperation::RemoveFace { face_index } => {
            if *face_index >= draft.faces.len() {
                return Err(edit_error(
                    "EDIT_WALKMESH_FACE_NOT_FOUND",
                    format!("face {face_index} does not exist"),
                ));
            }
            draft.faces.remove(*face_index);
            if !draft.surface_ids.is_empty() && *face_index < draft.surface_ids.len() {
                draft.surface_ids.remove(*face_index);
            }
            compact_walkmesh_vertices(draft);
        }
        WalkmeshOperation::WeldVertices { tolerance } => {
            weld_walkmesh_vertices(draft, *tolerance)?;
        }
        WalkmeshOperation::ExtrudeFace {
            face_index,
            distance,
        } => extrude_walkmesh_face(draft, *face_index, *distance)?,
        WalkmeshOperation::MoveVertex {
            vertex_index,
            position,
        } => {
            if !position.iter().all(|value| value.is_finite()) {
                return Err(edit_error(
                    "EDIT_WALKMESH_VERTEX_INVALID",
                    "vertex position must contain finite values",
                ));
            }
            let vertex = draft.vertices.get_mut(*vertex_index).ok_or_else(|| {
                edit_error(
                    "EDIT_WALKMESH_VERTEX_NOT_FOUND",
                    format!("vertex {vertex_index} does not exist"),
                )
            })?;
            *vertex = *position;
        }
        WalkmeshOperation::SetSurface {
            face_index,
            surface_id,
        } => {
            if *face_index >= draft.faces.len() {
                return Err(edit_error(
                    "EDIT_WALKMESH_FACE_NOT_FOUND",
                    format!("face {face_index} does not exist"),
                ));
            }
            draft.surface_ids.resize(draft.faces.len(), 0);
            draft.surface_ids[*face_index] = *surface_id;
        }
    }
    Ok(validate_walkmesh(draft))
}

fn extrude_walkmesh_face(
    draft: &mut WalkmeshDraft,
    face_index: usize,
    distance: f32,
) -> AppResult<()> {
    if !distance.is_finite() || distance.abs() < 1.0e-5 || distance.abs() > 1_000.0 {
        return Err(edit_error(
            "EDIT_WALKMESH_EXTRUSION_INVALID",
            "extrusion distance must be finite and between 0.00001 and 1000",
        ));
    }
    let face = *draft.faces.get(face_index).ok_or_else(|| {
        edit_error(
            "EDIT_WALKMESH_FACE_NOT_FOUND",
            format!("face {face_index} does not exist"),
        )
    })?;
    if face
        .iter()
        .any(|vertex| *vertex as usize >= draft.vertices.len())
    {
        return Err(edit_error(
            "EDIT_WALKMESH_FACE_INVALID",
            format!("face {face_index} references a missing vertex"),
        ));
    }
    let [a, b, c] = face.map(|vertex| draft.vertices[vertex as usize]);
    let normal = normalize_vector(triangle_cross(a, b, c)).ok_or_else(|| {
        edit_error(
            "EDIT_WALKMESH_FACE_INVALID",
            format!("face {face_index} has no geometric normal"),
        )
    })?;
    let offset = normal.map(|value| value * distance);
    let [d, e, f] = [a, b, c].map(|vertex| {
        [
            vertex[0] + offset[0],
            vertex[1] + offset[1],
            vertex[2] + offset[2],
        ]
    });
    let first = draft.vertices.len() as u32;
    draft.vertices.extend([d, e, f]);
    let [a, b, c] = face;
    let [d, e, f] = [first, first + 1, first + 2];
    draft.faces.extend([
        [f, e, d],
        [b, a, d],
        [b, d, e],
        [c, b, e],
        [c, e, f],
        [a, c, f],
        [a, f, d],
    ]);
    if !draft.surface_ids.is_empty() {
        let surface = draft
            .surface_ids
            .get(face_index)
            .copied()
            .unwrap_or_default();
        draft.surface_ids.extend([surface; 7]);
    }
    Ok(())
}

fn weld_walkmesh_vertices(draft: &mut WalkmeshDraft, tolerance: f32) -> AppResult<()> {
    if !tolerance.is_finite() || !(1.0e-6..=10.0).contains(&tolerance) {
        return Err(edit_error(
            "EDIT_WALKMESH_WELD_TOLERANCE_INVALID",
            "weld tolerance must be between 0.000001 and 10",
        ));
    }
    let mut cells = BTreeMap::<(i64, i64, i64), Vec<u32>>::new();
    let mut vertices = Vec::<[f32; 3]>::new();
    let mut remap = Vec::with_capacity(draft.vertices.len());
    for vertex in &draft.vertices {
        let cell = (
            (vertex[0] / tolerance).floor() as i64,
            (vertex[1] / tolerance).floor() as i64,
            (vertex[2] / tolerance).floor() as i64,
        );
        let mut replacement = None;
        'neighbors: for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    if let Some(candidates) = cells.get(&(cell.0 + x, cell.1 + y, cell.2 + z)) {
                        for candidate in candidates {
                            let existing = vertices[*candidate as usize];
                            let delta = [
                                existing[0] - vertex[0],
                                existing[1] - vertex[1],
                                existing[2] - vertex[2],
                            ];
                            if vector_length_squared(delta) <= tolerance * tolerance {
                                replacement = Some(*candidate);
                                break 'neighbors;
                            }
                        }
                    }
                }
            }
        }
        let index = replacement.unwrap_or_else(|| {
            let index = vertices.len() as u32;
            vertices.push(*vertex);
            cells.entry(cell).or_default().push(index);
            index
        });
        remap.push(index);
    }
    let mut faces = Vec::with_capacity(draft.faces.len());
    let mut surfaces = Vec::with_capacity(draft.faces.len());
    for (index, face) in draft.faces.iter().enumerate() {
        if face.iter().any(|value| *value as usize >= remap.len()) {
            return Err(edit_error(
                "EDIT_WALKMESH_FACE_INVALID",
                format!("face {index} references a missing vertex"),
            ));
        }
        let face = face.map(|value| remap[value as usize]);
        if face[0] == face[1] || face[1] == face[2] || face[0] == face[2] {
            continue;
        }
        faces.push(face);
        surfaces.push(draft.surface_ids.get(index).copied().unwrap_or_default());
    }
    draft.vertices = vertices;
    draft.faces = faces;
    draft.surface_ids = surfaces;
    compact_walkmesh_vertices(draft);
    Ok(())
}

fn compact_walkmesh_vertices(draft: &mut WalkmeshDraft) {
    let mut used = vec![false; draft.vertices.len()];
    for face in &draft.faces {
        for vertex in face {
            if let Some(value) = used.get_mut(*vertex as usize) {
                *value = true;
            }
        }
    }
    let mut remap = vec![0_u32; draft.vertices.len()];
    let mut vertices = Vec::with_capacity(draft.vertices.len());
    for (index, vertex) in draft.vertices.iter().enumerate() {
        if used[index] {
            remap[index] = vertices.len() as u32;
            vertices.push(*vertex);
        }
    }
    for face in &mut draft.faces {
        *face = face.map(|vertex| remap[vertex as usize]);
    }
    draft.vertices = vertices;
}

const WALKMESH_SURFACES: [&str; 20] = [
    "Dirt",
    "Obscuring",
    "Grass",
    "Stone",
    "Wood",
    "Water",
    "Nonwalk",
    "Transparent",
    "Carpet",
    "Metal",
    "Puddles",
    "Swamp",
    "Mud",
    "Leaves",
    "Lava",
    "BottomlessPit",
    "DeepWater",
    "Door",
    "Snow",
    "Sand",
];

fn validate_walkmesh_kind(kind: WalkmeshKind, draft: &WalkmeshDraft) -> AppResult<()> {
    if matches!(kind, WalkmeshKind::Wok)
        && draft
            .surface_ids
            .iter()
            .any(|surface| !(0..=19).contains(surface))
    {
        return Err(edit_error(
            "EDIT_WALKMESH_SURFACE_INVALID",
            "WOK surface identifiers must be between 0 and 19",
        ));
    }
    if matches!(kind, WalkmeshKind::Wok) && (!draft.variants.is_empty() || !draft.hooks.is_empty())
    {
        return Err(edit_error(
            "EDIT_WALKMESH_WOK_STRUCTURE_INVALID",
            "WOK resources accept one AABB geometry and no PWK/DWK hooks",
        ));
    }
    Ok(())
}

fn write_walkmesh_geometry(
    output: &mut String,
    vertices: &[[f32; 3]],
    faces: &[[u32; 3]],
    surfaces: &[i32],
    include_tverts: bool,
) {
    output.push_str(&format!("  verts {}\n", vertices.len()));
    for vertex in vertices {
        output.push_str(&format!(
            "    {} {} {}\n",
            format_walkmesh_float(vertex[0]),
            format_walkmesh_float(vertex[1]),
            format_walkmesh_float(vertex[2])
        ));
    }
    output.push_str(&format!("  faces {}\n", faces.len()));
    for (index, face) in faces.iter().enumerate() {
        let surface = surfaces.get(index).copied().unwrap_or_default();
        let texture = if include_tverts { *face } else { [0, 0, 0] };
        output.push_str(&format!(
            "    {} {} {}  {}  {} {} {}  {}\n",
            face[0],
            face[1],
            face[2],
            surface,
            texture[0],
            texture[1],
            texture[2],
            if include_tverts { 4 } else { 7 }
        ));
    }
    if include_tverts {
        let (minimum, maximum) = walkmesh_bounds(vertices.iter().copied());
        let width = (maximum[0] - minimum[0]).abs().max(1.0);
        let height = (maximum[1] - minimum[1]).abs().max(1.0);
        output.push_str(&format!("  tverts {}\n", vertices.len()));
        for vertex in vertices {
            output.push_str(&format!(
                "    {} {} 0\n",
                format_walkmesh_float((vertex[0] - minimum[0]) / width),
                format_walkmesh_float((vertex[1] - minimum[1]) / height)
            ));
        }
    }
}

struct WalkmeshTrimesh<'a> {
    position: [f32; 3],
    rotation: [f32; 4],
    vertices: &'a [[f32; 3]],
    faces: &'a [[u32; 3]],
    surfaces: &'a [i32],
}

fn write_walkmesh_trimesh(
    output: &mut String,
    name: &str,
    parent: &str,
    geometry: WalkmeshTrimesh<'_>,
) {
    output.push_str(&format!("node trimesh {name}\n  parent {parent}\n"));
    write_walkmesh_transform(output, geometry.position, geometry.rotation);
    output.push_str("  bitmap NULL\n");
    write_walkmesh_geometry(
        output,
        geometry.vertices,
        geometry.faces,
        geometry.surfaces,
        false,
    );
    output.push_str("endnode\n");
}

fn write_walkmesh_hook(output: &mut String, hook: &WalkmeshHookDraft, parent: &str) {
    output.push_str(&format!("node dummy {}\n  parent {parent}\n", hook.name));
    write_walkmesh_transform(output, hook.position, hook.rotation);
    output.push_str("endnode\n");
}

fn write_walkmesh_transform(output: &mut String, position: [f32; 3], rotation: [f32; 4]) {
    let orientation = quaternion_axis_angle(rotation);
    output.push_str(&format!(
        "  position {} {} {}\n  orientation {} {} {} {}\n",
        format_walkmesh_float(position[0]),
        format_walkmesh_float(position[1]),
        format_walkmesh_float(position[2]),
        format_walkmesh_float(orientation[0]),
        format_walkmesh_float(orientation[1]),
        format_walkmesh_float(orientation[2]),
        format_walkmesh_float(orientation[3])
    ));
}

fn default_pwk_hooks(resref: &str, vertices: &[[f32; 3]]) -> Vec<WalkmeshHookDraft> {
    let (minimum, maximum) = walkmesh_bounds(vertices.iter().copied());
    let x = (minimum[0] + maximum[0]) * 0.5;
    let z = maximum[2];
    vec![
        WalkmeshHookDraft {
            name: format!("{resref}_pwk_use01"),
            position: [x, maximum[1], z],
            rotation: [0.0, 0.0, 0.0, 1.0],
        },
        WalkmeshHookDraft {
            name: format!("{resref}_pwk_use02"),
            position: [x, minimum[1], z],
            rotation: [0.0, 0.0, 0.0, 1.0],
        },
    ]
}

fn default_dwk_hooks(resref: &str, vertices: &[[f32; 3]]) -> Vec<WalkmeshHookDraft> {
    let (minimum, maximum) = walkmesh_bounds(vertices.iter().copied());
    let x = (minimum[0] + maximum[0]) * 0.5;
    let y = (minimum[1] + maximum[1]) * 0.5;
    let z = minimum[2];
    [
        ("closed_01", [x, minimum[1], z]),
        ("closed_02", [x, maximum[1], z]),
        ("open1_01", [minimum[0], y, z]),
        ("open1_02", [minimum[0], maximum[1], z]),
        ("open2_01", [maximum[0], y, z]),
        ("open2_02", [maximum[0], minimum[1], z]),
    ]
    .into_iter()
    .map(|(suffix, position)| WalkmeshHookDraft {
        name: format!("{resref}_DWK_dp_{suffix}"),
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
    })
    .collect()
}

fn dwk_variants(resref: &str, draft: &WalkmeshDraft) -> Vec<WalkmeshVariantDraft> {
    ["open1", "open2"]
        .into_iter()
        .map(|state| {
            draft
                .variants
                .iter()
                .find(|variant| variant.name.to_ascii_lowercase().contains(state))
                .cloned()
                .unwrap_or_else(|| WalkmeshVariantDraft {
                    name: format!("{resref}_DWK_wg_{state}"),
                    position: [0.0, 0.0, 0.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    vertices: draft.vertices.clone(),
                    faces: draft.faces.clone(),
                    surface_ids: draft.surface_ids.clone(),
                })
        })
        .collect()
}

fn write_walkmesh_aabb_tree(output: &mut String, vertices: &[[f32; 3]], faces: &[[u32; 3]]) {
    let indices = (0..faces.len()).collect::<Vec<_>>();
    write_aabb_branch(output, vertices, faces, &indices, 4);
}

fn write_aabb_branch(
    output: &mut String,
    vertices: &[[f32; 3]],
    faces: &[[u32; 3]],
    indices: &[usize],
    indent: usize,
) {
    let points = indices.iter().flat_map(|index| {
        faces[*index]
            .into_iter()
            .map(|vertex| vertices[vertex as usize])
    });
    let (minimum, maximum) = walkmesh_bounds(points);
    output.push_str(&" ".repeat(indent));
    output.push_str(&format!(
        "aabb {} {} {} {} {} {} {}\n",
        format_walkmesh_float(minimum[0]),
        format_walkmesh_float(minimum[1]),
        format_walkmesh_float(minimum[2]),
        format_walkmesh_float(maximum[0]),
        format_walkmesh_float(maximum[1]),
        format_walkmesh_float(maximum[2]),
        indices
            .first()
            .copied()
            .filter(|_| indices.len() == 1)
            .map(|value| value as i64)
            .unwrap_or(-1)
    ));
    if indices.len() <= 1 {
        return;
    }
    let extent = [
        maximum[0] - minimum[0],
        maximum[1] - minimum[1],
        maximum[2] - minimum[2],
    ];
    let axis = if extent[0] >= extent[1] && extent[0] >= extent[2] {
        0
    } else if extent[1] >= extent[2] {
        1
    } else {
        2
    };
    let mut sorted = indices.to_vec();
    sorted.sort_by(|left, right| {
        let centroid = |index: usize| {
            let face = faces[index];
            face.into_iter()
                .map(|vertex| vertices[vertex as usize][axis])
                .sum::<f32>()
                / 3.0
        };
        centroid(*left)
            .total_cmp(&centroid(*right))
            .then_with(|| left.cmp(right))
    });
    let middle = sorted.len() / 2;
    write_aabb_branch(output, vertices, faces, &sorted[..middle], indent + 4);
    write_aabb_branch(output, vertices, faces, &sorted[middle..], indent + 4);
}

fn walkmesh_bounds(points: impl Iterator<Item = [f32; 3]>) -> ([f32; 3], [f32; 3]) {
    points.fold(
        ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]),
        |(mut minimum, mut maximum), point| {
            for axis in 0..3 {
                minimum[axis] = minimum[axis].min(point[axis]);
                maximum[axis] = maximum[axis].max(point[axis]);
            }
            (minimum, maximum)
        },
    )
}

fn triangle_cross(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ]
}

fn vector_length_squared(value: [f32; 3]) -> f32 {
    value.iter().map(|component| component * component).sum()
}

fn normalize_vector(value: [f32; 3]) -> Option<[f32; 3]> {
    let length = vector_length_squared(value).sqrt();
    (length > 1.0e-6).then(|| value.map(|component| component / length))
}

fn transform_point(point: [f32; 3], translation: [f32; 3], rotation: [f32; 4]) -> [f32; 3] {
    let [x, y, z, w] = rotation;
    let dot_uv = x * point[0] + y * point[1] + z * point[2];
    let dot_uu = x * x + y * y + z * z;
    let cross = [
        y * point[2] - z * point[1],
        z * point[0] - x * point[2],
        x * point[1] - y * point[0],
    ];
    [
        2.0 * dot_uv * x + (w * w - dot_uu) * point[0] + 2.0 * w * cross[0] + translation[0],
        2.0 * dot_uv * y + (w * w - dot_uu) * point[1] + 2.0 * w * cross[1] + translation[1],
        2.0 * dot_uv * z + (w * w - dot_uu) * point[2] + 2.0 * w * cross[2] + translation[2],
    ]
}

fn quaternion_axis_angle(rotation: [f32; 4]) -> [f32; 4] {
    let length = rotation
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();
    let quaternion = if length > f32::EPSILON {
        rotation.map(|value| value / length)
    } else {
        [0.0, 0.0, 0.0, 1.0]
    };
    let angle = 2.0 * quaternion[3].clamp(-1.0, 1.0).acos();
    let sine = (1.0 - quaternion[3] * quaternion[3]).max(0.0).sqrt();
    if sine <= 1.0e-6 {
        [1.0, 0.0, 0.0, 0.0]
    } else {
        [
            quaternion[0] / sine,
            quaternion[1] / sine,
            quaternion[2] / sine,
            angle,
        ]
    }
}

fn validate_walkmesh_resref(resref: &str) -> AppResult<()> {
    if resref.is_empty()
        || resref.len() > 16
        || !resref
            .bytes()
            .all(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || value == b'_')
    {
        return Err(edit_error(
            "EDIT_WALKMESH_RESREF_INVALID",
            "walkmesh resref must contain 1-16 lowercase ASCII letters, digits, or underscores",
        ));
    }
    Ok(())
}

fn walkmesh_extension(kind: WalkmeshKind) -> &'static str {
    match kind {
        WalkmeshKind::Wok => "wok",
        WalkmeshKind::Pwk => "pwk",
        WalkmeshKind::Dwk => "dwk",
    }
}

fn format_walkmesh_float(value: f32) -> String {
    if value == 0.0 {
        return "0".to_owned();
    }
    let value = format!("{value:.6}");
    value.trim_end_matches('0').trim_end_matches('.').to_owned()
}
