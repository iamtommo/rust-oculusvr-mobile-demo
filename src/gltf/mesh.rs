use byteorder::{LittleEndian, ReadBytesExt};
use math::vector::{float2, float3, float4, int4};

use crate::gltf::{get_buffer_cursor, GltfComponentType, GltfFile};
use crate::model::Mesh;
use crate::render::gl_geometry::VertexAttribs;

pub fn load_mesh(name: &str, file: &GltfFile) -> Mesh {
    info!("begin load mesh {}", name);
    let mut loaded_mesh: Mesh = Mesh {
        attribs: VertexAttribs {
            position: Vec::new(),
            normal: Vec::new(),
            tangent: Vec::new(),
            binormal: Vec::new(),
            color: Vec::new(),
            uv0: Vec::new(),
            uv1: Vec::new(),
            joint_indices: Vec::new(),
            joint_weights: Vec::new()
        },
        indices: Vec::new()
    };

    let meshes = file.meshes.as_ref().unwrap();
    let mesh = &meshes[0];
    let mesh_primitives = &mesh.primitives;
    let mesh_primitive = &mesh_primitives[0];

    // load mesh indices
    if mesh_primitive.indices.is_some() {
        let indices_accessor = &file.accessors[mesh_primitive.indices.unwrap()];
        let indices_buffer_view = &file.buffer_views[indices_accessor.buffer_view];
        if indices_accessor.accessor_type != "SCALAR" || indices_accessor.component_type != GltfComponentType::UnsignedShort as i64 {
            panic!("unsupported accessor type for INDICES");
        }

        let mut cursor = get_buffer_cursor(&file, &indices_buffer_view);
        for i in 0..indices_accessor.count {
            let index = cursor.read_u16::<LittleEndian>().unwrap();
            loaded_mesh.indices.push(index);
        }
    }

    // load mesh attributes
    for (attr_key, attr_val) in &mesh_primitive.attributes {
        let accessor_index = *attr_val;
        if attr_key == "POSITION" {
            let position_accessor = &file.accessors[accessor_index];
            let position_buffer_view = &file.buffer_views[position_accessor.buffer_view];
            if position_accessor.accessor_type != "VEC3" || position_accessor.component_type != GltfComponentType::Float as i64 {
                panic!("unsupported accessor type for POSITION");
            }

            let mut cursor = get_buffer_cursor(&file, &position_buffer_view);
            for _ in 0..position_accessor.count {
                let x = cursor.read_f32::<LittleEndian>().unwrap();
                let y = cursor.read_f32::<LittleEndian>().unwrap();
                let z = cursor.read_f32::<LittleEndian>().unwrap();
                let vertex = float3::new(x, y, z);
                loaded_mesh.attribs.position.push(vertex);
            }
        } else if attr_key == "TEXCOORD_0" {
            let uv_accessor = &file.accessors[accessor_index];
            let uv_buffer_view = &file.buffer_views[uv_accessor.buffer_view];
            if uv_accessor.accessor_type != "VEC2" || uv_accessor.component_type != GltfComponentType::Float as i64 {
                panic!("unsupported accessor type for TEXCOORD_0");
            }

            let mut cursor = get_buffer_cursor(&file, &uv_buffer_view);
            for _ in 0..uv_accessor.count {
                let x = cursor.read_f32::<LittleEndian>().unwrap();
                let y = cursor.read_f32::<LittleEndian>().unwrap();
                loaded_mesh.attribs.uv0.push(float2::new(x, y));
            }
        } else if attr_key == "JOINTS_0" {
            let joints0_accessor_index = *mesh_primitive.attributes.get("JOINTS_0").unwrap();
            let joints0_accessor = &file.accessors[joints0_accessor_index];
            if joints0_accessor.accessor_type != "VEC4" {
                panic!("unsupported accessor type for JOINTS_0");
            }
            let joints0_buffer_view = &file.buffer_views[joints0_accessor.buffer_view];
            let mut cursor = get_buffer_cursor(&file, &joints0_buffer_view);
            for _ in 0..joints0_accessor.count {
                if joints0_accessor.component_type == GltfComponentType::UnsignedByte as i64 {
                    let b1 = cursor.read_u8().unwrap() as i32;
                    let b2 = cursor.read_u8().unwrap() as i32;
                    let b3 = cursor.read_u8().unwrap() as i32;
                    let b4 = cursor.read_u8().unwrap() as i32;
                    loaded_mesh.attribs.joint_indices.push(int4::new(b1, b2, b3, b4));
                } else if joints0_accessor.component_type == GltfComponentType::UnsignedShort as i64 {
                    let b1 = cursor.read_u16::<LittleEndian>().unwrap() as i32;
                    let b2 = cursor.read_u16::<LittleEndian>().unwrap() as i32;
                    let b3 = cursor.read_u16::<LittleEndian>().unwrap() as i32;
                    let b4 = cursor.read_u16::<LittleEndian>().unwrap() as i32;
                    loaded_mesh.attribs.joint_indices.push(int4::new(b1, b2, b3, b4));
                } else {
                    panic!("unsupported component type for JOINTS_0")
                }
            }
        } else if attr_key == "WEIGHTS_0" {
            let weights0_accessor_index = *mesh_primitive.attributes.get("WEIGHTS_0").unwrap();
            let weights0_accessor = &file.accessors[weights0_accessor_index];
            if weights0_accessor.accessor_type != "VEC4" {
                panic!("unsupported accessor type for WEIGHTS_0");
            }
            let weights0_buffer_view = &file.buffer_views[weights0_accessor.buffer_view];
            let mut cursor = get_buffer_cursor(&file, &weights0_buffer_view);
            for _ in 0..weights0_accessor.count {
                if weights0_accessor.component_type == GltfComponentType::Float as i64 {
                    let w1 = cursor.read_f32::<LittleEndian>().unwrap();
                    let w2 = cursor.read_f32::<LittleEndian>().unwrap();
                    let w3 = cursor.read_f32::<LittleEndian>().unwrap();
                    let w4 = cursor.read_f32::<LittleEndian>().unwrap();
                    loaded_mesh.attribs.joint_weights.push(float4::new(w1, w2, w3, w4));
                } else {
                    // can also be normalized u8 or normalized u16
                    panic!("yet unsupported WEIGHTS_0 component type");
                }
            }
        }
    }

    info!("finished loading mesh: positions={}, indices={}, uvs={}, colors={}, normals={},\
                joint_indices={}, joint_weights={}",
          loaded_mesh.attribs.position.len(), loaded_mesh.indices.len(), loaded_mesh.attribs.uv0.len(),
          loaded_mesh.attribs.color.len(), loaded_mesh.attribs.normal.len(), loaded_mesh.attribs.joint_indices.len(),
          loaded_mesh.attribs.joint_weights.len());

    return loaded_mesh;
}