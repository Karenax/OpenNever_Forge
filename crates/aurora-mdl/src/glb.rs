use crate::{MdlError, MdlModel, MdlNode, TrackPath};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const GLB_CACHE_SCHEMA_VERSION: u32 = 7;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GlbArtifact {
    #[serde(skip)]
    pub bytes: Vec<u8>,
    pub schema_version: u32,
    pub source_sha256: String,
    pub glb_sha256: String,
    pub node_count: usize,
    pub mesh_count: usize,
    pub primitive_count: usize,
    pub animation_count: usize,
    pub skin_count: usize,
    pub byte_length: usize,
}

pub fn export_glb(model: &MdlModel) -> Result<GlbArtifact, MdlError> {
    let mut builder = GlbBuilder::default();
    let (safe_parents, safe_children) = safe_hierarchy(&model.nodes);
    let node_number_map = model
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.node_number as i16, index))
        .collect::<BTreeMap<_, _>>();

    let mut mesh_for_node = vec![None; model.nodes.len()];
    let mut skin_for_node = vec![None; model.nodes.len()];
    for (node_index, node) in model.nodes.iter().enumerate() {
        let Some(mesh) = &node.mesh else {
            continue;
        };
        if mesh.positions.is_empty() || mesh.indices.is_empty() {
            continue;
        }
        let position_values = mesh
            .positions
            .iter()
            .copied()
            .map(convert_vec3)
            .collect::<Vec<_>>();
        let normal_values = mesh
            .normals
            .iter()
            .copied()
            .map(convert_vec3)
            .collect::<Vec<_>>();
        let uv_values = mesh
            .uv0
            .iter()
            .map(|value| [value[0], 1.0 - value[1]])
            .collect::<Vec<_>>();
        // NWN is Z-up and glTF is Y-up. This axis conversion has a positive
        // determinant, so it preserves triangle winding.

        let position_accessor = builder.push_vec3_f32(&position_values, 34962, true)?;
        let normal_accessor = if normal_values.len() == position_values.len() {
            Some(builder.push_vec3_f32(&normal_values, 34962, false)?)
        } else {
            None
        };
        let uv_accessor = if uv_values.len() == position_values.len() {
            Some(builder.push_vec2_f32(&uv_values, 34962)?)
        } else {
            None
        };
        let color_accessor = if mesh.colors.len() == position_values.len() {
            Some(builder.push_rgba_u8(&mesh.colors, 34962)?)
        } else {
            None
        };
        let index_accessor = builder.push_indices(&mesh.indices)?;
        let material_index = builder.materials.len();
        let alpha = if mesh.material.transparency_hint == 0 {
            1.0
        } else {
            0.75
        };
        builder.materials.push(json!({
            "name": format!("material:{}", node.name),
            "pbrMetallicRoughness": {
                "baseColorFactor": [
                    mesh.material.diffuse[0], mesh.material.diffuse[1], mesh.material.diffuse[2], alpha
                ],
                "metallicFactor": 0.0,
                "roughnessFactor": shininess_to_roughness(mesh.material.shininess)
            },
            "doubleSided": true,
            "alphaMode": if alpha < 1.0 { "BLEND" } else { "OPAQUE" },
            "extras": {
                "nwnTextures": mesh.material.textures,
                "ambient": mesh.material.ambient,
                "specular": mesh.material.specular,
                "render": mesh.material.render,
                "nwnTileFade": mesh.material.tile_fade,
                "walkmesh": mesh.walkmesh
            }
        }));
        let mut attributes = serde_json::Map::new();
        attributes.insert("POSITION".to_owned(), json!(position_accessor));
        if let Some(accessor) = normal_accessor {
            attributes.insert("NORMAL".to_owned(), json!(accessor));
        }
        if let Some(accessor) = uv_accessor {
            attributes.insert("TEXCOORD_0".to_owned(), json!(accessor));
        }
        if let Some(accessor) = color_accessor {
            attributes.insert("COLOR_0".to_owned(), json!(accessor));
        }

        if let Some(skin) = &mesh.skin
            && skin.weights.len() == position_values.len()
            && skin.bone_indices.len() == position_values.len()
            && !skin.bone_mapping.is_empty()
        {
            let fallback = model
                .nodes
                .iter()
                .enumerate()
                .find_map(|(index, value)| value.parent.is_none().then_some(index))
                .unwrap_or(0);
            let (joints, vertex_joints, vertex_weights) = remap_skin(
                &node_number_map,
                &skin.bone_mapping,
                &skin.bone_indices,
                &skin.weights,
                fallback,
            );
            let joints_accessor = builder.push_joints(&vertex_joints, 34962)?;
            let weights_accessor = builder.push_vec4_f32(&vertex_weights, 34962)?;
            attributes.insert("JOINTS_0".to_owned(), json!(joints_accessor));
            attributes.insert("WEIGHTS_0".to_owned(), json!(weights_accessor));
            let skin_index = builder.skins.len();
            builder.skins.push(json!({
                "name": format!("skin:{}", node.name),
                "joints": joints,
                "skeleton": safe_parents[node_index].unwrap_or(fallback),
                "extras": { "nwnBoneMapping": skin.bone_mapping }
            }));
            skin_for_node[node_index] = Some(skin_index);
        }

        let mesh_index = builder.meshes.len();
        builder.meshes.push(json!({
            "name": node.name,
            "primitives": [{
                "attributes": Value::Object(attributes),
                "indices": index_accessor,
                "material": material_index,
                "mode": 4
            }]
        }));
        mesh_for_node[node_index] = Some(mesh_index);
    }

    let mut nodes_json = Vec::with_capacity(model.nodes.len());
    for (index, node) in model.nodes.iter().enumerate() {
        let mut value = serde_json::Map::new();
        value.insert("name".to_owned(), json!(node.name));
        if !safe_children[index].is_empty() {
            value.insert("children".to_owned(), json!(safe_children[index]));
        }
        let translation = convert_vec3(node.translation);
        if translation != [0.0, 0.0, 0.0] {
            value.insert("translation".to_owned(), json!(translation));
        }
        let rotation = convert_quaternion(node.rotation);
        if rotation != [0.0, 0.0, 0.0, 1.0] {
            value.insert("rotation".to_owned(), json!(rotation));
        }
        let scale = [node.scale[0], node.scale[2], node.scale[1]];
        if scale != [1.0, 1.0, 1.0] {
            value.insert("scale".to_owned(), json!(scale));
        }
        if skin_for_node[index].is_none()
            && let Some(mesh) = mesh_for_node[index]
        {
            value.insert("mesh".to_owned(), json!(mesh));
        }
        value.insert(
            "extras".to_owned(),
            json!({
                "nwnNodeNumber": node.node_number,
                "nwnKinds": node.kinds,
                "nwnReferenceModel": node.reference_model
            }),
        );
        nodes_json.push(Value::Object(value));
    }
    let mut skinned_roots = Vec::new();
    for (source_index, skin) in skin_for_node.iter().enumerate() {
        let (Some(skin), Some(mesh)) = (*skin, mesh_for_node[source_index]) else {
            continue;
        };
        skinned_roots.push(nodes_json.len());
        nodes_json.push(json!({
            "name": format!("{}:skinned-mesh", model.nodes[source_index].name),
            "mesh": mesh,
            "skin": skin,
            "extras": {
                "nwnSourceNode": source_index,
                "nwnSkinCarrier": true
            }
        }));
    }

    let names = model
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.name.to_ascii_lowercase(), index))
        .collect::<BTreeMap<_, _>>();
    for animation in &model.animations {
        let mut samplers = Vec::new();
        let mut channels = Vec::new();
        for track in &animation.tracks {
            let Some(node_index) = names.get(&track.node.to_ascii_lowercase()).copied() else {
                continue;
            };
            if track.times.is_empty() || track.times.len() != track.values.len() {
                continue;
            }
            let input = builder.push_scalar_f32(&track.times, 0, true)?;
            let (output, path) = match track.path {
                TrackPath::Translation => {
                    let values = track
                        .values
                        .iter()
                        .map(|value| convert_vec3([value[0], value[1], value[2]]))
                        .collect::<Vec<_>>();
                    (builder.push_vec3_f32(&values, 0, false)?, "translation")
                }
                TrackPath::Rotation => {
                    let values = track
                        .values
                        .iter()
                        .copied()
                        .map(convert_quaternion)
                        .collect::<Vec<_>>();
                    (builder.push_vec4_f32(&values, 0)?, "rotation")
                }
                TrackPath::Scale => {
                    let values = track
                        .values
                        .iter()
                        .map(|value| [value[0], value[2], value[1]])
                        .collect::<Vec<_>>();
                    (builder.push_vec3_f32(&values, 0, false)?, "scale")
                }
            };
            let sampler = samplers.len();
            samplers.push(json!({ "input": input, "output": output, "interpolation": "LINEAR" }));
            channels.push(
                json!({ "sampler": sampler, "target": { "node": node_index, "path": path } }),
            );
        }
        if !channels.is_empty() {
            builder.animations.push(json!({
                "name": animation.name,
                "samplers": samplers,
                "channels": channels,
                "extras": {
                    "nwnLength": animation.length,
                    "nwnTransition": animation.transition,
                    "nwnRoot": animation.root_node,
                    "nwnEvents": animation.events
                }
            }));
        }
    }

    let mut roots = safe_parents
        .iter()
        .enumerate()
        .filter_map(|(index, parent)| parent.is_none().then_some(index))
        .collect::<Vec<_>>();
    roots.extend(skinned_roots);
    let exported_node_count = nodes_json.len();
    let skin_count = builder.skins.len();
    let animation_count = builder.animations.len();
    let mut json_document = json!({
        "asset": {
            "version": "2.0",
            "generator": "OpenNever Forge aurora-mdl 0.1",
            "copyright": "Generated locally from user-owned NWN resources; not redistributed",
            "extras": {
                "cacheSchemaVersion": GLB_CACHE_SCHEMA_VERSION,
                "sourceSha256": model.source_sha256,
                "sourceFormat": model.format,
                "supermodel": model.supermodel,
                "nwnBoundsMin": model.bounds_min,
                "nwnBoundsMax": model.bounds_max,
                "nwnRadius": model.radius,
                "nwnModelScale": model.model_scale
            }
        },
        "scene": 0,
        "scenes": [{ "name": model.name, "nodes": roots }],
        "nodes": nodes_json,
        "meshes": builder.meshes,
        "materials": builder.materials,
        "skins": builder.skins,
        "animations": builder.animations,
        "accessors": builder.accessors,
        "bufferViews": builder.buffer_views,
        "buffers": [{ "byteLength": builder.binary.len() }]
    });
    let object = json_document
        .as_object_mut()
        .expect("the GLB root is always a JSON object");
    if skin_count == 0 {
        object.remove("skins");
    }
    if animation_count == 0 {
        object.remove("animations");
    }
    let bytes = encode_glb(&json_document, &builder.binary)?;
    let glb_sha256 = format!("{:x}", Sha256::digest(&bytes));
    let primitive_count = builder.meshes.len();
    Ok(GlbArtifact {
        byte_length: bytes.len(),
        bytes,
        schema_version: GLB_CACHE_SCHEMA_VERSION,
        source_sha256: model.source_sha256.clone(),
        glb_sha256,
        node_count: exported_node_count,
        mesh_count: builder.meshes.len(),
        primitive_count,
        animation_count,
        skin_count,
    })
}

