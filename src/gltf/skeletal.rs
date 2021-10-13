use std::collections::HashMap;

use byteorder::{LittleEndian, ReadBytesExt};
use math::matrix::{float4x4, matrix4x4_identity, matrix4x4_inverse, matrix4x4_mul, matrix4x4_trs};
use math::quaternion::quaternion;
use math::vector::float3;

use crate::anim::skeletal::{SkeletalAnimation, TRS};
use crate::gltf::{get_buffer_cursor, GltfAnimation, GltfComponentType, GltfFile, GltfNode, trs_from_gltf_node};
use crate::gltf::mesh::load_mesh;
use crate::model::{Joint, Rig, RigRemapTable, SkeletalMesh};
use math::inverse_lerp;

pub fn load_animations(skeletal_mesh: &SkeletalMesh, file: &GltfFile) -> SkeletalAnimation {
    if file.animations.is_none() {
        panic!("no animations");
    }
    let animations = &file.animations.as_ref().unwrap();
    if animations.len() > 1 {
        panic!("multiple animations not yet supported");
    }

    let anim: &GltfAnimation = &animations[0];
    let anim_name = (&anim).name.clone();
    info!("load animation: {}, channels {}, samplers {}", anim.name, anim.channels.len(), anim.samplers.len());

    // load rig scale
    let scene = &file.scenes[0];
    let scene_root_nodes = scene.root_nodes.as_ref().unwrap();
    let rig_node: &GltfNode = &file.nodes[scene_root_nodes[0]];
    let mut rig_scale = 1.0;
    if rig_node.scale.is_some() {
        let components = rig_node.scale.as_ref().unwrap();
        if components[0] != components[1] || components[0] != components[2] {
            panic!("non uniform rig scale {},{},{}", components[0], components[1], components[2]);
        }
        rig_scale = components[0];
    }
    info!("rig scale {}", rig_scale);

    // read samples
    let mut sparse_channels: HashMap<usize, SparseChannel> = HashMap::new();
    for gltf_channel in anim.channels.iter() {
        // use remapped target bone index
        let bone_index = skeletal_mesh.rig.remap_table.joints[&gltf_channel.target.node];

        if !sparse_channels.contains_key(&bone_index) {
            sparse_channels.insert(bone_index, SparseChannel {
                translations: vec![],
                translation_times: vec![],
                translation_time_min: 0.0,
                translation_time_max: 0.0,
                rotations: vec![],
                rotation_times: vec![],
                rotation_time_min: 0.0,
                rotation_time_max: 0.0,
                scales: vec![],
                scale_times: vec![],
                scale_time_min: 0.0,
                scale_time_max: 0.0
            });
        }
        let mut channel = sparse_channels.get_mut(&bone_index).unwrap();

        let sampler = &anim.samplers[gltf_channel.sampler];
        let input_accessor = &file.accessors[sampler.input];
        let output_accessor = &file.accessors[sampler.output];

        if input_accessor.count != output_accessor.count {
            // todo doesn't hold true for some samplers
            panic!("sampler input/output count mismatch");
        }
        if input_accessor.component_type != GltfComponentType::Float as i64 {
            panic!("sampler input component type != float: {:?}", input_accessor.component_type);
        }
        if input_accessor.min.is_none() || input_accessor.max.is_none() {
            panic!("sampler input missing min|max bounds");
        }
        if input_accessor.accessor_type != "SCALAR" {
            panic!("sampler input incorrect accessor type {:?} expected SCALAR", input_accessor.accessor_type);
        }
        // TODO treat everything as linear for now
        /*if sampler.interpolation != "LINEAR" {
            panic!("unsupported sampler interpolation {}", sampler.interpolation);
        }*/

        let input_buffer_view = &file.buffer_views[input_accessor.buffer_view];
        let output_buffer_view = &file.buffer_views[output_accessor.buffer_view];
        let mut input_cursor = get_buffer_cursor(&file, input_buffer_view);
        let mut output_cursor = get_buffer_cursor(&file, output_buffer_view);

        for f in 0..input_accessor.count as usize {
            let frame_time = input_cursor.read_f32::<LittleEndian>().unwrap();
            if gltf_channel.target.path == "translation" {
                let x = output_cursor.read_f32::<LittleEndian>().unwrap();
                let y = output_cursor.read_f32::<LittleEndian>().unwrap();
                let z = output_cursor.read_f32::<LittleEndian>().unwrap();
                // note: scale translations by rig scale
                channel.translations.push(float3::new(x, y, z) * rig_scale);
                channel.translation_times.push(frame_time);
                channel.translation_time_min = input_accessor.min.as_ref().unwrap()[0];
                channel.translation_time_max = input_accessor.max.as_ref().unwrap()[0];
            } else if gltf_channel.target.path == "rotation" {
                let x = output_cursor.read_f32::<LittleEndian>().unwrap();
                let y = output_cursor.read_f32::<LittleEndian>().unwrap();
                let z = output_cursor.read_f32::<LittleEndian>().unwrap();
                let w = output_cursor.read_f32::<LittleEndian>().unwrap();
                channel.rotations.push(quaternion::new(x, y, z, w));
                channel.rotation_times.push(frame_time);
                channel.rotation_time_min = input_accessor.min.as_ref().unwrap()[0];
                channel.rotation_time_max = input_accessor.max.as_ref().unwrap()[0];
            } else if gltf_channel.target.path == "scale" {
                let x = output_cursor.read_f32::<LittleEndian>().unwrap();
                let y = output_cursor.read_f32::<LittleEndian>().unwrap();
                let z = output_cursor.read_f32::<LittleEndian>().unwrap();
                channel.scales.push(float3::new(x, y, z));
                channel.scale_times.push(frame_time);
                channel.scale_time_min = input_accessor.min.as_ref().unwrap()[0];
                channel.scale_time_max = input_accessor.max.as_ref().unwrap()[0];
            }
        }
    }

    for (joint, chan) in sparse_channels.iter() {
        let full_sample = chan.translations.len() == chan.rotations.len() && chan.translations.len() == chan.scales.len();
        if !full_sample {
            error!("NOT FULL SAMPLE FOR BONE {}", joint);
        }
    }

    // discover time bounds
    let mut min_time = 9999f32;
    let mut max_time = 0.0f32;
    for (_, channel) in sparse_channels.iter() {
        if channel.translation_time_min < min_time {
            min_time = channel.translation_time_min;
        }
        if channel.rotation_time_min < min_time {
            min_time = channel.rotation_time_min;
        }
        if channel.scale_time_min < min_time {
            min_time = channel.scale_time_min;
        }
        if channel.translation_time_max > max_time {
            max_time = channel.translation_time_max;
        }
        if channel.rotation_time_max > max_time {
            max_time = channel.rotation_time_max;
        }
        if channel.scale_time_max > max_time {
            max_time = channel.scale_time_max;
        }
    }
    debug!("animation time bounds: min={}, max={}", min_time, max_time);

    // make dense
    let sample_rate = 30.0f32;
    let time_step = 1.0 / sample_rate;
    let frame_count = (max_time / time_step).floor() as usize;
    let mut dense = SkeletalAnimation {
        name: anim_name,
        sample_rate,
        num_frames: frame_count,
        min_time,
        max_time,
        joints: vec![vec![TRS::default(); frame_count]; skeletal_mesh.rig.joint_count]
    };
    //make_dense_presampled(&mut dense, &skeletal_mesh.rig, &sparse_channels);
    make_dense(&mut dense, &skeletal_mesh.rig, time_step, &sparse_channels);

    /*debug!("finished loading animation: frames={}, frame_min={}, frame_max={}",
           animation.frame_count, animation.frame_min, animation.frame_max);*/
    return dense;
}

