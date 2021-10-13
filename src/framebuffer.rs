use std::ffi::CString;
use std::iter::FromIterator;

use egl::eglext::eglGetProcAddress;
use gles3::gles::*;
use ovr_mobile_sys as ovr;
use gl::types::*;

use crate::egl::OvrEgl;

pub struct OvrFramebuffer {
    pub width: i32,
    pub height: i32,
    pub multisamples: i32,
    pub texture_swap_chain_length: i32,
    pub texture_swap_chain_index: i32,
    pub multiview: bool,
    pub color_texture_swap_chain: *mut ovr::ovrTextureSwapChain,
    pub depth_buffers: Vec<gl::types::GLuint>,
    pub frame_buffers: Vec<gl::types::GLuint>
}

pub unsafe fn ovr_framebuffer_create(egl: &OvrEgl, framebuffer: &mut OvrFramebuffer, multiview: bool,
                                     color_format: gl::types::GLenum, width: i32, height: i32, multisamples: i32) {
    let gl_renderbuffer_storage_multisample_ext = egl_get_proc_address("glRenderbufferStorageMultisampleEXT");

    let gl_framebuffer_texture_2d_multisample_ext = egl_get_proc_address("glFramebufferTexture2DMultisampleEXT");

    let gl_framebuffer_texture_multiview_ovr = egl_get_proc_address("glFramebufferTextureMultiviewOVR");
    let gl_framebuffer_texture_multiview_ovr_fn = std::mem::transmute::<*const u8, fn(u32, u32, u32, u32, u32, u32)>(gl_framebuffer_texture_multiview_ovr);

    let gl_framebuffer_texture_multisample_multiview_ovr = egl_get_proc_address("glFramebufferTextureMultisampleMultiviewOVR");
    let gl_framebuffer_texture_multisample_multiview_ovr_fn = std::mem::transmute::<*const u8, fn(u32, u32, u32, u32, u32, u32, u32)>(gl_framebuffer_texture_multisample_multiview_ovr);

    println!("extension ptr1 {:?}", gl_renderbuffer_storage_multisample_ext);
    println!("extension ptr2 {:?}", gl_framebuffer_texture_2d_multisample_ext);
    println!("extension ptr3 {:?}", gl_framebuffer_texture_multiview_ovr);
    println!("extension ptr4 {:?}", gl_framebuffer_texture_multisample_multiview_ovr);

    framebuffer.width = width;
    framebuffer.height = height;
    framebuffer.multisamples = multisamples;
    framebuffer.multiview = (multiview && !gl_framebuffer_texture_multiview_ovr.is_null());
    println!("framebuffer.multiview={}", framebuffer.multiview);

    let texture_type = if multiview {
        ovr::ovrTextureType::VRAPI_TEXTURE_TYPE_2D_ARRAY
    } else {
        ovr::ovrTextureType::VRAPI_TEXTURE_TYPE_2D
    };
    println!("vrapi_createTextureSwapChain3");
    framebuffer.color_texture_swap_chain = unsafe {
        ovr::vrapi_CreateTextureSwapChain3(texture_type, color_format as i64, width, height, 1, 3)
    };
    framebuffer.texture_swap_chain_length = unsafe {
        ovr::vrapi_GetTextureSwapChainLength(framebuffer.color_texture_swap_chain)
    };
    println!("vrapi swap chain length {}", framebuffer.texture_swap_chain_length);

    framebuffer.depth_buffers = Vec::from_iter(0..framebuffer.texture_swap_chain_length).into_iter().map(|_| 0 as gl::types::GLuint).collect();
    framebuffer.frame_buffers = Vec::from_iter(0..framebuffer.texture_swap_chain_length).into_iter().map(|_| 0 as gl::types::GLuint).collect();

    for i in 0..framebuffer.texture_swap_chain_length as usize {
        // create color buffer texture
        let color_texture: GLuint = ovr::vrapi_GetTextureSwapChainHandle(framebuffer.color_texture_swap_chain, i as i32);
        let color_texture_target: GLenum = if framebuffer.multiview {
            GL_TEXTURE_2D_ARRAY
        } else {
            GL_TEXTURE_2D
        };

        glBindTexture(color_texture_target, color_texture);

        if egl.extensions.EXT_texture_border_clamp {
            glTexParameteri(color_texture_target, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_BORDER as i32);
            glTexParameteri(color_texture_target, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_BORDER as i32);
            let border_color = [0f32, 0f32, 0f32, 0f32];
            glTexParameterfv(color_texture_target, GL_TEXTURE_BORDER_COLOR, &(border_color[0]));
        } else {
            // Just clamp to edge. However, this requires manually clearing the border
            // around the layer to clear the edge texels.
            glTexParameteri(color_texture_target, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE as i32);
            glTexParameteri(color_texture_target, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE as i32);
        }

        glTexParameteri(color_texture_target, GL_TEXTURE_MIN_FILTER, GL_LINEAR as i32);
        glTexParameteri(color_texture_target, GL_TEXTURE_MAG_FILTER, GL_LINEAR as i32);
        glBindTexture(color_texture_target, 0);

        if framebuffer.multiview {
            // create depth buffer texture
            glGenTextures(1, &mut framebuffer.depth_buffers[i]);
            glBindTexture( GL_TEXTURE_2D_ARRAY, framebuffer.depth_buffers[i]);
            glTexStorage3D( GL_TEXTURE_2D_ARRAY, 1, GL_DEPTH_COMPONENT24, width, height, 2);
            glBindTexture( GL_TEXTURE_2D_ARRAY, 0);

            // Create the frame buffer.
            glGenFramebuffers( 1, &mut framebuffer.frame_buffers[i]);
            glBindFramebuffer( GL_DRAW_FRAMEBUFFER, framebuffer.frame_buffers[i]);
            if multisamples > 1 && (!gl_framebuffer_texture_multisample_multiview_ovr.is_null()) {
                gl_framebuffer_texture_multisample_multiview_ovr_fn( GL_DRAW_FRAMEBUFFER, GL_DEPTH_ATTACHMENT, framebuffer.depth_buffers[i], 0 /* level */, multisamples as u32 /* samples */, 0 /* baseViewIndex */, 2 /* numViews */ );
                gl_framebuffer_texture_multisample_multiview_ovr_fn( GL_DRAW_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, color_texture, 0 /* level */, multisamples as u32 /* samples */, 0 /* baseViewIndex */, 2 /* numViews */ );
            } else {
                gl_framebuffer_texture_multiview_ovr_fn( GL_DRAW_FRAMEBUFFER, GL_DEPTH_ATTACHMENT, framebuffer.depth_buffers[i], 0 /* level */, 0 /* baseViewIndex */, 2 /* numViews */ );
                gl_framebuffer_texture_multiview_ovr_fn( GL_DRAW_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, color_texture, 0 /* level */, 0 /* baseViewIndex */, 2 /* numViews */ );
            }

            let mut render_framebuffer_status = glCheckFramebufferStatus(GL_DRAW_FRAMEBUFFER);
            glBindFramebuffer( GL_DRAW_FRAMEBUFFER, 0 );
            if render_framebuffer_status != GL_FRAMEBUFFER_COMPLETE {
                panic!("incomplete frame buffer object: {:?}", render_framebuffer_status);
            }
        }
    }
}

