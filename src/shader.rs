use gles3::gles::*;
use gl::types::*;
use std::ffi::{CString, CStr};
use std::mem;

pub const MAX_PROGRAM_UNIFORMS: usize = 8;
pub const MAX_PROGRAM_TEXTURES: usize = 8;

pub struct ShaderProgram {
    pub program: GLuint,
    pub vertex_shader: GLuint,
    pub fragment_shader: GLuint,
    pub model_matrix: ProgramUniform,
    pub uniforms: [ProgramUniform; 4],
    // these will be -1 if not used by the program
    pub uniform_location: [GLint; MAX_PROGRAM_UNIFORMS],
    pub uniform_binding: [GLint; MAX_PROGRAM_UNIFORMS],
    pub textures: [GLint; MAX_PROGRAM_TEXTURES],
}

pub struct ProgramUniform {
    pub index: ProgramUniformIndex,
    pub uniform_type: ProgramUniformType,
    pub name: &'static str
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ProgramUniformIndex {
    UniformModelMatrix = 0,
    UniformViewId = 1,
    UniformSceneMatrices = 2,
    UniformJointMatrices = 3
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ProgramUniformType {
    UniformTypeVector4 = 0,
    UniformTypeMatrix4x4 = 1,
    UniformTypeInt = 2,
    UniformTypeBuffer = 3
}

pub const VertexAttributeLocationPosition: GLuint = 0;
pub const VertexAttributeLocationNormal: GLuint = 1;
pub const VertexAttributeLocationTangent: GLuint = 2;
pub const VertexAttributeLocationBinormal: GLuint = 3;
pub const VertexAttributeLocationColor: GLuint = 4;
pub const VertexAttributeLocationUv0: GLuint = 5;
pub const VertexAttributeLocationUv1: GLuint = 6;
pub const VertexAttributeLocationJointIndices: GLuint = 7;
pub const VertexAttributeLocationJointWeights: GLuint = 8;
pub const VertexAttributeLocationFontParams: GLuint = 9;

pub const NUM_PROGRAMS: usize = 4;
#[derive(Copy, Clone, Debug)]
pub enum ProgramId {
    VertexColor = 0,
    SingleTexture = 1,
    SingleTextureSkinned1 = 2,
    SingleTextureSkinned4 = 3
}
const PROGRAM_SOURCES: [[&str; 2]; NUM_PROGRAMS] = [
    [SOLID_COLOR_VERTEX_SHADER, SOLID_COLOR_FRAGMENT_SHADER],
    [SINGLE_TEXTURE_VERTEX_SHADER, SINGLE_TEXTURE_FRAGMENT_SHADER],
    [SINGLE_TEXTURE_SKINNED1_VERTEX_SHADER, SINGLE_TEXTURE_SKINNED1_FRAGMENT_SHADER],
    [SINGLE_TEXTURE_SKINNED4_VERTEX_SHADER, SINGLE_TEXTURE_SKINNED4_FRAGMENT_SHADER]
];

pub fn build_shader_program(program_id: ProgramId, multiview: bool) -> ShaderProgram {
    debug!("build_shader_program: begin {:?}", program_id);
    let mut program: ShaderProgram = unsafe { mem::zeroed() };
    unsafe {
        let mut r: GLint = 1;

        let mut vertex_source: String = PROGRAM_VERSION.to_owned();
        if !multiview {
            vertex_source.push_str("#define DISABLE_MULTIVIEW 1\n")
        }
        vertex_source.push_str(VERTEX_HEADER);
        vertex_source.push_str(PROGRAM_SOURCES[program_id as usize][0]);

        let mut fragment_source = PROGRAM_VERSION.to_owned();
        fragment_source.push_str(FRAGMENT_HEADER);
        fragment_source.push_str(PROGRAM_SOURCES[program_id as usize][1]);

        program.vertex_shader = glCreateShader(GL_VERTEX_SHADER);
        glShaderSource(program.vertex_shader, 1, &(CString::new(vertex_source).unwrap().as_ptr()), std::ptr::null());
        glCompileShader(program.vertex_shader);
        glGetShaderiv(program.vertex_shader, GL_COMPILE_STATUS, &mut r);
        if r == 0 {
            let mut err_buf: Vec<u8> = Vec::with_capacity(4096);
            err_buf.extend([b' '].iter().cycle().take(err_buf.capacity()));
            let err = unsafe { CString::from_vec_unchecked(err_buf) };
            let mut len: GLint = 0;
            glGetShaderInfoLog(program.vertex_shader, 4096, &mut len, err.as_ptr() as *mut GLchar);
            eprintln!("error compiling vertex shader len {:?}: {:?}", len, err);
        }

        program.fragment_shader = glCreateShader(GL_FRAGMENT_SHADER);
        glShaderSource(program.fragment_shader, 1, &CString::new(fragment_source).unwrap().as_ptr(), std::ptr::null());
        glCompileShader(program.fragment_shader);
        glGetShaderiv(program.fragment_shader, GL_COMPILE_STATUS, &mut r);
        if r == 0 {
            let mut err_buf: Vec<u8> = Vec::with_capacity(4096);
            err_buf.extend([b' '].iter().cycle().take(err_buf.capacity()));
            let err = unsafe { CString::from_vec_unchecked(err_buf) };
            let mut len: GLint = 0;
            glGetShaderInfoLog(program.fragment_shader, 4096, &mut len, err.as_ptr() as *mut GLchar/* *(&(msg[0])) as *mut GLchar */);
            eprintln!("error compiling fragment shader len {:?}: {:?}", len, err);
        }

        program.program = glCreateProgram();
        glAttachShader(program.program, program.vertex_shader);
        glAttachShader(program.program, program.fragment_shader);

        // set attributes before linking
        glBindAttribLocation(program.program, VertexAttributeLocationPosition, "Position\0".as_ptr() as *const _ as *const GLchar);
        glBindAttribLocation(program.program, VertexAttributeLocationNormal, "Normal\0".as_ptr() as *const _ as *const GLchar);
        glBindAttribLocation(program.program, VertexAttributeLocationTangent, "Tangent\0".as_ptr() as *const _ as *const GLchar);
        glBindAttribLocation(program.program, VertexAttributeLocationBinormal, "Binormal\0".as_ptr() as *const _ as *const GLchar);
        glBindAttribLocation(program.program, VertexAttributeLocationColor, "Color\0".as_ptr() as *const _ as *const GLchar);
        glBindAttribLocation(program.program, VertexAttributeLocationUv0, "TexCoord\0".as_ptr() as *const _ as *const GLchar);
        glBindAttribLocation(program.program, VertexAttributeLocationUv1, "TexCoord1\0".as_ptr() as *const _ as *const GLchar);
        glBindAttribLocation(program.program, VertexAttributeLocationJointIndices, "JointIndices\0".as_ptr() as *const _ as *const GLchar);
        glBindAttribLocation(program.program, VertexAttributeLocationJointWeights, "JointWeights\0".as_ptr() as *const _ as *const GLchar);
        glBindAttribLocation(program.program, VertexAttributeLocationFontParams, "FontParams\0".as_ptr() as *const _ as *const GLchar);

        glLinkProgram(program.program);
        glGetProgramiv(program.program, GL_LINK_STATUS, &mut r);
        if r == 0 {
            panic!("program link failed");
        }

        // get the uniform locations
        program.uniforms = [
            ProgramUniform { index: ProgramUniformIndex::UniformModelMatrix, uniform_type: ProgramUniformType::UniformTypeMatrix4x4, name: "ModelMatrix"},
            ProgramUniform { index: ProgramUniformIndex::UniformViewId, uniform_type: ProgramUniformType::UniformTypeInt, name: "ViewID"},
            ProgramUniform { index: ProgramUniformIndex::UniformSceneMatrices, uniform_type: ProgramUniformType::UniformTypeBuffer, name: "SceneMatrices"},
            ProgramUniform { index: ProgramUniformIndex::UniformJointMatrices, uniform_type: ProgramUniformType::UniformTypeBuffer, name: "JointMatrices"}
        ];

        let mut num_buffer_bindings = 0;
        for i in 0..program.uniform_location.len() as usize {
            // default to -1
            program.uniform_location[i] = -1;
        }
        for i in 0..program.uniforms.len() {
            let uniform = &program.uniforms[i];
            let uniform_index = uniform.index as usize;
            let mut uname = uniform.name.to_owned();
            uname.push_str("\0");
            let uniform_name_cstr = CStr::from_ptr(uname.as_ptr().cast()).as_ptr();

            if uniform.uniform_type == ProgramUniformType::UniformTypeBuffer {
                program.uniform_location[uniform_index] = glGetUniformBlockIndex(program.program, uniform_name_cstr) as i32;
                program.uniform_binding[uniform_index] = num_buffer_bindings;
                num_buffer_bindings += 1;
                glUniformBlockBinding(program.program, program.uniform_location[uniform_index] as u32, program.uniform_binding[uniform_index] as u32);
            } else {
                program.uniform_location[uniform_index] = glGetUniformLocation(program.program, uniform_name_cstr);
                program.uniform_binding[uniform_index] = program.uniform_location[uniform_index];
            }
            debug!("uniform_bind: {} type {:?} at {}", program.uniforms[i].name, program.uniforms[i].uniform_type, program.uniform_location[i]);
        }

        glUseProgram(program.program);

        // implicit texture bindings
        for i in 0..MAX_PROGRAM_TEXTURES {
            let texture_name = format!("Texture{}\0", i);
            let name = CStr::from_ptr(texture_name.as_ptr().cast()).as_ptr();
            program.textures[i] = glGetUniformLocation(program.program, name);
            if program.textures[i] != -1 {
                debug!("program texture {} location: {}", i, program.textures[i]);
                glUniform1i(program.textures[i], i as i32);
            }
        }

        glUseProgram(0);
    }
    return program;
}

pub fn destroy_shader_program(program: &mut ShaderProgram) {
    unsafe {
        if program.program != 0 {
            glDeleteProgram(program.program);
            program.program = 0;
        }
        if program.vertex_shader != 0 {
            glDeleteShader(program.vertex_shader);
            program.vertex_shader = 0;
        }
        if program.fragment_shader != 0 {
            glDeleteShader(program.fragment_shader);
            program.fragment_shader = 0;
        }
    }
}

const PROGRAM_VERSION: &str = "#version 300 es\n";

const VERTEX_HEADER: &str = r#"
#ifndef DISABLE_MULTIVIEW
    #define DISABLE_MULTIVIEW 0
#endif
#define NUM_VIEWS 2
#if defined( GL_OVR_multiview2 ) && ! DISABLE_MULTIVIEW
    #extension GL_OVR_multiview2 : enable
    layout(num_views=NUM_VIEWS) in;
    #define VIEW_ID gl_ViewID_OVR
#else
    uniform int ViewID;
    #define VIEW_ID ViewID
#endif
uniform mat4 ModelMatrix;
uniform SceneMatrices
{
	uniform mat4 ViewMatrix[2];
	uniform mat4 ProjectionMatrix[2];
} sm;
#define TransformVertex(localPos) (sm.ProjectionMatrix[VIEW_ID] * ( sm.ViewMatrix[VIEW_ID] * ( ModelMatrix * localPos )))
"#;

const FRAGMENT_HEADER: &str = r#"
#define gl_FragColor fragColor
out mediump vec4 fragColor;
#define texture2D texture
#define textureCube texture
"#;

/// - Solid color
pub const SOLID_COLOR_VERTEX_SHADER: &str = r#"
in vec4 Position;
//in vec4 vertexColor;
out vec4 fragmentColor;
void main()
{
	gl_Position = TransformVertex(Position);//sm.ProjectionMatrix[VIEW_ID] * (sm.ViewMatrix[VIEW_ID] * (ModelMatrix * Position));
	fragmentColor = vec4(0.4, 0.4, 0.8, 1);//vertexColor;
}
"#;
pub const SOLID_COLOR_FRAGMENT_SHADER: &str = r#"
in lowp vec4 fragmentColor;
void main()
{
	gl_FragColor = fragmentColor;
}
"#;

/// -- Single texture
pub const SINGLE_TEXTURE_VERTEX_SHADER: &str = r#"
in highp vec4 Position;
in highp vec2 TexCoord;
out highp vec2 oTexCoord;
void main() {
    gl_Position = TransformVertex(Position);
    oTexCoord = TexCoord;
}
"#;

pub const SINGLE_TEXTURE_FRAGMENT_SHADER: &str = r#"
uniform sampler2D Texture0;
in highp vec2 oTexCoord;
void main() {
    gl_FragColor = texture2D(Texture0, oTexCoord);
}
"#;

/// -- Single texture skinned - single joint
pub const SINGLE_TEXTURE_SKINNED1_VERTEX_SHADER: &str = r#"
uniform JointMatrices {
    highp mat4 Joints[64];
} jb;
in highp vec4 Position;
in highp vec2 TexCoord;
in highp vec4 JointWeights;
in highp vec4 JointIndices;
out highp vec2 oTexCoord;
void main() {
    highp vec4 localPos = jb.Joints[int(JointIndices.x)] * Position;
    gl_Position = TransformVertex(localPos);
    oTexCoord = TexCoord;
}
"#;

pub const SINGLE_TEXTURE_SKINNED1_FRAGMENT_SHADER: &str = r#"
uniform sampler2D Texture0;
in highp vec2 oTexCoord;
void main() {
    gl_FragColor = texture2D(Texture0, oTexCoord);
}
"#;

pub const SINGLE_TEXTURE_SKINNED4_VERTEX_SHADER: &str = r#"
uniform JointMatrices {
    highp mat4 Joints[64];
} jb;
in highp vec4 Position;
in highp vec2 TexCoord;
in highp vec4 JointWeights;
in highp vec4 JointIndices;
out highp vec2 oTexCoord;
void main() {
    highp vec4 localPos1 = jb.Joints[int(JointIndices.x)] * Position;
    highp vec4 localPos2 = jb.Joints[int(JointIndices.y)] * Position;
    highp vec4 localPos3 = jb.Joints[int(JointIndices.z)] * Position;
    highp vec4 localPos4 = jb.Joints[int(JointIndices.w)] * Position;
    highp vec4 localPos = localPos1 * JointWeights.x + localPos2 * JointWeights.y + localPos3 * JointWeights.z + localPos4 * JointWeights.w;
    gl_Position = TransformVertex(localPos);
    oTexCoord = TexCoord;
}
"#;
pub const SINGLE_TEXTURE_SKINNED4_FRAGMENT_SHADER: &str = r#"
uniform sampler2D Texture0;
in highp vec2 oTexCoord;
void main() {
    gl_FragColor = texture2D(Texture0, oTexCoord);
}
"#;