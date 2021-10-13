pub mod skeletal;
pub mod mesh;

use std::collections::HashMap;
use std::io::{Cursor, Read};

use base64;
use byteorder;
use byteorder::{LittleEndian, ReadBytesExt};
use math::matrix::{float4x4, matrix4x4_identity, matrix4x4_trs, matrix4x4_transpose, matrix4x4_mul, matrix4x4_inverse, matrix4x4_scale};
use math::quaternion::quaternion;
use math::vector::{float2, float3, float4, int4};
use serde::{Deserialize, Serialize};

use crate::model::{Joint, Mesh, Rig, RigRemapTable, SkeletalMesh};
use crate::render::gl_geometry::{VertexAttribs, MAX_JOINTS};
use crate::anim::skeletal::{SkeletalAnimation, TRS};
use crate::assets::Asset;

pub fn load_gltf(asset: &mut Asset) -> GltfFile {
    let mut file: GltfFile = serde_json::from_slice(&asset.get_buffer().unwrap()).unwrap();
    for buffer in &file.buffers {
        const BASE64_PREFIX: &str = "data:application/octet-stream;base64,";
        if !buffer.uri.starts_with(BASE64_PREFIX) {
            panic!("load_gltf failed: buffer uri is not base 64 encoded");
        }
        let data = base64::decode(&buffer.uri[BASE64_PREFIX.len()..]).unwrap();
        file.decoded_buffers.push(data);
    }
    return file;
}

fn matrix_from_gltf_node(node: &GltfNode) -> float4x4 {
    let trs = trs_from_gltf_node(&node);
    return matrix4x4_trs(&trs.0, &trs.1, &trs.2);
}

fn trs_from_gltf_node(node: &GltfNode) -> (float3, quaternion, float3) {
    let mut translation = float3::zero();
    let mut rotation = quaternion::identity();
    let mut scale = float3::new(1.0, 1.0, 1.0);

    let node_translation = node.translation.as_ref();
    if node_translation.is_some() {
        let values = node_translation.unwrap();
        assert_eq!(values.len(), 3);
        translation = float3::new(values[0], values[1], values[2]);
    }
    let node_rotation = node.rotation.as_ref();
    if node_rotation.is_some() {
        let values = node_rotation.unwrap();
        assert_eq!(values.len(), 4);
        rotation = quaternion::new(values[0], values[1], values[2], values[3]);
    }
    let node_scale = node.scale.as_ref();
    if node_scale.is_some() {
        let values = node_scale.unwrap();
        assert_eq!(values.len(), 3);
        scale = float3::new(values[0], values[1], values[2]);
    }
    return (translation, rotation, scale);
}



/// compute a cursor into the gltf buffer for given buffer view
fn get_buffer_cursor<'life>(gltf_file: &'life GltfFile, buffer_view: &GltfBufferView) -> Cursor<&'life Vec<u8>> {
    let buffer = &gltf_file.decoded_buffers[buffer_view.buffer];
    let mut cursor = Cursor::new(buffer);
    cursor.set_position(buffer_view.byte_offset as u64);
    return cursor;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GltfFile {
    pub meshes: Option<Vec<GltfMesh>>,
    #[serde(rename = "bufferViews")]
    pub buffer_views: Vec<GltfBufferView>,
    pub buffers: Vec<GltfBuffer>,
    pub accessors: Vec<GltfAccessor>,
    pub scenes: Vec<GltfScene>,
    pub nodes: Vec<GltfNode>,
    pub skins: Option<Vec<GltfSkin>>,
    pub animations: Option<Vec<GltfAnimation>>,

    /// decoded uri buffers (not part of gltf)
    #[serde(skip)]
    pub decoded_buffers: Vec<Vec<u8>>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GltfScene {
    pub name: Option<String>,
    #[serde(rename = "nodes")]
    pub root_nodes: Option<Vec<usize>>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GltfNode {
    pub name: String,
    pub children: Option<Vec<usize>>,
    pub translation: Option<Vec<f32>>,
    pub rotation: Option<Vec<f32>>,
    pub scale: Option<Vec<f32>>,
    pub matrix: Option<Vec<f32>>,
    pub skin: Option<usize>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GltfSkin {
    pub name: String,
    #[serde(rename="inverseBindMatrices")]
    pub inverse_bind_matrices: usize,
    pub joints: Vec<usize>,
    pub skeleton: Option<usize>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GltfMesh {
    pub name: String,
    pub primitives: Vec<GltfMeshPrimitive>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GltfMeshPrimitive {
    pub attributes: HashMap<String, usize>,
    pub indices: Option<usize>,
    pub material: Option<usize>,
    pub mode: Option<i64>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GltfBufferView {
    pub buffer: usize,
    #[serde(rename = "byteLength")]
    pub byte_length: usize,
    #[serde(rename = "byteOffset")]
    pub byte_offset: usize
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GltfBuffer {
    #[serde(rename = "byteLength")]
    pub byte_length: usize,
    pub uri: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GltfAccessor {
    #[serde(rename = "bufferView")]
    pub buffer_view: usize,
    #[serde(rename = "componentType")]
    pub component_type: i64,
    #[serde(rename = "type")]
    pub accessor_type: String,
    pub count: i64,
    pub min: Option<Vec<f32>>,
    pub max: Option<Vec<f32>>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GltfAnimation {
    pub name: String,
    pub channels: Vec<GltfAnimationChannel>,
    pub samplers: Vec<GltfAnimationSampler>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GltfAnimationChannel {
    pub sampler: usize,
    pub target: GltfAnimationChannelTarget
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GltfAnimationSampler {
    pub input: usize,
    pub interpolation: String,
    pub output: usize
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GltfAnimationChannelTarget {
    pub node: usize,
    pub path: String
}

pub enum GltfComponentType {
    Byte = 5120,
    UnsignedByte = 5121,
    Short = 5122,
    UnsignedShort = 5123,
    UnsignedInt = 5125,
    Float = 5126
}