#[derive(Default)]
struct GlbBuilder {
    binary: Vec<u8>,
    buffer_views: Vec<Value>,
    accessors: Vec<Value>,
    meshes: Vec<Value>,
    materials: Vec<Value>,
    skins: Vec<Value>,
    animations: Vec<Value>,
}

impl GlbBuilder {
    fn push_vec2_f32(&mut self, values: &[[f32; 2]], target: u32) -> Result<usize, MdlError> {
        let bytes = values
            .iter()
            .flat_map(|value| value.iter().flat_map(|item| item.to_le_bytes()))
            .collect::<Vec<_>>();
        self.push_accessor(bytes, values.len(), 5126, "VEC2", target, None, None, false)
    }

    fn push_vec3_f32(
        &mut self,
        values: &[[f32; 3]],
        target: u32,
        include_bounds: bool,
    ) -> Result<usize, MdlError> {
        let bytes = values
            .iter()
            .flat_map(|value| value.iter().flat_map(|item| item.to_le_bytes()))
            .collect::<Vec<_>>();
        let (minimum, maximum) = if include_bounds && !values.is_empty() {
            let mut minimum = [f32::INFINITY; 3];
            let mut maximum = [f32::NEG_INFINITY; 3];
            for value in values {
                for axis in 0..3 {
                    minimum[axis] = minimum[axis].min(value[axis]);
                    maximum[axis] = maximum[axis].max(value[axis]);
                }
            }
            (Some(json!(minimum)), Some(json!(maximum)))
        } else {
            (None, None)
        };
        self.push_accessor(
            bytes,
            values.len(),
            5126,
            "VEC3",
            target,
            minimum,
            maximum,
            false,
        )
    }

