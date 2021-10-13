use std::iter::FromIterator;

use gl;
use gl::types::*;
use gles3::gles::*;
use ovr_mobile_sys as ovr;
use ovr_mobile_sys::{vrapi_CreateTextureSwapChain2, vrapi_CreateTextureSwapChain3, vrapi_DestroyTextureSwapChain, vrapi_GetTextureSwapChainHandle, vrapi_GetTextureSwapChainLength};
use ovr_mobile_sys::ovrSystemProperty::*;
use ovr_mobile_sys::ovrTextureType_::{VRAPI_TEXTURE_TYPE_2D, VRAPI_TEXTURE_TYPE_2D_ARRAY};

use crate::*;
use crate::framebuffer;
use crate::framebuffer::OvrFramebuffer;
use crate::vrapi;

const NUM_MULTI_SAMPLES: i32 = 4;

macro_rules! gl {
    ($cmd:expr) => {
        unsafe { $cmd }
    }
}

pub struct OvrRenderer {
    pub num_buffers: i32,
    pub frame_buffers: [OvrFramebuffer; ovr::ovrFrameLayerEye::VRAPI_FRAME_LAYER_EYE_MAX as usize],
    pub eyefov_x: f32,
    pub eyefov_y: f32
}

pub struct GlExtensions {
    pub multi_view: bool, // GL_OVR_multiview, GL_OVR_multiview2
    pub EXT_texture_border_clamp: bool // GL_EXT_texture_border_clamp, GL_OES_texture_border_clamp
}

pub fn bind_scene_matrices_ubo(eye: i32, program: &ShaderProgram, scene_matrices: GlBuffer) {
    unsafe {
        glBindBufferBase(GL_UNIFORM_BUFFER,
                         program.uniform_binding[shader::ProgramUniformIndex::UniformSceneMatrices as usize] as u32,
                         scene_matrices.buffer);
        // NOTE: will not be present when multiview path is enabled.
        let l = program.uniform_location[shader::ProgramUniformIndex::UniformViewId as usize];
        if l >= 0 {
            glUniform1i(l, eye);
        }
    }
}

pub fn ovr_renderer_create(app_state: &OvrApp, renderer: &mut OvrRenderer) {
    renderer.num_buffers = if app_state.multiview {
        1
    } else {
        ovr::ovrFrameLayerEye::VRAPI_FRAME_LAYER_EYE_MAX as i32
    };

    println!("ovr_renderer_create num_eye_buffers {}", renderer.num_buffers);

    // Create the frame buffers.
    for eye in 0..renderer.num_buffers {
        unsafe {
            framebuffer::ovr_framebuffer_create(&app_state.egl,
                                   &mut renderer.frame_buffers[eye as usize],
                                   app_state.multiview, gles::GL_RGBA8,
                                   vrapi::get_system_property_int(&app_state.java, VRAPI_SYS_PROP_SUGGESTED_EYE_TEXTURE_WIDTH),
                                   vrapi::get_system_property_int(&app_state.java, VRAPI_SYS_PROP_SUGGESTED_EYE_TEXTURE_HEIGHT),
                                   NUM_MULTI_SAMPLES)
        }
    }
    println!("ovr_renderer_create finished");
    /*for ( int eye = 0; eye < renderer->NumBuffers; eye++ ){
        ovrFramebuffer_Create( &renderer->FrameBuffer[eye], useMultiview,
        GL_RGBA8,
        vrapi_GetSystemPropertyInt( java, VRAPI_SYS_PROP_SUGGESTED_EYE_TEXTURE_WIDTH ),
        vrapi_GetSystemPropertyInt( java, VRAPI_SYS_PROP_SUGGESTED_EYE_TEXTURE_HEIGHT ),
        NUM_MULTI_SAMPLES );

    }*/


    renderer.eyefov_x = vrapi::get_system_property_float(&app_state.java, VRAPI_SYS_PROP_SUGGESTED_EYE_FOV_DEGREES_X );
    renderer.eyefov_y = vrapi::get_system_property_float(&app_state.java, VRAPI_SYS_PROP_SUGGESTED_EYE_FOV_DEGREES_Y );
    println!("eye fovs {},{}", renderer.eyefov_x, renderer.eyefov_y);
}

pub fn ovr_renderer_destroy(renderer: &mut OvrRenderer) {
    for eye in 0..renderer.num_buffers as usize {
        framebuffer::ovr_framebuffer_destroy(&mut renderer.frame_buffers[eye]);
    }
}

pub fn gl_errchk() {
    unsafe {
        let err = glGetError();
        if err != GL_NO_ERROR {
            eprintln!("GL ERROR: {}", err);
        }
    }
}
