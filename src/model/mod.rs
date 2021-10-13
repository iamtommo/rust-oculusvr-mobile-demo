use std::collections::HashMap;

use math::matrix::float4x4;
use math::quaternion::quaternion;
use math::vector::{float2, float3};

use crate::render::gl_geometry::VertexAttribs;
use crate::anim::skeletal::{TRS};

pub struct Mesh {
    pub attribs: VertexAttribs,
    pub indices: Vec<u16>,
}

pub struct SkeletalMesh {
    pub mesh: Mesh,
    pub rig: Rig,
    pub joint_transforms: Vec<TRS>,
    pub joint_local_transforms: Vec<float4x4>,
    pub inverse_bind_matrices: Vec<float4x4>
}

/// skeletal rig
pub struct Rig {
    /// flat joints array
    pub joint_transforms: Vec<Joint>,
    /// num joints
    pub joint_count: usize,
    /// flat joint names array
    pub joint_names: Vec<String>,
    /// flat joint child joint indices array
    pub joint_children: Vec<Vec<usize>>,
    /// flat joint parent indices array
    pub joint_parents: Vec<usize>,
    /// source bone remap table
    pub remap_table: RigRemapTable
}

/// blittable bone
#[derive(Copy, Clone, Debug)]
pub struct Joint {
    /// joint index in rig joints array
    pub index: usize,
    /// parent joint index in rig joints array (zero for root joint)
    pub parent_index: usize,
    pub translation: float3,
    pub rotation: quaternion,
    pub scale: float3,
}

/// joint index remap table for source compatibility
pub struct RigRemapTable {
    pub joints: HashMap<usize, usize>
}