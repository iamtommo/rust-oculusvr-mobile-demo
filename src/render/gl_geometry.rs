use crate::model::Mesh;
use gles3::gles::*;
use gl::types::*;
use std::ffi::c_void;
use crate::shader::{VertexAttributeLocationPosition, VertexAttributeLocationNormal, VertexAttributeLocationJointIndices, VertexAttributeLocationTangent, VertexAttributeLocationBinormal, VertexAttributeLocationColor, VertexAttributeLocationUv0, VertexAttributeLocationUv1, VertexAttributeLocationJointWeights};
use math::vector::{float4, float2, float3, int4};

pub const MAX_JOINTS: i32 = 64;

#[derive(Copy, Clone, Debug)]
pub struct GlGeometry {
    pub vertex_buffer: u32,
    pub index_buffer: u32,
    pub vertex_array_object: u32,
    pub primitive_type: u32,//// GL_TRIANGLES / GL_LINES / GL_POINTS / etc
    pub vertex_count: i32,
    pub index_count: i32
}

pub struct VertexAttribs {
    pub position: Vec<float3>,
    pub normal: Vec<float3>,
    pub tangent: Vec<float3>,
    pub binormal: Vec<float3>,
    pub color: Vec<float4>,
    pub uv0: Vec<float2>,
    pub uv1: Vec<float2>,
    pub joint_indices: Vec<int4>,
    pub joint_weights: Vec<float4>
}

pub fn make_geometry(attribs: &VertexAttribs, indices: &Vec<u16>) -> GlGeometry {
    let mut vao: GLuint = 0;
    let mut vertex_buffer: GLuint = 0;
    let mut index_buffer: GLuint = 0;
    unsafe {
        glGenBuffers(1, &mut vertex_buffer);
        glGenBuffers(1, &mut index_buffer);
        glGenVertexArrays(1, &mut vao);
        glBindVertexArray(vao);
        glBindBuffer(GL_ARRAY_BUFFER, vertex_buffer);

        let mut packed: Vec<u8> = Vec::new();
        pack_vertex_attribute(&mut packed, &attribs.position, VertexAttributeLocationPosition, GL_FLOAT, 3);
        pack_vertex_attribute(&mut packed, &attribs.normal, VertexAttributeLocationNormal, GL_FLOAT, 3);
        pack_vertex_attribute(&mut packed, &attribs.tangent, VertexAttributeLocationTangent, GL_FLOAT, 3);
        pack_vertex_attribute(&mut packed, &attribs.binormal, VertexAttributeLocationBinormal, GL_FLOAT, 3);
        pack_vertex_attribute(&mut packed, &attribs.color, VertexAttributeLocationColor, GL_FLOAT, 4);
        pack_vertex_attribute(&mut packed, &attribs.uv0, VertexAttributeLocationUv0, GL_FLOAT, 2);
        pack_vertex_attribute(&mut packed, &attribs.uv1, VertexAttributeLocationUv1, GL_FLOAT, 2);
        pack_vertex_attribute(&mut packed, &attribs.joint_indices, VertexAttributeLocationJointIndices, GL_INT, 4);
        pack_vertex_attribute(&mut packed, &attribs.joint_weights, VertexAttributeLocationJointWeights, GL_FLOAT, 4);

        glBufferData(GL_ARRAY_BUFFER, packed.len() as isize, packed.as_ptr() as *const _ as *const c_void, GL_STATIC_DRAW);

        glBindBuffer(GL_ELEMENT_ARRAY_BUFFER, index_buffer);
        glBufferData(GL_ELEMENT_ARRAY_BUFFER, (indices.len() * 2) as GLsizeiptr,
                     indices.as_ptr() as *const _ as *const c_void, GL_STATIC_DRAW);

        glBindVertexArray(0);

        glDisableVertexAttribArray(VertexAttributeLocationPosition);
        glDisableVertexAttribArray(VertexAttributeLocationNormal);
        glDisableVertexAttribArray(VertexAttributeLocationTangent);
        glDisableVertexAttribArray(VertexAttributeLocationBinormal);
        glDisableVertexAttribArray(VertexAttributeLocationColor);
        glDisableVertexAttribArray(VertexAttributeLocationUv0);
        glDisableVertexAttribArray(VertexAttributeLocationUv1);
        glDisableVertexAttribArray(VertexAttributeLocationJointIndices);
        glDisableVertexAttribArray(VertexAttributeLocationJointWeights);
    }

    GlGeometry {
        vertex_array_object: vao,
        vertex_buffer: vertex_buffer,
        index_buffer: index_buffer,
        primitive_type: GL_TRIANGLES,
        vertex_count: attribs.position.len() as i32,
        index_count: indices.len() as i32
    }
}

pub fn pack_vertex_attribute<T>(packed: &mut Vec<u8>, attrib: &Vec<T>, gl_location: GLuint,
                        gl_type: u32, gl_components: i32) {
    unsafe {
        if attrib.len() == 0 {
            glDisableVertexAttribArray(gl_location);
            return;
        }

        let attrib_size = std::mem::size_of_val(&attrib[0]);
        let offset: usize = packed.len();
        let size: usize = attrib.len() * attrib_size;

        packed.resize(offset + size, 0);
        std::ptr::copy::<u8>(attrib.as_ptr() as *const u8, packed.as_mut_ptr().offset(offset as isize), size);

        glEnableVertexAttribArray(gl_location);
        glVertexAttribPointer(gl_location, gl_components, gl_type, GL_FALSE,
                              attrib_size as i32, offset as *const c_void);
    }
}