    fn push_vec4_f32(&mut self, values: &[[f32; 4]], target: u32) -> Result<usize, MdlError> {
        let bytes = values
            .iter()
            .flat_map(|value| value.iter().flat_map(|item| item.to_le_bytes()))
            .collect::<Vec<_>>();
        self.push_accessor(bytes, values.len(), 5126, "VEC4", target, None, None, false)
    }

    fn push_scalar_f32(
        &mut self,
        values: &[f32],
        target: u32,
        include_bounds: bool,
    ) -> Result<usize, MdlError> {
        let bytes = values
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect::<Vec<_>>();
        let minimum =
            include_bounds.then(|| json!([values.iter().copied().fold(f32::INFINITY, f32::min)]));
        let maximum = include_bounds
            .then(|| json!([values.iter().copied().fold(f32::NEG_INFINITY, f32::max)]));
        self.push_accessor(
            bytes,
            values.len(),
            5126,
            "SCALAR",
            target,
            minimum,
            maximum,
            false,
        )
    }

    fn push_rgba_u8(&mut self, values: &[[u8; 4]], target: u32) -> Result<usize, MdlError> {
        let bytes = values.iter().flatten().copied().collect::<Vec<_>>();
        self.push_accessor(bytes, values.len(), 5121, "VEC4", target, None, None, true)
    }