struct SparseChannel {
    pub translations: Vec<float3>,
    pub translation_times: Vec<f32>,
    pub translation_time_min: f32,
    pub translation_time_max: f32,

    pub rotations: Vec<quaternion>,
    pub rotation_times: Vec<f32>,
    pub rotation_time_min: f32,
    pub rotation_time_max: f32,

    pub scales: Vec<float3>,
    pub scale_times: Vec<f32>,
    pub scale_time_min: f32,
    pub scale_time_max: f32
}

fn make_dense(dense: &mut SkeletalAnimation, rig: &Rig, time_step: f32, sparse_channels: &HashMap<usize, SparseChannel>) {
    let identity_translation = float3::zero();
    let identity_rotation = quaternion::identity();
    let identity_scale = float3::one();
    for joint_idx in 0..rig.joint_count {
        if !sparse_channels.contains_key(&joint_idx) {
            continue;
        }
        let sparse = &sparse_channels[&joint_idx];
        let mut joint_frames: &mut Vec<TRS> = dense.joints.get_mut(joint_idx).unwrap();
        for frame in 0..dense.num_frames {
            let time = time_step * (frame as f32);

            let pair_translation: ((f32, &float3), (f32, &float3)) = select_keyframes::<float3>(time, &sparse.translation_times, &sparse.translations, &identity_translation);
            let pair_rotation = select_keyframes::<quaternion>(time, &sparse.rotation_times, &sparse.rotations, &identity_rotation);
            let pair_scale = select_keyframes::<float3>(time, &sparse.scale_times, &sparse.scales, &identity_scale);

            let alpha_translation = inverse_lerp((pair_translation.0).0, (pair_translation.1).0, time);
            let alpha_rotation = inverse_lerp((pair_rotation.0).0, (pair_rotation.1).0, time);
            let alpha_scale = inverse_lerp((pair_scale.0).0, (pair_scale.1).0, time);

            let translation = float3::lerp((pair_translation.0).1, (pair_translation.1).1, alpha_translation);
            let rotation = quaternion::slerp((pair_rotation.0).1, (pair_rotation.1).1, alpha_rotation);
            let scale = float3::lerp((pair_scale.0).1, (pair_scale.1).1, alpha_scale);

            joint_frames[frame] = TRS { translation, rotation, scale };
        }
    }
}