pub fn ovr_framebuffer_destroy(framebuffer: &mut OvrFramebuffer) {
    unsafe {
        glDeleteFramebuffers(framebuffer.texture_swap_chain_length, &mut framebuffer.frame_buffers[0]);
        if framebuffer.multiview {
            glDeleteTextures(framebuffer.texture_swap_chain_length, &mut framebuffer.depth_buffers[0])
        } else {
            glDeleteRenderbuffers(framebuffer.texture_swap_chain_length, &mut framebuffer.depth_buffers[0]);
        }
        ovr::vrapi_DestroyTextureSwapChain(framebuffer.color_texture_swap_chain);
    }
}

pub fn egl_get_proc_address(proc: &str) -> *const u8 {
    // idiotically i was originally passing an &str.as_ptr() directly to eglGetProcAddress
    // which failed silently because it obviously expects a null terminated C string
    unsafe { eglGetProcAddress(CString::new(proc).unwrap().as_ptr() as *const i8) }
}

pub fn ovr_framebuffer_set_current(fb: &OvrFramebuffer) {
    unsafe { glBindFramebuffer(GL_DRAW_FRAMEBUFFER, fb.frame_buffers[fb.texture_swap_chain_index as usize]); }
}

pub fn ovr_framebuffer_set_none() {
    unsafe { glBindFramebuffer(GL_DRAW_FRAMEBUFFER, 0)};
}

pub fn ovr_framebuffer_resolve(fb: &OvrFramebuffer) {
    unsafe {
        // Discard the depth buffer, so the tiler won't need to write it back out to memory.
        let depth_attachment = [GL_DEPTH_ATTACHMENT];
        glInvalidateFramebuffer(GL_DRAW_FRAMEBUFFER, 1, &depth_attachment[0]);
        // Flush this frame worth of commands.
        glFlush();
    }
}

pub fn ovr_framebuffer_advance(fb: &mut OvrFramebuffer) {
    // Advance to the next texture from the set.
    fb.texture_swap_chain_index = (fb.texture_swap_chain_index + 1) % fb.texture_swap_chain_length;
}