    fn push_joints(&mut self, values: &[[u16; 4]], target: u32) -> Result<usize, MdlError> {
        let bytes = values
            .iter()
            .flat_map(|value| value.iter().flat_map(|item| item.to_le_bytes()))
            .collect::<Vec<_>>();
        self.push_accessor(bytes, values.len(), 5123, "VEC4", target, None, None, false)
    }

    fn push_indices(&mut self, values: &[u32]) -> Result<usize, MdlError> {
        let maximum = values.iter().copied().max().unwrap_or(0);
        if maximum <= u16::MAX as u32 {
            let bytes = values
                .iter()
                .flat_map(|value| (*value as u16).to_le_bytes())
                .collect::<Vec<_>>();
            self.push_accessor(
                bytes,
                values.len(),
                5123,
                "SCALAR",
                34963,
                Some(json!([values.iter().copied().min().unwrap_or(0)])),
                Some(json!([maximum])),
                false,
            )
        } else {
            let bytes = values
                .iter()
                .flat_map(|value| value.to_le_bytes())
                .collect::<Vec<_>>();
            self.push_accessor(
                bytes,
                values.len(),
                5125,
                "SCALAR",
                34963,
                Some(json!([values.iter().copied().min().unwrap_or(0)])),
                Some(json!([maximum])),
                false,
            )
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn push_accessor(
        &mut self,
        bytes: Vec<u8>,
        count: usize,
        component_type: u32,
        accessor_type: &str,
        target: u32,
        minimum: Option<Value>,
        maximum: Option<Value>,
        normalized: bool,
    ) -> Result<usize, MdlError> {
        if count == 0 {
            return Err(MdlError {
                code: "GLB_ACCESSOR_EMPTY".to_owned(),
                message: format!("cannot create empty {accessor_type} accessor"),
                offset: None,
            });
        }
        align4(&mut self.binary, 0);
        let byte_offset = self.binary.len();
        self.binary.extend(bytes);
        let view = self.buffer_views.len();
        let mut view_json = serde_json::Map::new();
        view_json.insert("buffer".to_owned(), json!(0));
        view_json.insert("byteOffset".to_owned(), json!(byte_offset));
        view_json.insert(
            "byteLength".to_owned(),
            json!(self.binary.len() - byte_offset),
        );
        if target != 0 {
            view_json.insert("target".to_owned(), json!(target));
        }
        self.buffer_views.push(Value::Object(view_json));
        let accessor = self.accessors.len();
        let mut accessor_json = serde_json::Map::new();
        accessor_json.insert("bufferView".to_owned(), json!(view));
        accessor_json.insert("componentType".to_owned(), json!(component_type));
        accessor_json.insert("count".to_owned(), json!(count));
        accessor_json.insert("type".to_owned(), json!(accessor_type));
        if normalized {
            accessor_json.insert("normalized".to_owned(), json!(true));
        }
        if let Some(value) = minimum {
            accessor_json.insert("min".to_owned(), value);
        }
        if let Some(value) = maximum {
            accessor_json.insert("max".to_owned(), value);
        }
        self.accessors.push(Value::Object(accessor_json));
        Ok(accessor)
    }
}

fn encode_glb(document: &Value, binary: &[u8]) -> Result<Vec<u8>, MdlError> {
    let mut json_bytes = serde_json::to_vec(document).map_err(|error| MdlError {
        code: "GLB_JSON_SERIALIZATION_FAILED".to_owned(),
        message: error.to_string(),
        offset: None,
    })?;
    align4(&mut json_bytes, b' ');
    let mut binary = binary.to_vec();
    align4(&mut binary, 0);
    let total_length = 12_usize
        .checked_add(8)
        .and_then(|value| value.checked_add(json_bytes.len()))
        .and_then(|value| value.checked_add(8))
        .and_then(|value| value.checked_add(binary.len()))
        .ok_or_else(|| MdlError {
            code: "GLB_SIZE_OVERFLOW".to_owned(),
            message: "GLB output exceeds addressable memory".to_owned(),
            offset: None,
        })?;
    let total_length = u32::try_from(total_length).map_err(|_| MdlError {
        code: "GLB_SIZE_LIMIT_EXCEEDED".to_owned(),
        message: "GLB output exceeds 4 GiB".to_owned(),
        offset: None,
    })?;
    let json_length = u32::try_from(json_bytes.len()).map_err(|_| MdlError {
        code: "GLB_JSON_SIZE_LIMIT_EXCEEDED".to_owned(),
        message: "GLB JSON chunk exceeds 4 GiB".to_owned(),
        offset: None,
    })?;
    let binary_length = u32::try_from(binary.len()).map_err(|_| MdlError {
        code: "GLB_BINARY_SIZE_LIMIT_EXCEEDED".to_owned(),
        message: "GLB binary chunk exceeds 4 GiB".to_owned(),
        offset: None,
    })?;
    let mut output = Vec::with_capacity(total_length as usize);
    output.extend(0x4654_6C67_u32.to_le_bytes());
    output.extend(2_u32.to_le_bytes());
    output.extend(total_length.to_le_bytes());
    output.extend(json_length.to_le_bytes());
    output.extend(0x4E4F_534A_u32.to_le_bytes());
    output.extend(json_bytes);
    output.extend(binary_length.to_le_bytes());
    output.extend(0x004E_4942_u32.to_le_bytes());
    output.extend(binary);
    Ok(output)
}

fn align4(bytes: &mut Vec<u8>, padding: u8) {
    while !bytes.len().is_multiple_of(4) {
        bytes.push(padding);
    }
}

fn convert_vec3(value: [f32; 3]) -> [f32; 3] {
    [value[0], value[2], -value[1]]
}

fn convert_quaternion(value: [f32; 4]) -> [f32; 4] {
    [value[0], value[2], -value[1], value[3]]
}

fn remap_skin(
    node_number_map: &BTreeMap<i16, usize>,
    bone_mapping: &[i16],
    source_indices: &[[u16; 4]],
    source_weights: &[[f32; 4]],
    fallback: usize,
) -> (Vec<usize>, Vec<[u16; 4]>, Vec<[f32; 4]>) {
    let mut joints = vec![fallback];
    let mut joint_indices = BTreeMap::from([(fallback, 0_u16)]);
    let mapping = bone_mapping
        .iter()
        .map(|node_number| {
            let node = node_number_map.get(node_number).copied()?;
            if let Some(index) = joint_indices.get(&node) {
                return Some(*index);
            }
            let index = u16::try_from(joints.len()).ok()?;
            joints.push(node);
            joint_indices.insert(node, index);
            Some(index)
        })
        .collect::<Vec<_>>();

    let mut vertex_joints = Vec::with_capacity(source_indices.len());
    let mut vertex_weights = Vec::with_capacity(source_weights.len());
    for (indices, weights) in source_indices.iter().zip(source_weights) {
        let mut combined = BTreeMap::<u16, f32>::new();
        for (source, weight) in indices.iter().zip(weights) {
            if !weight.is_finite() || *weight <= 0.0 {
                continue;
            }
            let Some(Some(joint)) = mapping.get(usize::from(*source)) else {
                continue;
            };
            *combined.entry(*joint).or_default() += *weight;
        }
        let mut influences = combined.into_iter().collect::<Vec<_>>();
        influences.sort_by(|left, right| {
            right
                .1
                .total_cmp(&left.1)
                .then_with(|| left.0.cmp(&right.0))
        });
        influences.truncate(4);
        if influences.is_empty() {
            influences.push((0, 1.0));
        }
        let total = influences.iter().map(|(_, weight)| *weight).sum::<f32>();
        let mut remapped_indices = [0_u16; 4];
        let mut remapped_weights = [0.0_f32; 4];
        for (slot, (joint, weight)) in influences.into_iter().enumerate() {
            remapped_indices[slot] = joint;
            remapped_weights[slot] = weight / total;
        }
        vertex_joints.push(remapped_indices);
        vertex_weights.push(remapped_weights);
    }
    (joints, vertex_joints, vertex_weights)
}

fn safe_hierarchy(nodes: &[MdlNode]) -> (Vec<Option<usize>>, Vec<Vec<usize>>) {
    let mut parents = vec![None; nodes.len()];
    for (index, node) in nodes.iter().enumerate() {
        let Some(parent) = node.parent.filter(|parent| *parent < nodes.len()) else {
            continue;
        };
        let mut seen = BTreeMap::from([(index, ())]);
        let mut current = Some(parent);
        let mut valid = true;
        while let Some(candidate) = current {
            if candidate >= nodes.len() || seen.insert(candidate, ()).is_some() {
                valid = false;
                break;
            }
            current = nodes[candidate].parent;
        }
        if valid {
            parents[index] = Some(parent);
        }
    }
    let mut children = vec![Vec::new(); nodes.len()];
    for (index, parent) in parents.iter().enumerate() {
        if let Some(parent) = parent {
            children[*parent].push(index);
        }
    }
    (parents, children)
}

fn shininess_to_roughness(value: f32) -> f32 {
    (1.0 - (value / 128.0).clamp(0.0, 1.0)).clamp(0.04, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_mdl;

    fn glb_parts(bytes: &[u8]) -> (Value, &[u8]) {
        let json_length = usize::try_from(u32::from_le_bytes(
            bytes[12..16].try_into().expect("JSON length"),
        ))
        .expect("JSON length fits usize");
        let json_end = 20 + json_length;
        let document = serde_json::from_slice(&bytes[20..json_end]).expect("GLB JSON");
        let binary_length = usize::try_from(u32::from_le_bytes(
            bytes[json_end..json_end + 4]
                .try_into()
                .expect("binary length"),
        ))
        .expect("binary length fits usize");
        let binary_start = json_end + 8;
        (document, &bytes[binary_start..binary_start + binary_length])
    }

    fn accessor_offset(document: &Value, accessor: usize) -> usize {
        let view = document["accessors"][accessor]["bufferView"]
            .as_u64()
            .expect("buffer view") as usize;
        let view_offset = document["bufferViews"][view]["byteOffset"]
            .as_u64()
            .unwrap_or(0) as usize;
        let accessor_offset = document["accessors"][accessor]["byteOffset"]
            .as_u64()
            .unwrap_or(0) as usize;
        view_offset + accessor_offset
    }

    fn read_vec3_f32(document: &Value, binary: &[u8], accessor: usize) -> Vec<[f32; 3]> {
        assert_eq!(document["accessors"][accessor]["componentType"], 5126);
        assert_eq!(document["accessors"][accessor]["type"], "VEC3");
        let count = document["accessors"][accessor]["count"]
            .as_u64()
            .expect("accessor count") as usize;
        let offset = accessor_offset(document, accessor);
        (0..count)
            .map(|index| {
                let start = offset + index * 12;
                std::array::from_fn(|axis| {
                    let at = start + axis * 4;
                    f32::from_le_bytes(binary[at..at + 4].try_into().expect("f32"))
                })
            })
            .collect()
    }

    fn read_u16_indices(document: &Value, binary: &[u8], accessor: usize) -> Vec<usize> {
        assert_eq!(document["accessors"][accessor]["componentType"], 5123);
        let count = document["accessors"][accessor]["count"]
            .as_u64()
            .expect("index count") as usize;
        let offset = accessor_offset(document, accessor);
        (0..count)
            .map(|index| {
                let at = offset + index * 2;
                usize::from(u16::from_le_bytes(
                    binary[at..at + 2].try_into().expect("u16"),
                ))
            })
            .collect()
    }

    #[test]
    fn exports_deterministic_glb_with_triangle_mesh() {
        let model = parse_mdl(
            b"newmodel triangle\nnode trimesh body\ntilefade 1\nverts 3\n0 0 0\n1 0 0\n0 1 0\nfaces 1\n0 1 2 0 0 1 2 0\nendnode\n",
        )
        .expect("model");
        let first = export_glb(&model).expect("first GLB");
        let second = export_glb(&model).expect("second GLB");
        assert_eq!(first.bytes, second.bytes);
        assert_eq!(&first.bytes[..4], b"glTF");
        assert_eq!(first.mesh_count, 1);
        assert_eq!(first.primitive_count, 1);
        let json_length = u32::from_le_bytes(first.bytes[12..16].try_into().expect("JSON length"));
        let document: Value = serde_json::from_slice(
            &first.bytes[20..20 + usize::try_from(json_length).expect("length fits usize")],
        )
        .expect("GLB JSON");
        assert!(document.get("skins").is_none());
        assert!(document.get("animations").is_none());
        assert_eq!(document["materials"][0]["extras"]["nwnTileFade"], 1);
    }

    #[test]
    fn textured_floor_keeps_upward_winding_after_axis_conversion() {
        let model = parse_mdl(
            b"newmodel floor\nnode trimesh floor\nverts 3\n0 0 0\n1 0 0\n0 1 0\ntverts 3\n0 0\n1 0\n0 1\nfaces 1\n0 1 2 0 0 1 2 0\nbitmap floor_stone\nendnode\ndonemodel floor\n",
        )
        .expect("textured floor model");
        let artifact = export_glb(&model).expect("floor GLB");
        let (document, binary) = glb_parts(&artifact.bytes);
        let primitive = &document["meshes"][0]["primitives"][0];
        let positions_accessor = primitive["attributes"]["POSITION"]
            .as_u64()
            .expect("positions accessor") as usize;
        let normals_accessor = primitive["attributes"]["NORMAL"]
            .as_u64()
            .expect("normals accessor") as usize;
        let indices_accessor = primitive["indices"].as_u64().expect("indices accessor") as usize;
        let positions = read_vec3_f32(&document, binary, positions_accessor);
        let normals = read_vec3_f32(&document, binary, normals_accessor);
        let indices = read_u16_indices(&document, binary, indices_accessor);

        let [a, b, c] = [
            positions[indices[0]],
            positions[indices[1]],
            positions[indices[2]],
        ];
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let face_normal = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        let vertex_normal = normals[indices[0]];
        let alignment = face_normal[0] * vertex_normal[0]
            + face_normal[1] * vertex_normal[1]
            + face_normal[2] * vertex_normal[2];

        assert_eq!(indices, [0, 1, 2]);
        assert!(
            face_normal[1] > 0.0,
            "the floor front face must point upward"
        );
        assert!(
            alignment > 0.0,
            "face winding and vertex normals must agree"
        );
        assert_eq!(
            document["materials"][0]["extras"]["nwnTextures"],
            json!(["floor_stone"])
        );
    }

    #[test]
    fn skin_remapping_deduplicates_joints_and_normalizes_weights() {
        let node_number_map = BTreeMap::from([(2_i16, 3_usize), (4_i16, 7_usize)]);
        let (joints, indices, weights) = remap_skin(
            &node_number_map,
            &[-1, 2, 2, 4],
            &[[0, 1, 2, 3], [0, 0, 0, 0]],
            &[[0.1, 0.2, 0.3, 0.4], [0.0, 0.0, 0.0, 0.0]],
            0,
        );
        assert_eq!(joints, vec![0, 3, 7]);
        assert_eq!(indices[0], [1, 2, 0, 0]);
        assert!((weights[0][0] - 5.0 / 9.0).abs() < f32::EPSILON);
        assert!((weights[0][1] - 4.0 / 9.0).abs() < f32::EPSILON);
        assert_eq!(indices[1], [0; 4]);
        assert_eq!(weights[1], [1.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn cyclic_ascii_parent_is_reported_and_not_exported() {
        let model = parse_mdl(
            b"newmodel loop\nnode dummy root\nparent root\nendnode\nnode trimesh body\nparent root\nverts 3\n0 0 0\n1 0 0\n0 1 0\nfaces 1\n0 1 2 0 0 1 2 0\nendnode\n",
        )
        .expect("recoverable model");
        assert!(
            model
                .diagnostics
                .iter()
                .any(|value| value.code == "MDL_NODE_SELF_PARENT")
        );
        let artifact = export_glb(&model).expect("GLB");
        let json_length =
            u32::from_le_bytes(artifact.bytes[12..16].try_into().expect("JSON length"));
        let document: Value = serde_json::from_slice(
            &artifact.bytes[20..20 + usize::try_from(json_length).expect("length fits usize")],
        )
        .expect("GLB JSON");
        let root_children = document["nodes"][0]["children"].as_array();
        assert!(root_children.is_none_or(|children| !children.iter().any(|value| value == 0)));
    }
}