fn make_dense_presampled(dense: &mut SkeletalAnimation, rig: &Rig, sparse_channels: &HashMap<usize, SparseChannel>) {
    for joint_idx in 0..rig.joint_count {
        if !sparse_channels.contains_key(&joint_idx) {
            continue;
        }
        let sparse = &sparse_channels[&joint_idx];
        let mut joint_frames: &mut Vec<TRS> = dense.joints.get_mut(joint_idx).unwrap();
        let correct_sample = sparse.translations.len() == sparse.rotations.len() && sparse.translations.len() == sparse.scales.len();
        if !correct_sample {
            panic!("translations len != rotations len != scales len");
        }
        if sparse.translations.len() != dense.num_frames {
            panic!("expected num frames {} but got {}", dense.num_frames, sparse.translations.len());
        }
        for frame in 0..dense.num_frames {
            joint_frames[frame] = TRS { translation: sparse.translations[frame], rotation: sparse.rotations[frame], scale: sparse.scales[frame] };
        }
    }
}


/// case: no left keyframe: identity
/// case: no right keyframe: left keyframe
/// returns ((time, left), (time, right))
fn select_keyframes<'a, T>(time: f32, keyframe_times: &Vec<f32>, keyframes: &'a Vec<T>, identity: &'a T) -> ((f32, &'a T), (f32, &'a T)) {
    let (l, r) = search_keyframe_indices(time, keyframe_times);
    let mut left_time = 0.0f32;
    let mut right_time = 0.0f32;
    let mut left = if l.is_some() {
        let idx = l.unwrap();
        left_time = keyframe_times[idx];
        &keyframes[idx]
    } else {
        left_time = 0.0f32;
        &identity
    };
    let mut right = if r.is_some() {
        let idx = r.unwrap();
        right_time = keyframe_times[idx];
        &keyframes[idx]
    } else {
        right_time = left_time;
        left
    };
    return ((left_time, left), (right_time, right));
}

/// search keyframe indices
/// expects keyframe_times to be in chronological order
fn search_keyframe_indices(time: f32, keyframe_times: &Vec<f32>) -> (Option<usize>, Option<usize>) {
    let mut left = None;
    let mut right = None;
    // sweep from left to right and break on first occurrence of right
    for i in 0..keyframe_times.len() {
        let t = keyframe_times[i];
        if t < time {
            left = Some(i);
        }
        if t > time && right == None {
            right = Some(i);
            break;
        }
    }
    return (left, right);
}
#[cfg(test)]
#[test]
fn test_search_keyframes() {
    let test1_kf = vec![0.0, 1.0, 2.0, 3.0];
    let (test1_left, test1_right) = search_keyframe_indices(1.5, &test1_kf);
    assert_eq!(1, test1_left);
    assert_eq!(2, test1_right);
}

pub fn load_rig(file: &GltfFile) -> Rig {
    let skinned_mesh_node = file.nodes.iter()
        .find(|n| n.skin.is_some())
        .expect("no skinned mesh found");
    info!("skinned mesh {}", skinned_mesh_node.name);
    let file_skins = &file.skins.as_ref().unwrap();
    let skin_index = skinned_mesh_node.skin.unwrap();
    let skin = &file_skins[skin_index];
    let root_bone_node_index = skin.joints[0];
    let root_bone = &file.nodes[root_bone_node_index];
    info!("root bone: {}", root_bone.name);

    let skin_indices_array = skin.joints.clone();

    // load rig
    let mut rig = Rig {
        joint_transforms: Vec::new(),
        joint_count: 0,
        joint_names: Vec::new(),
        joint_children: Vec::new(),
        joint_parents: Vec::new(),
        remap_table: RigRemapTable { joints: HashMap::new() },
    };
    recur_build_rig(&mut rig, 0, root_bone_node_index, &file, root_bone);
    rig.joint_count = rig.joint_transforms.len();

    debug!("load_rig: finished with {} total bones", rig.joint_transforms.len());
    return rig;
}

