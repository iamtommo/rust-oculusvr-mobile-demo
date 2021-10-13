use math::vector::float3;
use math::quaternion::quaternion;
use math::matrix::{float4x4, matrix4x4_transpose, matrix4x4_identity, matrix4x4_trs, matrix4x4_mul, matrix4x4_translation, matrix4x4_scale};
use crate::model::{SkeletalMesh, Rig};
use crate::render::gl_geometry;
use std::collections::HashMap;
use math::inverse_lerp;

/// densely packed joint transforms @ sample_rate
pub struct SkeletalAnimation {
    pub name: String,
    /// channel sample rate in Hz
    pub sample_rate: f32,
    pub num_frames: usize,
    pub min_time: f32,
    pub max_time: f32,
    /// vec of joint indices to vec of TRS per frame
    pub joints: Vec<Vec<TRS>>
}

#[derive(Copy, Clone)]
pub struct TRS {
    pub translation: float3,
    pub rotation: quaternion,
    pub scale: float3
}
impl Default for TRS {
    fn default() -> Self {
        TRS { translation: float3::zero(), rotation: quaternion::identity(), scale: float3::one() }
    }
}

pub enum InterpMethod {
    Step, Linear, CubicSpline
}

pub fn sample_bear(mesh: &SkeletalMesh, anim: &SkeletalAnimation, time: f32) -> Vec<float4x4> {
    let mut joint_matrices = Vec::new();
    for joint in 0..mesh.rig.joint_count {
        joint_matrices.push(mesh.joint_local_transforms[joint]);
    }

    let time_step = 1f32 / anim.sample_rate;
    for joint_idx in 0..anim.joints.len() {
        let frames = &anim.joints[joint_idx];
        let index_left = (time / time_step).floor() as usize;
        let index_right = index_left + 1;

        let time_left = time_step * (index_left as f32);
        let time_right = time_step * (index_right as f32);

        // case: no left keyframe: identity
        // case: no right keyframe: left keyframe
        let identity = TRS::default();
        let frame_left = frames.get(index_left).or(Some(&identity)).unwrap();
        let frame_right = frames.get(index_right).or(Some(&frame_left)).unwrap();

        let alpha = inverse_lerp(time_left, time_right, time);
        //info!("joint {} numframes {} time {} timestep {} idxl {}, idxr {} tl {} tr {} alpha {}", joint_idx, frames.len(), time, time_step, index_left, index_right, time_left, time_right, alpha);
        let translation = float3::lerp(&frame_left.translation, &frame_right.translation, alpha);
        let rotation = quaternion::slerp(&frame_left.rotation, &frame_right.rotation, alpha);
        let scale = float3::lerp(&frame_left.scale, &frame_right.scale, alpha);

        let joint_trs = mesh.joint_transforms[joint_idx];
        /*let transform = matrix4x4_trs(&(joint_trs.translation + translation),
                                      &quaternion::mul(&joint_trs.rotation, &rotation),
                                      &(joint_trs.scale * scale));// todo additive or multiplicative?*/
        //let transform = matrix4x4_mul(&matrix4x4_trs(&translation, &rotation, &scale), &mesh.joint_local_transforms[joint_idx]);
        let transform = matrix4x4_trs(&translation, &rotation, &scale);
        joint_matrices[joint_idx] = transform;
    }

    pose_hierarchy(&mesh.rig, &mut joint_matrices, &mesh.inverse_bind_matrices,
                   0, &matrix4x4_identity());

    return joint_matrices;
}

pub fn pose_hierarchy(rig: &Rig, joint_matrices: &mut Vec<float4x4>, inverse_bind_matrices: &Vec<float4x4>,
                      joint_idx: usize, parent: &float4x4) {
    let joint_local = joint_matrices[joint_idx];
    let joint_world = matrix4x4_mul(&parent, &joint_local);
    for child_joint_idx in rig.joint_children[joint_idx].iter() {
        pose_hierarchy(&rig, joint_matrices, inverse_bind_matrices, *child_joint_idx, &joint_world);
    }
    let posed = matrix4x4_mul(&joint_world, &inverse_bind_matrices[joint_idx]);
    joint_matrices[joint_idx] = matrix4x4_transpose(&posed);
}