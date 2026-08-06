use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};

const FILE_HEADER_SIZE: usize = 12;
const GEOMETRY_HEADER_SIZE: usize = 112;
const MODEL_HEADER_SIZE: usize = 232;
const NODE_HEADER_SIZE: usize = 112;
const MESH_HEADER_SIZE: usize = 512;
const MAX_NODES: usize = 65_536;
const MAX_ARRAY_ENTRIES: usize = 4_000_000;
const MAX_VERTICES: usize = 2_000_000;
const MAX_FACES: usize = 4_000_000;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum MdlFormat {
    Binary,
    Ascii,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum MdlNodeKind {
    Dummy,
    Light,
    Emitter,
    Camera,
    Reference,
    Trimesh,
    Skin,
    Animmesh,
    Danglymesh,
    Aabb,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MdlMaterial {
    pub diffuse: [f32; 3],
    pub ambient: [f32; 3],
    pub specular: [f32; 3],
    pub shininess: f32,
    pub transparency_hint: u32,
    pub textures: Vec<String>,
    pub render: bool,
    /// Aurora tile visibility mode: 0 = always visible, 1 = fade,
    /// 2 = tile base, 3 = fade with neighboring tiles.
    #[serde(default)]
    pub tile_fade: u32,
}

impl Default for MdlMaterial {
    fn default() -> Self {
        Self {
            diffuse: [0.8, 0.8, 0.8],
            ambient: [0.2, 0.2, 0.2],
            specular: [0.0, 0.0, 0.0],
            shininess: 0.0,
            transparency_hint: 0,
            textures: Vec::new(),
            render: true,
            tile_fade: 0,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MdlSkin {
    pub weights: Vec<[f32; 4]>,
    pub bone_indices: Vec<[u16; 4]>,
    pub bone_mapping: Vec<i16>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MdlMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uv0: Vec<[f32; 2]>,
    pub colors: Vec<[u8; 4]>,
    pub indices: Vec<u32>,
    /// Per-face Aurora surface/material identifier. For AABB walkmeshes this
    /// is the walkability surface carried by the MDL face record.
    #[serde(default)]
    pub surface_ids: Vec<i32>,
    pub material: MdlMaterial,
    pub skin: Option<MdlSkin>,
    pub walkmesh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MdlNode {
    pub name: String,
    pub node_number: u32,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub kinds: BTreeSet<MdlNodeKind>,
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    pub mesh: Option<MdlMesh>,
    pub reference_model: Option<String>,
}

impl MdlNode {
    fn placeholder(parent: Option<usize>) -> Self {
        Self {
            name: String::new(),
            node_number: 0,
            parent,
            children: Vec::new(),
            kinds: BTreeSet::new(),
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
            mesh: None,
            reference_model: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum TrackPath {
    Translation,
    Rotation,
    Scale,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnimationTrack {
    pub node: String,
    pub path: TrackPath,
    pub times: Vec<f32>,
    pub values: Vec<[f32; 4]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnimationEvent {
    pub time: f32,
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MdlAnimation {
    pub name: String,
    pub root_node: String,
    pub length: f32,
    pub transition: f32,
    pub events: Vec<AnimationEvent>,
    pub tracks: Vec<AnimationTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MdlDiagnostic {
    pub code: String,
    pub message: String,
    pub offset: Option<usize>,
    pub node: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MdlModel {
    pub format: MdlFormat,
    pub name: String,
    pub supermodel: Option<String>,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub radius: f32,
    pub model_scale: f32,
    pub nodes: Vec<MdlNode>,
    pub animations: Vec<MdlAnimation>,
    pub diagnostics: Vec<MdlDiagnostic>,
    pub source_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MdlError {
    pub code: String,
    pub message: String,
    pub offset: Option<usize>,
}

impl MdlError {
    fn new(code: &str, message: impl Into<String>, offset: Option<usize>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
            offset,
        }
    }
}

impl Display for MdlError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self.offset {
            Some(offset) => write!(formatter, "{} at 0x{offset:X}: {}", self.code, self.message),
            None => write!(formatter, "{}: {}", self.code, self.message),
        }
    }
}

impl std::error::Error for MdlError {}

pub fn parse_mdl(bytes: &[u8]) -> Result<MdlModel, MdlError> {
    if is_ascii_mdl(bytes) {
        parse_ascii(bytes)
    } else {
        BinaryParser::new(bytes)?.parse()
    }
}

fn is_ascii_mdl(bytes: &[u8]) -> bool {
    if bytes.starts_with(b"newmodel")
        || bytes.starts_with(b"#MAXMODEL")
        || bytes.starts_with(b"#MAXDOOR")
    {
        return true;
    }
    let sample = &bytes[..bytes.len().min(16 * 1024)];
    if sample
        .iter()
        .take(64)
        .any(|byte| !byte.is_ascii() && *byte != 0)
    {
        return false;
    }
    String::from_utf8_lossy(sample)
        .lines()
        .take(512)
        .map(str::trim_start)
        .any(|line| {
            let line = line.to_ascii_lowercase();
            line.starts_with("newmodel ")
                || (line.starts_with("#max") && line.contains(" ascii"))
                || line.contains("walkmesh  ascii")
                || line.contains("pwkmesh  ascii")
                || line.contains("dwkmesh  ascii")
                || line.starts_with("beginwalkmeshgeom ")
        })
}

#[derive(Debug, Clone, Copy)]
struct ArrayDef {
    pointer: u32,
    used: usize,
}

#[derive(Debug, Clone)]
struct Controller {
    kind: u32,
    value_count: usize,
    time_start: usize,
    data_start: usize,
    columns: usize,
}

struct BinaryParser<'a> {
    bytes: &'a [u8],
    mdx_base: usize,
    diagnostics: Vec<MdlDiagnostic>,
    visited_nodes: BTreeSet<usize>,
}

impl<'a> BinaryParser<'a> {
    fn new(bytes: &'a [u8]) -> Result<Self, MdlError> {
        if bytes.len() < FILE_HEADER_SIZE + MODEL_HEADER_SIZE {
            return Err(MdlError::new(
                "MDL_BINARY_TRUNCATED",
                format!(
                    "expected at least {} bytes, found {}",
                    FILE_HEADER_SIZE + MODEL_HEADER_SIZE,
                    bytes.len()
                ),
                Some(bytes.len()),
            ));
        }
        let identifier = read_u32(bytes, 0)?;
        if identifier != 0 {
            return Err(MdlError::new(
                "MDL_BINARY_IDENTIFIER_INVALID",
                format!("expected binary identifier 0, found 0x{identifier:08X}"),
                Some(0),
            ));
        }
        let mdx_offset = read_u32(bytes, 4)? as usize;
        let mdx_base = FILE_HEADER_SIZE.checked_add(mdx_offset).ok_or_else(|| {
            MdlError::new("MDL_POINTER_OVERFLOW", "MDX base pointer overflow", Some(4))
        })?;
        if mdx_base > bytes.len() {
            return Err(MdlError::new(
                "MDL_MDX_POINTER_OUT_OF_RANGE",
                format!(
                    "MDX base 0x{mdx_base:X} exceeds file size 0x{:X}",
                    bytes.len()
                ),
                Some(4),
            ));
        }
        Ok(Self {
            bytes,
            mdx_base,
            diagnostics: Vec::new(),
            visited_nodes: BTreeSet::new(),
        })
    }

    fn parse(mut self) -> Result<MdlModel, MdlError> {
        let base = FILE_HEADER_SIZE;
        let name = read_c_string(self.bytes, base + 8, 64)?;
        let root_pointer = read_u32(self.bytes, base + 72)?;
        let declared_nodes = read_u32(self.bytes, base + 76)? as usize;
        let animation_def = read_array(self.bytes, base + 120)?;
        let bounds_min = read_vec3(self.bytes, base + 136)?;
        let bounds_max = read_vec3(self.bytes, base + 148)?;
        let radius = finite_or(read_f32(self.bytes, base + 160)?, 0.0);
        let model_scale = finite_or(read_f32(self.bytes, base + 164)?, 1.0);
        let supermodel_name = read_c_string(self.bytes, base + 168, 64)?;
        let supermodel = normalize_resource_name(&supermodel_name);

        let mut nodes = Vec::new();
        if root_pointer != 0 {
            let root = self.model_pointer(root_pointer, base + 72)?;
            self.parse_node(root, None, &mut nodes)?;
        }
        if declared_nodes != nodes.len() {
            self.warn(
                "MDL_NODE_COUNT_MISMATCH",
                format!(
                    "header declares {declared_nodes} nodes, parsed {}",
                    nodes.len()
                ),
                Some(base + 76),
                None,
            );
        }
        let animations = self.parse_animations(animation_def)?;
        self.diagnostics.extend(hierarchy_diagnostics(&nodes));
        let source_sha256 = format!("{:x}", Sha256::digest(self.bytes));
        Ok(MdlModel {
            format: MdlFormat::Binary,
            name,
            supermodel,
            bounds_min,
            bounds_max,
            radius,
            model_scale,
            nodes,
            animations,
            diagnostics: self.diagnostics,
            source_sha256,
        })
    }

    fn parse_node(
        &mut self,
        offset: usize,
        parent: Option<usize>,
        nodes: &mut Vec<MdlNode>,
    ) -> Result<Option<usize>, MdlError> {
        if nodes.len() >= MAX_NODES {
            return Err(MdlError::new(
                "MDL_NODE_LIMIT_EXCEEDED",
                format!("more than {MAX_NODES} nodes"),
                Some(offset),
            ));
        }
        if !self.visited_nodes.insert(offset) {
            self.warn(
                "MDL_NODE_CYCLE",
                "node pointer cycle ignored",
                Some(offset),
                None,
            );
            return Ok(None);
        }
        checked_slice(self.bytes, offset, NODE_HEADER_SIZE)?;
        let node_index = nodes.len();
        nodes.push(MdlNode::placeholder(parent));
        let node_number = read_u32(self.bytes, offset + 28)?;
        let name = read_c_string(self.bytes, offset + 32, 32)?;
        let children = read_array(self.bytes, offset + 72)?;
        let controller_keys = read_array(self.bytes, offset + 84)?;
        let controller_data = read_array(self.bytes, offset + 96)?;
        let content = read_u32(self.bytes, offset + 108)?;
        let mut kinds = node_kinds(content);
        let controllers = self.read_controllers(controller_keys, &name)?;
        let data = self.read_float_array(controller_data, "controller data", &name)?;
        let (translation, rotation, scale) = base_transform(&controllers, &data, self, &name);

        let mut tail = offset + NODE_HEADER_SIZE;
        tail = advance_if(content, 1 << 1, tail, 92, self.bytes.len(), "light", &name)?;
        tail = advance_if(
            content,
            1 << 2,
            tail,
            216,
            self.bytes.len(),
            "emitter",
            &name,
        )?;
        let reference_model = if content & (1 << 4) != 0 {
            checked_slice(self.bytes, tail, 68)?;
            let value = normalize_resource_name(&read_c_string(self.bytes, tail, 64)?);
            tail += 68;
            value
        } else {
            None
        };
        let mesh_header = if content & (1 << 5) != 0 {
            checked_slice(self.bytes, tail, MESH_HEADER_SIZE)?;
            let value = tail;
            tail += MESH_HEADER_SIZE;
            Some(value)
        } else {
            None
        };
        let skin_header = if content & (1 << 6) != 0 {
            checked_slice(self.bytes, tail, 100)?;
            let value = tail;
            tail += 100;
            Some(value)
        } else {
            None
        };
        tail = advance_if(
            content,
            1 << 7,
            tail,
            56,
            self.bytes.len(),
            "animmesh",
            &name,
        )?;
        tail = advance_if(
            content,
            1 << 8,
            tail,
            24,
            self.bytes.len(),
            "danglymesh",
            &name,
        )?;
        let aabb_header = if content & (1 << 9) != 0 {
            checked_slice(self.bytes, tail, 4)?;
            Some(tail)
        } else {
            None
        };

        let mesh = match mesh_header {
            Some(mesh_offset) => {
                match self.parse_mesh(mesh_offset, skin_header, aabb_header, &name) {
                    Ok(mesh) => Some(mesh),
                    Err(error) => {
                        self.warn(&error.code, error.message, error.offset, Some(name.clone()));
                        None
                    }
                }
            }
            None => None,
        };
        if mesh.is_some() && kinds.is_empty() {
            kinds.insert(MdlNodeKind::Trimesh);
        }

        nodes[node_index] = MdlNode {
            name: name.clone(),
            node_number,
            parent,
            children: Vec::new(),
            kinds,
            translation,
            rotation,
            scale,
            mesh,
            reference_model,
        };

        for child_pointer in self.read_pointer_array(children, "children", &name)? {
            let child_offset = match self.model_pointer(child_pointer, offset + 72) {
                Ok(value) => value,
                Err(error) => {
                    self.warn(&error.code, error.message, error.offset, Some(name.clone()));
                    continue;
                }
            };
            if let Some(child_index) = self.parse_node(child_offset, Some(node_index), nodes)? {
                nodes[node_index].children.push(child_index);
            }
        }
        Ok(Some(node_index))
    }

    fn parse_mesh(
        &mut self,
        offset: usize,
        skin_offset: Option<usize>,
        aabb_offset: Option<usize>,
        node: &str,
    ) -> Result<MdlMesh, MdlError> {
        let faces = read_array(self.bytes, offset + 8)?;
        if faces.used > MAX_FACES {
            return Err(MdlError::new(
                "MDL_FACE_LIMIT_EXCEEDED",
                format!("{} faces exceeds limit {MAX_FACES}", faces.used),
                Some(offset + 8),
            ));
        }
        let diffuse = read_vec3(self.bytes, offset + 60)?.map(clamp_color);
        let ambient = read_vec3(self.bytes, offset + 72)?.map(clamp_color);
        let specular = read_vec3(self.bytes, offset + 84)?.map(clamp_color);
        let shininess = finite_or(read_f32(self.bytes, offset + 96)?, 0.0).max(0.0);
        let render = read_u32(self.bytes, offset + 108)? != 0;
        let transparency_hint = read_u32(self.bytes, offset + 112)?;
        let tile_fade = read_u32(self.bytes, offset + 376)?;
        let mut textures = Vec::new();
        for texture_offset in [120, 184, 248, 312] {
            if let Some(texture) =
                normalize_resource_name(&read_c_string(self.bytes, offset + texture_offset, 64)?)
            {
                textures.push(texture);
            }
        }
        textures.sort();
        textures.dedup();

        let vertex_pointer = read_i32(self.bytes, offset + 444)?;
        let vertex_count = read_u16(self.bytes, offset + 448)? as usize;
        if vertex_count > MAX_VERTICES {
            return Err(MdlError::new(
                "MDL_VERTEX_LIMIT_EXCEEDED",
                format!("{vertex_count} vertices exceeds limit {MAX_VERTICES}"),
                Some(offset + 448),
            ));
        }
        let uv_pointer = read_i32(self.bytes, offset + 452)?;
        let normal_pointer = read_i32(self.bytes, offset + 468)?;
        let color_pointer = read_i32(self.bytes, offset + 472)?;
        let positions = self.read_mdx_vec3(vertex_pointer, vertex_count, "vertices", node)?;
        let mut normals = if normal_pointer >= 0 {
            self.read_mdx_vec3(normal_pointer, vertex_count, "normals", node)?
        } else {
            Vec::new()
        };
        let uv0 = if uv_pointer >= 0 {
            self.read_mdx_vec2(uv_pointer, vertex_count, "texture coordinates", node)?
        } else {
            Vec::new()
        };
        let colors = if color_pointer >= 0 {
            self.read_mdx_rgba(color_pointer, vertex_count, node)?
        } else {
            Vec::new()
        };
        let (indices, surface_ids) = self.read_faces(faces, vertex_count, node)?;
        if normals.len() != positions.len() {
            normals = calculate_normals(&positions, &indices);
            self.warn(
                "MDL_NORMALS_REBUILT",
                "vertex normals absent or incomplete; deterministic face normals generated",
                Some(offset + 468),
                Some(node.to_owned()),
            );
        }
        let skin = match skin_offset {
            Some(value) => self.parse_skin(value, vertex_count, node)?,
            None => None,
        };
        let walkmesh = aabb_offset.is_some();
        if let Some(aabb) = aabb_offset {
            let root = read_u32(self.bytes, aabb)?;
            if root == 0 {
                self.warn(
                    "MDL_AABB_EMPTY",
                    "walkmesh AABB node has no tree root",
                    Some(aabb),
                    Some(node.to_owned()),
                );
            }
        }
        Ok(MdlMesh {
            positions,
            normals,
            uv0,
            colors,
            indices,
            surface_ids,
            material: MdlMaterial {
                diffuse,
                ambient,
                specular,
                shininess,
                transparency_hint,
                textures,
                render,
                tile_fade,
            },
            skin,
            walkmesh,
        })
    }

    fn parse_skin(
        &mut self,
        offset: usize,
        vertex_count: usize,
        node: &str,
    ) -> Result<Option<MdlSkin>, MdlError> {
        let weight_pointer = read_i32(self.bytes, offset + 12)?;
        let bone_index_pointer = read_i32(self.bytes, offset + 16)?;
        let mapping_pointer = read_i32(self.bytes, offset + 20)?;
        let mapping_count = read_i32(self.bytes, offset + 24)?;
        if mapping_count < 0 || mapping_count as usize > MAX_NODES {
            self.warn(
                "MDL_SKIN_MAPPING_INVALID",
                format!("invalid bone mapping count {mapping_count}"),
                Some(offset + 24),
                Some(node.to_owned()),
            );
            return Ok(None);
        }
        let mapping_count = mapping_count as usize;
        let bone_mapping = if mapping_count > 0 && mapping_pointer >= 0 {
            let absolute = self.model_pointer(mapping_pointer as u32, offset + 20)?;
            (0..mapping_count)
                .map(|index| read_i16(self.bytes, absolute + index * 2))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            Vec::new()
        };
        if weight_pointer < 0 || bone_index_pointer < 0 || vertex_count == 0 {
            self.warn(
                "MDL_SKIN_DATA_MISSING",
                "skin header is present but weights or bone references are absent",
                Some(offset),
                Some(node.to_owned()),
            );
            return Ok(None);
        }
        let weights_offset = self.mdx_pointer(weight_pointer, offset + 12)?;
        let indices_offset = self.mdx_pointer(bone_index_pointer, offset + 16)?;
        checked_slice(self.bytes, weights_offset, vertex_count.saturating_mul(16))?;
        checked_slice(self.bytes, indices_offset, vertex_count.saturating_mul(8))?;
        let mut weights = Vec::with_capacity(vertex_count);
        let mut bone_indices = Vec::with_capacity(vertex_count);
        for index in 0..vertex_count {
            let weight = [
                finite_or(read_f32(self.bytes, weights_offset + index * 16)?, 0.0).max(0.0),
                finite_or(read_f32(self.bytes, weights_offset + index * 16 + 4)?, 0.0).max(0.0),
                finite_or(read_f32(self.bytes, weights_offset + index * 16 + 8)?, 0.0).max(0.0),
                finite_or(read_f32(self.bytes, weights_offset + index * 16 + 12)?, 0.0).max(0.0),
            ];
            let sum: f32 = weight.iter().sum();
            weights.push(if sum > f32::EPSILON {
                weight.map(|value| value / sum)
            } else {
                [1.0, 0.0, 0.0, 0.0]
            });
            bone_indices.push([
                read_i16(self.bytes, indices_offset + index * 8)?.max(0) as u16,
                read_i16(self.bytes, indices_offset + index * 8 + 2)?.max(0) as u16,
                read_i16(self.bytes, indices_offset + index * 8 + 4)?.max(0) as u16,
                read_i16(self.bytes, indices_offset + index * 8 + 6)?.max(0) as u16,
            ]);
        }
        Ok(Some(MdlSkin {
            weights,
            bone_indices,
            bone_mapping,
        }))
    }

    fn read_faces(
        &mut self,
        definition: ArrayDef,
        vertex_count: usize,
        node: &str,
    ) -> Result<(Vec<u32>, Vec<i32>), MdlError> {
        if definition.used == 0 {
            return Ok((Vec::new(), Vec::new()));
        }
        let offset = self.model_pointer(definition.pointer, definition.pointer as usize)?;
        checked_slice(self.bytes, offset, definition.used.saturating_mul(32))?;
        let mut indices = Vec::with_capacity(definition.used * 3);
        let mut surface_ids = Vec::with_capacity(definition.used);
        for face in 0..definition.used {
            let face_offset = offset + face * 32;
            let values = [
                read_u16(self.bytes, face_offset + 26)? as u32,
                read_u16(self.bytes, face_offset + 28)? as u32,
                read_u16(self.bytes, face_offset + 30)? as u32,
            ];
            if values.iter().any(|value| *value as usize >= vertex_count) {
                self.warn(
                    "MDL_FACE_VERTEX_OUT_OF_RANGE",
                    format!("face {face} references {values:?} with {vertex_count} vertices"),
                    Some(face_offset + 26),
                    Some(node.to_owned()),
                );
                continue;
            }
            indices.extend(values);
            surface_ids.push(read_i32(self.bytes, face_offset + 16)?);
        }
        Ok((indices, surface_ids))
    }

    fn parse_animations(&mut self, definition: ArrayDef) -> Result<Vec<MdlAnimation>, MdlError> {
        let pointers = self.read_pointer_array(definition, "animations", "model")?;
        let mut animations = Vec::new();
        for pointer in pointers {
            let offset = match self.model_pointer(pointer, definition.pointer as usize) {
                Ok(value) => value,
                Err(error) => {
                    self.warn(&error.code, error.message, error.offset, None);
                    continue;
                }
            };
            checked_slice(self.bytes, offset, GEOMETRY_HEADER_SIZE + 84)?;
            let name = read_c_string(self.bytes, offset + 8, 64)?;
            let root_pointer = read_u32(self.bytes, offset + 72)?;
            let length = finite_or(read_f32(self.bytes, offset + 112)?, 0.0).max(0.0);
            let transition = finite_or(read_f32(self.bytes, offset + 116)?, 0.0).max(0.0);
            let root_node = read_c_string(self.bytes, offset + 120, 64)?;
            let events = self.read_events(read_array(self.bytes, offset + 184)?, &name)?;
            let mut tracks = Vec::new();
            let mut visited = BTreeSet::new();
            if root_pointer != 0 {
                let root = self.model_pointer(root_pointer, offset + 72)?;
                self.collect_animation_tracks(root, &mut visited, &mut tracks, &name)?;
            }
            tracks.sort_by(|left, right| {
                left.node
                    .cmp(&right.node)
                    .then_with(|| left.path.cmp(&right.path))
            });
            animations.push(MdlAnimation {
                name,
                root_node,
                length,
                transition,
                events,
                tracks,
            });
        }
        animations.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(animations)
    }

    fn collect_animation_tracks(
        &mut self,
        offset: usize,
        visited: &mut BTreeSet<usize>,
        tracks: &mut Vec<AnimationTrack>,
        animation: &str,
    ) -> Result<(), MdlError> {
        if visited.len() >= MAX_NODES || !visited.insert(offset) {
            return Ok(());
        }
        checked_slice(self.bytes, offset, NODE_HEADER_SIZE)?;
        let name = read_c_string(self.bytes, offset + 32, 32)?;
        let children = read_array(self.bytes, offset + 72)?;
        let keys = read_array(self.bytes, offset + 84)?;
        let data_definition = read_array(self.bytes, offset + 96)?;
        let controllers = self.read_controllers(keys, &name)?;
        let data = self.read_float_array(data_definition, "animation controller data", &name)?;
        for controller in controllers {
            if let Some(track) = controller_track(&controller, &data, &name, self, animation) {
                tracks.push(track);
            }
        }
        for pointer in self.read_pointer_array(children, "animation children", &name)? {
            match self.model_pointer(pointer, offset + 72) {
                Ok(child) => self.collect_animation_tracks(child, visited, tracks, animation)?,
                Err(error) => {
                    self.warn(&error.code, error.message, error.offset, Some(name.clone()))
                }
            }
        }
        Ok(())
    }

    fn read_events(
        &mut self,
        definition: ArrayDef,
        animation: &str,
    ) -> Result<Vec<AnimationEvent>, MdlError> {
        if definition.used == 0 {
            return Ok(Vec::new());
        }
        let offset = self.model_pointer(definition.pointer, definition.pointer as usize)?;
        checked_slice(self.bytes, offset, definition.used.saturating_mul(36))?;
        let mut events = Vec::with_capacity(definition.used);
        for index in 0..definition.used {
            let entry = offset + index * 36;
            events.push(AnimationEvent {
                time: finite_or(read_f32(self.bytes, entry)?, 0.0),
                name: read_c_string(self.bytes, entry + 4, 32)?,
            });
        }
        events.sort_by(|left, right| {
            left.time
                .total_cmp(&right.time)
                .then_with(|| left.name.cmp(&right.name))
        });
        if events.iter().any(|event| event.name.is_empty()) {
            self.warn(
                "MDL_ANIMATION_EVENT_EMPTY",
                format!("animation {animation} contains an unnamed event"),
                Some(offset),
                None,
            );
        }
        Ok(events)
    }

    fn read_controllers(
        &mut self,
        definition: ArrayDef,
        node: &str,
    ) -> Result<Vec<Controller>, MdlError> {
        if definition.used == 0 {
            return Ok(Vec::new());
        }
        if definition.used > MAX_ARRAY_ENTRIES {
            return Err(MdlError::new(
                "MDL_CONTROLLER_LIMIT_EXCEEDED",
                format!("{} controllers exceeds limit", definition.used),
                Some(definition.pointer as usize),
            ));
        }
        let offset = self.model_pointer(definition.pointer, definition.pointer as usize)?;
        checked_slice(self.bytes, offset, definition.used.saturating_mul(12))?;
        let mut controllers = Vec::with_capacity(definition.used);
        for index in 0..definition.used {
            let entry = offset + index * 12;
            let raw_columns = self.bytes[entry + 10] as usize;
            let columns = raw_columns & 0x0f;
            if columns == 0 || columns > 4 {
                self.warn(
                    "MDL_CONTROLLER_COLUMNS_UNSUPPORTED",
                    format!("controller on {node} declares {raw_columns} columns"),
                    Some(entry + 10),
                    Some(node.to_owned()),
                );
            }
            controllers.push(Controller {
                kind: read_u32(self.bytes, entry)?,
                value_count: read_u16(self.bytes, entry + 4)? as usize,
                time_start: read_u16(self.bytes, entry + 6)? as usize,
                data_start: read_u16(self.bytes, entry + 8)? as usize,
                columns,
            });
        }
        Ok(controllers)
    }

    fn read_float_array(
        &mut self,
        definition: ArrayDef,
        label: &str,
        node: &str,
    ) -> Result<Vec<f32>, MdlError> {
        if definition.used == 0 {
            return Ok(Vec::new());
        }
        if definition.used > MAX_ARRAY_ENTRIES {
            return Err(MdlError::new(
                "MDL_ARRAY_LIMIT_EXCEEDED",
                format!("{label} contains {} entries", definition.used),
                Some(definition.pointer as usize),
            ));
        }
        let offset = self.model_pointer(definition.pointer, definition.pointer as usize)?;
        checked_slice(self.bytes, offset, definition.used.saturating_mul(4))?;
        let mut values = Vec::with_capacity(definition.used);
        for index in 0..definition.used {
            let value = read_f32(self.bytes, offset + index * 4)?;
            if value.is_finite() {
                values.push(value);
            } else {
                values.push(0.0);
                self.warn(
                    "MDL_FLOAT_NON_FINITE",
                    format!("non-finite value in {label}"),
                    Some(offset + index * 4),
                    Some(node.to_owned()),
                );
            }
        }
        Ok(values)
    }

    fn read_pointer_array(
        &mut self,
        definition: ArrayDef,
        label: &str,
        node: &str,
    ) -> Result<Vec<u32>, MdlError> {
        if definition.used == 0 {
            return Ok(Vec::new());
        }
        if definition.used > MAX_NODES {
            return Err(MdlError::new(
                "MDL_POINTER_ARRAY_LIMIT_EXCEEDED",
                format!("{label} on {node} contains {} entries", definition.used),
                Some(definition.pointer as usize),
            ));
        }
        let offset = self.model_pointer(definition.pointer, definition.pointer as usize)?;
        checked_slice(self.bytes, offset, definition.used.saturating_mul(4))?;
        (0..definition.used)
            .map(|index| read_u32(self.bytes, offset + index * 4))
            .collect()
    }

    fn read_mdx_vec3(
        &self,
        pointer: i32,
        count: usize,
        label: &str,
        node: &str,
    ) -> Result<Vec<[f32; 3]>, MdlError> {
        if count == 0 {
            return Ok(Vec::new());
        }
        let offset = self.mdx_pointer(pointer, 0)?;
        checked_slice(self.bytes, offset, count.saturating_mul(12)).map_err(|mut error| {
            error.message = format!("{label} for node {node}: {}", error.message);
            error
        })?;
        (0..count)
            .map(|index| read_vec3(self.bytes, offset + index * 12))
            .collect()
    }

    fn read_mdx_vec2(
        &self,
        pointer: i32,
        count: usize,
        label: &str,
        node: &str,
    ) -> Result<Vec<[f32; 2]>, MdlError> {
        let offset = self.mdx_pointer(pointer, 0)?;
        checked_slice(self.bytes, offset, count.saturating_mul(8)).map_err(|mut error| {
            error.message = format!("{label} for node {node}: {}", error.message);
            error
        })?;
        (0..count)
            .map(|index| {
                Ok([
                    finite_or(read_f32(self.bytes, offset + index * 8)?, 0.0),
                    finite_or(read_f32(self.bytes, offset + index * 8 + 4)?, 0.0),
                ])
            })
            .collect()
    }

    fn read_mdx_rgba(
        &self,
        pointer: i32,
        count: usize,
        node: &str,
    ) -> Result<Vec<[u8; 4]>, MdlError> {
        let offset = self.mdx_pointer(pointer, 0)?;
        let slice =
            checked_slice(self.bytes, offset, count.saturating_mul(4)).map_err(|mut error| {
                error.message = format!("vertex colors for node {node}: {}", error.message);
                error
            })?;
        Ok(slice
            .chunks_exact(4)
            .map(|value| [value[0], value[1], value[2], value[3]])
            .collect())
    }

    fn model_pointer(&self, pointer: u32, source: usize) -> Result<usize, MdlError> {
        let absolute = FILE_HEADER_SIZE
            .checked_add(pointer as usize)
            .ok_or_else(|| {
                MdlError::new(
                    "MDL_POINTER_OVERFLOW",
                    "model pointer overflow",
                    Some(source),
                )
            })?;
        if absolute >= self.bytes.len() {
            return Err(MdlError::new(
                "MDL_POINTER_OUT_OF_RANGE",
                format!(
                    "pointer 0x{absolute:X} exceeds file size 0x{:X}",
                    self.bytes.len()
                ),
                Some(source),
            ));
        }
        Ok(absolute)
    }

    fn mdx_pointer(&self, pointer: i32, source: usize) -> Result<usize, MdlError> {
        if pointer < 0 {
            return Err(MdlError::new(
                "MDL_MDX_POINTER_ABSENT",
                "negative MDX pointer",
                Some(source),
            ));
        }
        let absolute = self.mdx_base.checked_add(pointer as usize).ok_or_else(|| {
            MdlError::new("MDL_POINTER_OVERFLOW", "MDX pointer overflow", Some(source))
        })?;
        if absolute >= self.bytes.len() {
            return Err(MdlError::new(
                "MDL_MDX_POINTER_OUT_OF_RANGE",
                format!(
                    "pointer 0x{absolute:X} exceeds file size 0x{:X}",
                    self.bytes.len()
                ),
                Some(source),
            ));
        }
        Ok(absolute)
    }

    fn warn(
        &mut self,
        code: &str,
        message: impl Into<String>,
        offset: Option<usize>,
        node: Option<String>,
    ) {
        self.diagnostics.push(MdlDiagnostic {
            code: code.to_owned(),
            message: message.into(),
            offset,
            node,
        });
    }
}

fn controller_track(
    controller: &Controller,
    data: &[f32],
    node: &str,
    parser: &mut BinaryParser<'_>,
    animation: &str,
) -> Option<AnimationTrack> {
    let (path, expected_columns) = match controller.kind {
        8 => (TrackPath::Translation, 3),
        20 => (TrackPath::Rotation, 4),
        36 => (TrackPath::Scale, 1),
        _ => return None,
    };
    let columns = if controller.columns == 0 {
        expected_columns
    } else {
        controller.columns
    };
    if columns != expected_columns || controller.value_count == 0 {
        parser.warn(
            "MDL_CONTROLLER_SHAPE_UNSUPPORTED",
            format!(
                "animation {animation}, node {node}, controller {} has {} values × {columns} columns",
                controller.kind, controller.value_count
            ),
            None,
            Some(node.to_owned()),
        );
        return None;
    }
    let value_end = controller
        .data_start
        .checked_add(controller.value_count.checked_mul(columns)?)?;
    if value_end > data.len() {
        parser.warn(
            "MDL_CONTROLLER_DATA_OUT_OF_RANGE",
            format!("animation {animation}, node {node}, values exceed controller data"),
            None,
            Some(node.to_owned()),
        );
        return None;
    }
    let times = if controller.value_count == 1 {
        vec![0.0]
    } else {
        let end = controller.time_start.checked_add(controller.value_count)?;
        if end > data.len() {
            parser.warn(
                "MDL_CONTROLLER_TIME_OUT_OF_RANGE",
                format!("animation {animation}, node {node}, time keys exceed controller data"),
                None,
                Some(node.to_owned()),
            );
            return None;
        }
        data[controller.time_start..end].to_vec()
    };
    let mut values = Vec::with_capacity(controller.value_count);
    for index in 0..controller.value_count {
        let start = controller.data_start + index * columns;
        values.push(match path {
            TrackPath::Translation => [data[start], data[start + 1], data[start + 2], 0.0],
            TrackPath::Rotation => normalize_quaternion([
                data[start],
                data[start + 1],
                data[start + 2],
                data[start + 3],
            ]),
            TrackPath::Scale => [data[start], data[start], data[start], 0.0],
        });
    }
    Some(AnimationTrack {
        node: node.to_owned(),
        path,
        times,
        values,
    })
}

fn base_transform(
    controllers: &[Controller],
    data: &[f32],
    parser: &mut BinaryParser<'_>,
    node: &str,
) -> ([f32; 3], [f32; 4], [f32; 3]) {
    let mut translation = [0.0, 0.0, 0.0];
    let mut rotation = [0.0, 0.0, 0.0, 1.0];
    let mut scale = [1.0, 1.0, 1.0];
    for controller in controllers {
        let Some(track) = controller_track(controller, data, node, parser, "base") else {
            continue;
        };
        let Some(value) = track.values.first().copied() else {
            continue;
        };
        match track.path {
            TrackPath::Translation => translation = [value[0], value[1], value[2]],
            TrackPath::Rotation => rotation = value,
            TrackPath::Scale => scale = [value[0], value[1], value[2]],
        }
    }
    (translation, rotation, scale)
}

fn node_kinds(content: u32) -> BTreeSet<MdlNodeKind> {
    let mut kinds = BTreeSet::new();
    if content & (1 << 1) != 0 {
        kinds.insert(MdlNodeKind::Light);
    }
    if content & (1 << 2) != 0 {
        kinds.insert(MdlNodeKind::Emitter);
    }
    if content & (1 << 3) != 0 {
        kinds.insert(MdlNodeKind::Camera);
    }
    if content & (1 << 4) != 0 {
        kinds.insert(MdlNodeKind::Reference);
    }
    if content & (1 << 5) != 0 {
        kinds.insert(MdlNodeKind::Trimesh);
    }
    if content & (1 << 6) != 0 {
        kinds.insert(MdlNodeKind::Skin);
    }
    if content & (1 << 7) != 0 {
        kinds.insert(MdlNodeKind::Animmesh);
    }
    if content & (1 << 8) != 0 {
        kinds.insert(MdlNodeKind::Danglymesh);
    }
    if content & (1 << 9) != 0 {
        kinds.insert(MdlNodeKind::Aabb);
    }
    if kinds.is_empty() {
        kinds.insert(MdlNodeKind::Dummy);
    }
    kinds
}

fn advance_if(
    content: u32,
    flag: u32,
    offset: usize,
    amount: usize,
    file_size: usize,
    label: &str,
    node: &str,
) -> Result<usize, MdlError> {
    if content & flag == 0 {
        return Ok(offset);
    }
    let end = offset.checked_add(amount).ok_or_else(|| {
        MdlError::new(
            "MDL_POINTER_OVERFLOW",
            format!("{label} header overflow on {node}"),
            Some(offset),
        )
    })?;
    if end > file_size {
        return Err(MdlError::new(
            "MDL_NODE_HEADER_TRUNCATED",
            format!("{label} header on {node} exceeds file bounds"),
            Some(offset),
        ));
    }
    Ok(end)
}

fn parse_ascii(bytes: &[u8]) -> Result<MdlModel, MdlError> {
    let text = String::from_utf8_lossy(bytes);
    let mut model = MdlModel {
        format: MdlFormat::Ascii,
        name: String::new(),
        supermodel: None,
        bounds_min: [0.0, 0.0, 0.0],
        bounds_max: [0.0, 0.0, 0.0],
        radius: 0.0,
        model_scale: 1.0,
        nodes: Vec::new(),
        animations: Vec::new(),
        diagnostics: Vec::new(),
        source_sha256: format!("{:x}", Sha256::digest(bytes)),
    };
    let lines = text.lines().map(str::trim).collect::<Vec<_>>();
    let mut index = 0;
    let mut parents = Vec::<Option<String>>::new();
    while index < lines.len() {
        let words = words(lines[index]);
        if words.is_empty() || words[0].starts_with('#') {
            index += 1;
            continue;
        }
        match words[0].to_ascii_lowercase().as_str() {
            "newmodel" if words.len() > 1 => model.name = words[1].to_owned(),
            "beginwalkmeshgeom" if words.len() > 1 => model.name = words[1].to_owned(),
            "setsupermodel" if words.len() > 2 => {
                model.supermodel = normalize_resource_name(words[2]);
            }
            "setanimationscale" if words.len() > 1 => {
                model.model_scale = parse_float(words[1]).unwrap_or(1.0);
            }
            "node" if words.len() > 2 => {
                let (node, parent, next) = parse_ascii_node(&lines, index, &mut model.diagnostics)?;
                model.nodes.push(node);
                parents.push(parent);
                index = next;
                continue;
            }
            "newanim" if words.len() > 1 => {
                let (animation, next) = parse_ascii_animation(&lines, index)?;
                model.animations.push(animation);
                index = next;
                continue;
            }
            _ => {}
        }
        index += 1;
    }
    let names = model
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.name.to_ascii_lowercase(), index))
        .collect::<BTreeMap<_, _>>();
    for (node_index, parent_name) in parents.into_iter().enumerate() {
        if let Some(parent_name) = parent_name
            && let Some(parent_index) = names.get(&parent_name.to_ascii_lowercase()).copied()
        {
            model.nodes[node_index].parent = Some(parent_index);
            model.nodes[parent_index].children.push(node_index);
        }
    }
    model
        .diagnostics
        .extend(hierarchy_diagnostics(&model.nodes));
    derive_bounds(&mut model);
    Ok(model)
}

fn hierarchy_diagnostics(nodes: &[MdlNode]) -> Vec<MdlDiagnostic> {
    let mut diagnostics = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        let Some(parent) = node.parent else {
            continue;
        };
        if parent >= nodes.len() {
            diagnostics.push(MdlDiagnostic {
                code: "MDL_NODE_PARENT_OUT_OF_RANGE".to_owned(),
                message: format!(
                    "node {} parent {parent} is outside the node table",
                    node.name
                ),
                offset: None,
                node: Some(node.name.clone()),
            });
            continue;
        }
        let mut seen = BTreeSet::from([index]);
        let mut current = Some(parent);
        while let Some(candidate) = current {
            if !seen.insert(candidate) {
                diagnostics.push(MdlDiagnostic {
                    code: if candidate == index {
                        "MDL_NODE_SELF_PARENT"
                    } else {
                        "MDL_NODE_PARENT_CYCLE"
                    }
                    .to_owned(),
                    message: format!("node {} parent hierarchy contains a cycle", node.name),
                    offset: None,
                    node: Some(node.name.clone()),
                });
                break;
            }
            current = nodes.get(candidate).and_then(|value| value.parent);
        }
    }
    diagnostics
}

fn parse_ascii_node(
    lines: &[&str],
    start: usize,
    diagnostics: &mut Vec<MdlDiagnostic>,
) -> Result<(MdlNode, Option<String>, usize), MdlError> {
    let header = words(lines[start]);
    let kind = ascii_node_kind(header[1]);
    let mut node = MdlNode::placeholder(None);
    node.name = header[2].to_owned();
    node.node_number = start as u32;
    node.kinds.insert(kind);
    let mut parent = None;
    let mut mesh = MdlMesh::default();
    let mut has_mesh = false;
    let mut tverts = Vec::new();
    let mut index = start + 1;
    while index < lines.len() {
        let values = words(lines[index]);
        if values.is_empty() || values[0].starts_with('#') {
            index += 1;
            continue;
        }
        let key = values[0].to_ascii_lowercase();
        if key == "endnode" {
            break;
        }
        match key.as_str() {
            "parent" if values.len() > 1 && !values[1].eq_ignore_ascii_case("null") => {
                parent = Some(values[1].to_owned());
            }
            "position" if values.len() >= 4 => {
                node.translation = parse_vec3_words(&values[1..4]);
            }
            "orientation" if values.len() >= 5 => {
                let axis = parse_vec3_words(&values[1..4]);
                let angle = parse_float(values[4]).unwrap_or(0.0);
                node.rotation = axis_angle(axis, angle);
            }
            "scale" if values.len() >= 2 => {
                let value = parse_float(values[1]).unwrap_or(1.0);
                node.scale = [value, value, value];
            }
            "bitmap" | "texture0" if values.len() >= 2 => {
                if let Some(texture) = normalize_resource_name(values[1]) {
                    mesh.material.textures.push(texture);
                }
            }
            "diffuse" if values.len() >= 4 => {
                mesh.material.diffuse = parse_vec3_words(&values[1..4]).map(clamp_color);
            }
            "ambient" if values.len() >= 4 => {
                mesh.material.ambient = parse_vec3_words(&values[1..4]).map(clamp_color);
            }
            "specular" if values.len() >= 4 => {
                mesh.material.specular = parse_vec3_words(&values[1..4]).map(clamp_color);
            }
            "shininess" if values.len() >= 2 => {
                mesh.material.shininess = parse_float(values[1]).unwrap_or(0.0).max(0.0);
            }
            "render" if values.len() >= 2 => mesh.material.render = values[1] != "0",
            "tilefade" if values.len() >= 2 => {
                mesh.material.tile_fade = values[1].parse::<u32>().unwrap_or_default();
            }
            "verts" if values.len() >= 2 => {
                let count = parse_count(values[1], MAX_VERTICES, start)?;
                let end = (index + 1).saturating_add(count).min(lines.len());
                for line in &lines[index + 1..end] {
                    let values = words(line);
                    if values.len() >= 3 {
                        mesh.positions.push(parse_vec3_words(&values[..3]));
                    }
                }
                index = end.saturating_sub(1);
                has_mesh = true;
            }
            "normals" if values.len() >= 2 => {
                let count = parse_count(values[1], MAX_VERTICES, start)?;
                let end = (index + 1).saturating_add(count).min(lines.len());
                for line in &lines[index + 1..end] {
                    let values = words(line);
                    if values.len() >= 3 {
                        mesh.normals.push(parse_vec3_words(&values[..3]));
                    }
                }
                index = end.saturating_sub(1);
            }
            "tverts" if values.len() >= 2 => {
                let count = parse_count(values[1], MAX_VERTICES, start)?;
                let end = (index + 1).saturating_add(count).min(lines.len());
                for line in &lines[index + 1..end] {
                    let values = words(line);
                    if values.len() >= 2 {
                        tverts.push([
                            parse_float(values[0]).unwrap_or(0.0),
                            parse_float(values[1]).unwrap_or(0.0),
                        ]);
                    }
                }
                index = end.saturating_sub(1);
            }
            "faces" if values.len() >= 2 => {
                let count = parse_count(values[1], MAX_FACES, start)?;
                let end = (index + 1).saturating_add(count).min(lines.len());
                for (face, line) in lines[index + 1..end].iter().enumerate() {
                    let values = words(line);
                    if values.len() >= 3 {
                        let face_indices = [
                            values[0].parse::<u32>().ok(),
                            values[1].parse::<u32>().ok(),
                            values[2].parse::<u32>().ok(),
                        ];
                        if face_indices.iter().all(Option::is_some) {
                            mesh.indices.extend(face_indices.into_iter().flatten());
                            mesh.surface_ids.push(
                                values
                                    .get(3)
                                    .and_then(|value| value.parse::<i32>().ok())
                                    .unwrap_or_default(),
                            );
                        } else {
                            diagnostics.push(MdlDiagnostic {
                                code: "MDL_ASCII_FACE_INVALID".to_owned(),
                                message: format!("invalid face {face} on {}", node.name),
                                offset: Some(index + 1 + face),
                                node: Some(node.name.clone()),
                            });
                        }
                    }
                }
                index = end.saturating_sub(1);
            }
            "refmodel" if values.len() >= 2 => {
                node.reference_model = normalize_resource_name(values[1]);
            }
            _ => {}
        }
        index += 1;
    }
    if has_mesh {
        mesh.uv0 = tverts;
        if mesh.normals.len() != mesh.positions.len() {
            mesh.normals = calculate_normals(&mesh.positions, &mesh.indices);
        }
        mesh.material.textures.sort();
        mesh.material.textures.dedup();
        mesh.walkmesh = node.kinds.contains(&MdlNodeKind::Aabb);
        node.mesh = Some(mesh);
    }
    Ok((node, parent, index.saturating_add(1)))
}

fn parse_ascii_animation(lines: &[&str], start: usize) -> Result<(MdlAnimation, usize), MdlError> {
    let header = words(lines[start]);
    let mut animation = MdlAnimation {
        name: header[1].to_owned(),
        ..MdlAnimation::default()
    };
    let mut index = start + 1;
    while index < lines.len() {
        let values = words(lines[index]);
        if values.is_empty() {
            index += 1;
            continue;
        }
        match values[0].to_ascii_lowercase().as_str() {
            "doneanim" => break,
            "length" if values.len() > 1 => {
                animation.length = parse_float(values[1]).unwrap_or(0.0).max(0.0)
            }
            "transtime" if values.len() > 1 => {
                animation.transition = parse_float(values[1]).unwrap_or(0.0).max(0.0)
            }
            "animroot" if values.len() > 1 => animation.root_node = values[1].to_owned(),
            "event" if values.len() > 2 => animation.events.push(AnimationEvent {
                time: parse_float(values[1]).unwrap_or(0.0),
                name: values[2].to_owned(),
            }),
            _ => {}
        }
        index += 1;
    }
    Ok((animation, index.saturating_add(1)))
}

fn ascii_node_kind(value: &str) -> MdlNodeKind {
    match value.to_ascii_lowercase().as_str() {
        "light" => MdlNodeKind::Light,
        "emitter" => MdlNodeKind::Emitter,
        "camera" => MdlNodeKind::Camera,
        "reference" => MdlNodeKind::Reference,
        "trimesh" => MdlNodeKind::Trimesh,
        "skin" => MdlNodeKind::Skin,
        "animmesh" => MdlNodeKind::Animmesh,
        "danglymesh" => MdlNodeKind::Danglymesh,
        "aabb" => MdlNodeKind::Aabb,
        _ => MdlNodeKind::Dummy,
    }
}

fn derive_bounds(model: &mut MdlModel) {
    let mut minimum = [f32::INFINITY; 3];
    let mut maximum = [f32::NEG_INFINITY; 3];
    let mut found = false;
    for position in model
        .nodes
        .iter()
        .filter_map(|node| node.mesh.as_ref())
        .flat_map(|mesh| mesh.positions.iter())
    {
        found = true;
        for axis in 0..3 {
            minimum[axis] = minimum[axis].min(position[axis]);
            maximum[axis] = maximum[axis].max(position[axis]);
        }
    }
    if found {
        model.bounds_min = minimum;
        model.bounds_max = maximum;
        model.radius = maximum
            .iter()
            .zip(minimum)
            .map(|(maximum, minimum)| maximum.abs().max(minimum.abs()).powi(2))
            .sum::<f32>()
            .sqrt();
    }
}

fn calculate_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = vec![[0.0, 0.0, 0.0]; positions.len()];
    for face in indices.chunks_exact(3) {
        let Some(a) = positions.get(face[0] as usize) else {
            continue;
        };
        let Some(b) = positions.get(face[1] as usize) else {
            continue;
        };
        let Some(c) = positions.get(face[2] as usize) else {
            continue;
        };
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let normal = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        for vertex in face {
            if let Some(target) = normals.get_mut(*vertex as usize) {
                for axis in 0..3 {
                    target[axis] += normal[axis];
                }
            }
        }
    }
    normals.into_iter().map(normalize_vec3).collect()
}

fn normalize_vec3(value: [f32; 3]) -> [f32; 3] {
    let length = value.iter().map(|value| value * value).sum::<f32>().sqrt();
    if length > f32::EPSILON {
        value.map(|value| value / length)
    } else {
        [0.0, 1.0, 0.0]
    }
}

fn normalize_quaternion(value: [f32; 4]) -> [f32; 4] {
    let length = value.iter().map(|value| value * value).sum::<f32>().sqrt();
    if length > f32::EPSILON {
        value.map(|value| value / length)
    } else {
        [0.0, 0.0, 0.0, 1.0]
    }
}

fn axis_angle(axis: [f32; 3], angle: f32) -> [f32; 4] {
    let axis = normalize_vec3(axis);
    let half = angle * 0.5;
    let sine = half.sin();
    normalize_quaternion([axis[0] * sine, axis[1] * sine, axis[2] * sine, half.cos()])
}

fn normalize_resource_name(value: &str) -> Option<String> {
    let value = value.trim_matches(char::from(0)).trim();
    if value.is_empty() || value.eq_ignore_ascii_case("null") || value.eq_ignore_ascii_case("none")
    {
        None
    } else {
        Some(value.to_ascii_lowercase())
    }
}

fn parse_count(value: &str, maximum: usize, line: usize) -> Result<usize, MdlError> {
    let count = value.parse::<usize>().map_err(|_| {
        MdlError::new(
            "MDL_ASCII_COUNT_INVALID",
            format!("invalid count {value:?}"),
            Some(line),
        )
    })?;
    if count > maximum {
        return Err(MdlError::new(
            "MDL_ASCII_COUNT_LIMIT_EXCEEDED",
            format!("count {count} exceeds {maximum}"),
            Some(line),
        ));
    }
    Ok(count)
}

fn words(value: &str) -> Vec<&str> {
    value.split_whitespace().collect()
}

fn parse_float(value: &str) -> Option<f32> {
    value.parse::<f32>().ok().filter(|value| value.is_finite())
}

fn parse_vec3_words(values: &[&str]) -> [f32; 3] {
    [
        values
            .first()
            .and_then(|value| parse_float(value))
            .unwrap_or(0.0),
        values
            .get(1)
            .and_then(|value| parse_float(value))
            .unwrap_or(0.0),
        values
            .get(2)
            .and_then(|value| parse_float(value))
            .unwrap_or(0.0),
    ]
}

fn clamp_color(value: f32) -> f32 {
    finite_or(value, 0.0).clamp(0.0, 1.0)
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

fn read_array(bytes: &[u8], offset: usize) -> Result<ArrayDef, MdlError> {
    let pointer = read_u32(bytes, offset)?;
    let used = read_u32(bytes, offset + 4)? as usize;
    let allocated = read_u32(bytes, offset + 8)? as usize;
    if used > allocated && allocated != 0 {
        return Err(MdlError::new(
            "MDL_ARRAY_COUNT_INVALID",
            format!("used count {used} exceeds allocated count {allocated}"),
            Some(offset),
        ));
    }
    Ok(ArrayDef { pointer, used })
}

fn read_vec3(bytes: &[u8], offset: usize) -> Result<[f32; 3], MdlError> {
    Ok([
        finite_or(read_f32(bytes, offset)?, 0.0),
        finite_or(read_f32(bytes, offset + 4)?, 0.0),
        finite_or(read_f32(bytes, offset + 8)?, 0.0),
    ])
}

fn read_c_string(bytes: &[u8], offset: usize, length: usize) -> Result<String, MdlError> {
    let value = checked_slice(bytes, offset, length)?;
    let end = value
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(value.len());
    Ok(String::from_utf8_lossy(&value[..end]).trim().to_owned())
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, MdlError> {
    let value = checked_slice(bytes, offset, 2)?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_i16(bytes: &[u8], offset: usize) -> Result<i16, MdlError> {
    Ok(read_u16(bytes, offset)? as i16)
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, MdlError> {
    let value = checked_slice(bytes, offset, 4)?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

fn read_i32(bytes: &[u8], offset: usize) -> Result<i32, MdlError> {
    Ok(read_u32(bytes, offset)? as i32)
}

fn read_f32(bytes: &[u8], offset: usize) -> Result<f32, MdlError> {
    Ok(f32::from_bits(read_u32(bytes, offset)?))
}

fn checked_slice(bytes: &[u8], offset: usize, length: usize) -> Result<&[u8], MdlError> {
    let end = offset
        .checked_add(length)
        .ok_or_else(|| MdlError::new("MDL_POINTER_OVERFLOW", "slice end overflow", Some(offset)))?;
    bytes.get(offset..end).ok_or_else(|| {
        MdlError::new(
            "MDL_DATA_TRUNCATED",
            format!(
                "range 0x{offset:X}..0x{end:X} exceeds file size 0x{:X}",
                bytes.len()
            ),
            Some(offset),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ascii_triangle_without_external_tools() {
        let model = parse_mdl(
            b"newmodel triangle\nnode trimesh body\nparent null\nverts 3\n0 0 0\n1 0 0\n0 1 0\ntverts 3\n0 0\n1 0\n0 1\nfaces 1\n0 1 2 0 0 1 2 0\nbitmap stone\nendnode\ndonemodel triangle\n",
        )
        .expect("ASCII model");
        assert_eq!(model.format, MdlFormat::Ascii);
        assert_eq!(model.nodes.len(), 1);
        let mesh = model.nodes[0].mesh.as_ref().expect("mesh");
        assert_eq!(mesh.positions.len(), 3);
        assert_eq!(mesh.indices, [0, 1, 2]);
        assert_eq!(mesh.surface_ids, [0]);
        assert_eq!(mesh.material.textures, ["stone"]);
    }

    #[test]
    fn detects_ascii_model_after_exporter_comments() {
        let model = parse_mdl(b"# Exported by NeverBlender\n#MAXMODEL ASCII\nnewmodel comment\n")
            .expect("commented ASCII model");
        assert_eq!(model.format, MdlFormat::Ascii);
        assert_eq!(model.name, "comment");
    }

    #[test]
    fn parses_standalone_ascii_walkmesh_grammars() {
        let model = parse_mdl(
            b"# Exported from NWmax\n#NWmax WALKMESH  ASCII\nbeginwalkmeshgeom tile\nnode aabb walk\nparent tile\nverts 3\n0 0 0\n1 0 0\n0 1 0\nfaces 1\n0 1 2 3 0 1 2 4\naabb 0 0 0 1 1 0 0\nendnode\nendwalkmeshgeom tile\n",
        )
        .expect("standalone WOK");
        assert_eq!(model.name, "tile");
        let mesh = model.nodes[0].mesh.as_ref().expect("AABB mesh");
        assert!(mesh.walkmesh);
        assert_eq!(mesh.surface_ids, [3]);

        let model = parse_mdl(
            b"#NWmax PWKMESH  ASCII\nnode trimesh NoWalk\nparent object_pwk\nverts 3\n0 0 0\n1 0 0\n0 1 0\nfaces 1\n0 1 2 1 0 0 0 7\nendnode\nnode dummy object_pwk_use01\nparent object_pwk\nposition 0 1 0\nendnode\n",
        )
        .expect("standalone PWK");
        assert_eq!(model.nodes.len(), 2);
        assert_eq!(model.nodes[0].mesh.as_ref().expect("mesh").surface_ids, [1]);

        let model = parse_mdl(
            b"#MAXDOOR ASCII\n# model: object_pwk\nnode dummy object_pwk_use01\nparent object_pwk\nposition 0 1 0\nendnode\n",
        )
        .expect("legacy hook-only PWK");
        assert_eq!(model.nodes.len(), 1);
        assert!(model.nodes[0].mesh.is_none());
    }

    #[test]
    fn retains_binary_face_surface_identifiers() {
        let model = parse_mdl(&binary_triangle_fixture()).expect("binary model");
        let mesh = model.nodes[0].mesh.as_ref().expect("mesh");
        assert_eq!(mesh.surface_ids, [7]);
    }

    #[test]
    fn retains_binary_tile_fade_mode() {
        let mut bytes = binary_triangle_fixture();
        let mesh = FILE_HEADER_SIZE + MODEL_HEADER_SIZE + NODE_HEADER_SIZE;
        bytes[mesh + 376..mesh + 380].copy_from_slice(&3_u32.to_le_bytes());
        let model = parse_mdl(&bytes).expect("binary model");
        assert_eq!(
            model.nodes[0]
                .mesh
                .as_ref()
                .expect("mesh")
                .material
                .tile_fade,
            3
        );
    }

    #[test]
    fn rejects_binary_pointer_outside_file() {
        let mut bytes = vec![0_u8; FILE_HEADER_SIZE + MODEL_HEADER_SIZE];
        bytes[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        let error = parse_mdl(&bytes).expect_err("invalid MDX pointer");
        assert_eq!(error.code, "MDL_MDX_POINTER_OUT_OF_RANGE");
    }

    #[test]
    fn detects_binary_node_cycles_without_recursing_forever() {
        let mut bytes = binary_triangle_fixture();
        let root_pointer = 232_u32;
        let child_array_pointer = 344_u32;
        let root = FILE_HEADER_SIZE + root_pointer as usize;
        bytes[root + 72..root + 76].copy_from_slice(&child_array_pointer.to_le_bytes());
        bytes[root + 76..root + 80].copy_from_slice(&1_u32.to_le_bytes());
        bytes[root + 80..root + 84].copy_from_slice(&1_u32.to_le_bytes());
        let child_array = FILE_HEADER_SIZE + child_array_pointer as usize;
        bytes[child_array..child_array + 4].copy_from_slice(&root_pointer.to_le_bytes());
        let model = parse_mdl(&bytes).expect("cycle is recoverable");
        assert!(
            model
                .diagnostics
                .iter()
                .any(|value| value.code == "MDL_NODE_CYCLE")
        );
    }

    pub(crate) fn binary_triangle_fixture() -> Vec<u8> {
        let model_header = MODEL_HEADER_SIZE;
        let node_offset = model_header;
        let node_size = NODE_HEADER_SIZE + MESH_HEADER_SIZE;
        let face_offset = node_offset + node_size;
        let mdx_offset = face_offset + 32;
        let mut bytes = vec![0_u8; FILE_HEADER_SIZE + mdx_offset + 3 * 12 + 3 * 12 + 3 * 8];
        bytes[4..8].copy_from_slice(&(mdx_offset as u32).to_le_bytes());
        bytes[8..12].copy_from_slice(&((3 * 12 + 3 * 12 + 3 * 8) as u32).to_le_bytes());
        let base = FILE_HEADER_SIZE;
        bytes[base + 8..base + 16].copy_from_slice(b"triangle");
        bytes[base + 72..base + 76].copy_from_slice(&(node_offset as u32).to_le_bytes());
        bytes[base + 76..base + 80].copy_from_slice(&1_u32.to_le_bytes());
        bytes[base + 164..base + 168].copy_from_slice(&1.0_f32.to_le_bytes());
        let node = base + node_offset;
        bytes[node + 28..node + 32].copy_from_slice(&0_u32.to_le_bytes());
        bytes[node + 32..node + 36].copy_from_slice(b"body");
        bytes[node + 108..node + 112].copy_from_slice(&(1_u32 << 5).to_le_bytes());
        let mesh = node + NODE_HEADER_SIZE;
        bytes[mesh + 8..mesh + 12].copy_from_slice(&(face_offset as u32).to_le_bytes());
        bytes[mesh + 12..mesh + 16].copy_from_slice(&1_u32.to_le_bytes());
        bytes[mesh + 16..mesh + 20].copy_from_slice(&1_u32.to_le_bytes());
        for (axis, value) in [0.8_f32, 0.6, 0.4].into_iter().enumerate() {
            bytes[mesh + 60 + axis * 4..mesh + 64 + axis * 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes[mesh + 108..mesh + 112].copy_from_slice(&1_u32.to_le_bytes());
        bytes[mesh + 120..mesh + 125].copy_from_slice(b"stone");
        bytes[mesh + 444..mesh + 448].copy_from_slice(&0_i32.to_le_bytes());
        bytes[mesh + 448..mesh + 450].copy_from_slice(&3_u16.to_le_bytes());
        bytes[mesh + 452..mesh + 456].copy_from_slice(&(72_i32).to_le_bytes());
        bytes[mesh + 468..mesh + 472].copy_from_slice(&(36_i32).to_le_bytes());
        bytes[mesh + 472..mesh + 476].copy_from_slice(&(-1_i32).to_le_bytes());
        let face = base + face_offset;
        bytes[face + 16..face + 20].copy_from_slice(&7_i32.to_le_bytes());
        bytes[face + 26..face + 28].copy_from_slice(&0_u16.to_le_bytes());
        bytes[face + 28..face + 30].copy_from_slice(&1_u16.to_le_bytes());
        bytes[face + 30..face + 32].copy_from_slice(&2_u16.to_le_bytes());
        let mdx = base + mdx_offset;
        let positions = [[0.0_f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        for (vertex, position) in positions.into_iter().enumerate() {
            for (axis, value) in position.into_iter().enumerate() {
                let at = mdx + vertex * 12 + axis * 4;
                bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
            }
        }
        for vertex in 0..3 {
            let at = mdx + 36 + vertex * 12 + 8;
            bytes[at..at + 4].copy_from_slice(&1.0_f32.to_le_bytes());
        }
        let uv = [[0.0_f32, 0.0], [1.0, 0.0], [0.0, 1.0]];
        for (vertex, coordinates) in uv.into_iter().enumerate() {
            for (axis, value) in coordinates.into_iter().enumerate() {
                let at = mdx + 72 + vertex * 8 + axis * 4;
                bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
            }
        }
        bytes
    }
}