/// depth-first recursive
/// source_bone_index = joint index in source animation file (for remap table)
fn recur_build_rig(rig: &mut Rig, parent_joint_index: usize, source_joint_index: usize, file: &GltfFile, node: &GltfNode) {
    let joint_index = rig.joint_transforms.len();

    // remap children as we walk the hierarchy (root joint is always index zero)
    rig.remap_table.joints.insert(source_joint_index, joint_index);
    debug!("remap source joint {} idx {} to {}", node.name, source_joint_index, joint_index);

    let joint = joint_from_gltf_node(node, joint_index, parent_joint_index);
    rig.joint_transforms.push(joint);
    rig.joint_names.push(node.name.clone());
    rig.joint_parents.push(parent_joint_index);
    rig.joint_children.push(Vec::<usize>::new());
    if joint_index != 0 {
        // add this joint to its parents child indices array
        rig.joint_children[parent_joint_index].push(joint_index);
    }
    if node.children.is_some() {
        // first add children
        let child_node_indices = node.children.as_ref().unwrap();
        for child_node_index in child_node_indices.iter() {
            let child_node = &file.nodes[*child_node_index];
            recur_build_rig(rig, joint_index, *child_node_index, file, child_node);
        }
    }
}

fn joint_from_gltf_node(node: &GltfNode, joint_index: usize, parent_joint_index: usize) -> Joint {
    let trs = trs_from_gltf_node(node);
    let mut joint = Joint {
        index: joint_index,
        parent_index: parent_joint_index,
        translation: trs.0,
        rotation: trs.1,
        scale: trs.2
    };
    return joint;
}

pub fn load_skeletal_entity(name: &str, file: &GltfFile) -> SkeletalMesh {
    info!("begin load skinned mesh {}", name);
    let mut mesh = load_mesh(name, file);
    if mesh.attribs.joint_indices.len() == 0 {
        panic!("mesh has no joint indices");
    }
    if mesh.attribs.joint_weights.len() == 0 {
        panic!("mesh has no joint weights");
    }

    let rig = load_rig(file);
    info!("rig hierarchy:");
    fn print_rig_node(rig: &Rig, idx: usize) {
        info!("{} children {}", rig.joint_names[idx], rig.joint_children[idx].len());
        for c in rig.joint_children[idx].iter() {
            print_rig_node(rig, *c);
        }
    }
    print_rig_node(&rig, 0);

    debug!("remap mesh joint_indices");
    let mut joint_indices = mesh.attribs.joint_indices.clone();
    for i in 0..joint_indices.len() {
        let mut remapped_indices = joint_indices[i];
        fn remap(idx: usize, remap_table: &RigRemapTable) -> usize {
            return *remap_table.joints.get(&idx).or(Option::Some(&idx)).unwrap();
        }
        remapped_indices.x = remap(remapped_indices.x as usize, &rig.remap_table) as i32;
        remapped_indices.y = remap(remapped_indices.y as usize, &rig.remap_table) as i32;
        remapped_indices.z = remap(remapped_indices.z as usize, &rig.remap_table) as i32;
        remapped_indices.w = remap(remapped_indices.w as usize, &rig.remap_table) as i32;
        joint_indices[i] = remapped_indices;
    }
    mesh.attribs.joint_indices = joint_indices;

    // build joint local transforms
    let mut joint_local_transforms: Vec<float4x4> = vec![matrix4x4_identity(); rig.joint_count];
    let mut joint_transforms: Vec<TRS> = vec![TRS::default(); rig.joint_count];
    for joint_idx in 0..rig.joint_count {
        let joint = rig.joint_transforms[joint_idx];
        joint_local_transforms[joint_idx] = matrix4x4_trs(&joint.translation, &joint.rotation, &joint.scale);
        joint_transforms[joint_idx] = TRS { translation: joint.translation, rotation: joint.rotation, scale: joint.scale };
    }

    // build inverse binds
    let mut inverse_bind_matrices: Vec<float4x4> = vec![matrix4x4_identity(); rig.joint_count];
    recur_build_inverse_bind_matrices(&rig, &joint_local_transforms, &mut inverse_bind_matrices,
                                      0, &matrix4x4_identity());

    return SkeletalMesh {
        mesh,
        rig,
        joint_local_transforms,
        joint_transforms,
        inverse_bind_matrices
    }
}

fn recur_build_inverse_bind_matrices(rig: &Rig, joint_local_transforms: &Vec<float4x4>, inverse_bind_matrices: &mut Vec<float4x4>, joint_index: usize, parent: &float4x4) {
    let joint_local_transform = joint_local_transforms[joint_index];
    let bind_pose = matrix4x4_mul(&parent, &joint_local_transform);
    inverse_bind_matrices[joint_index] = matrix4x4_inverse(&bind_pose);
    for child_joint in rig.joint_children[joint_index].iter() {
        recur_build_inverse_bind_matrices(rig, joint_local_transforms, inverse_bind_matrices, *child_joint, &bind_pose);
    }
}