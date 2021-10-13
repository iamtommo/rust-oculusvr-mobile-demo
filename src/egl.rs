extern crate egl as khr;
use khr::egl;
use khr::egl::{EGLDisplay, EGLConfig, EGLSurface, EGLContext};

use gles3::gles;
use gles3::gles::*;

use crate::graphics::GlExtensions;
use std::ffi::CStr;
use std::os::raw::c_char;

pub struct OvrEgl {
    pub major_version: egl::EGLint,
    pub minor_version: egl::EGLint,
    pub display: EGLDisplay,
    pub config: EGLConfig,
    pub tiny_surface: EGLSurface,
    pub main_surface: EGLSurface,
    pub context: EGLContext,
    pub extensions: GlExtensions
}

pub(crate) fn create_egl(egl: &mut OvrEgl) -> &mut OvrEgl {
    egl.display = egl::GetDisplay(std::ptr::null_mut());

    println!("egl::initialize for display {:?}", egl.display);
    egl::Initialize(egl.display, &mut egl.major_version, &mut egl.minor_version);
    println!("egl::initialized with major version {} minor version {}", egl.major_version, egl.minor_version);

    // from oculus:
    // Do NOT use eglChooseConfig, because the Android EGL code pushes in multisample
    // flags in eglChooseConfig if the user has selected the "force 4x MSAA" option in
    // settings, and that is completely wasted for our warp target.
    let mut configs: [EGLConfig; 1024] = [std::ptr::null_mut(); 1024];
    let mut num_configs = 0;
    egl::GetConfigs(egl.display, &mut configs[0], 1024, &mut num_configs);

    println!("egl::get_configs num_configs {}", num_configs);
    if num_configs == 0 {
        panic!("egl::get_configs failed: {}", egl::GetError())
    }

    let config_attribs: [i32; 15] = [
        egl::EGL_RED_SIZE as i32, 8,
        egl::EGL_GREEN_SIZE as i32, 8,
        egl::EGL_BLUE_SIZE as i32, 8,
        egl::EGL_ALPHA_SIZE as i32, 8, // oculus: need alpha for the multi-pass timewarp compositor
        egl::EGL_DEPTH_SIZE as i32, 0,
        0x3026 as i32, 0,//egl::EGL_STENCIL_SIZE, 0,
        0x3031 as i32, 0,//egl::EGL_SAMPLES, 0,
        egl::EGL_NONE as i32
    ];


    for i in 0..num_configs {
        let mut value: egl::EGLint = 0;
        let cfg = configs[i as usize];//unsafe { configs.configs.offset(i as isize) };
        egl::GetConfigAttrib(egl.display, cfg, egl::EGL_RENDERABLE_TYPE as i32, &mut value);

        // TODO EGL_OPENGL_ES3_BIT_KHR            0x00000040
        if (value & 0x00000040) != 0x00000040 {
            continue;
        }

        // oculus: The pbuffer config also needs to be compatible with normal window rendering
        // so it can share textures with the window context.
        egl::GetConfigAttrib(egl.display, cfg, egl::EGL_SURFACE_TYPE as i32, &mut value);
        if (value & (egl::EGL_WINDOW_BIT | egl::EGL_PBUFFER_BIT) as i32) != (egl::EGL_WINDOW_BIT | egl::EGL_PBUFFER_BIT) as i32 {
            continue;
        }

        let mut j = 0;
        loop {
            if config_attribs[j] == egl::EGL_NONE as i32 {
                break;
            }
            egl::GetConfigAttrib(egl.display, cfg, config_attribs[j] as i32, &mut value);
            if value != config_attribs[j + 1] as i32 {
                break;
            }
            j += 2;
        }

        if config_attribs[j] == egl::EGL_NONE as i32 {
            egl.config = cfg;
            break;
        }
    }

    if egl.config.is_null() {
        panic!("egl choose config failed: {}", egl::GetError());
    }

    let context_attribs: [i32; 3] = [egl::EGL_CONTEXT_CLIENT_VERSION as i32, 3, egl::EGL_NONE as i32];
    println!("egl::create_context");
    egl.context = egl::CreateContext(egl.display, egl.config, std::ptr::null_mut(), &context_attribs[0]);
    println!("egl_context ptr {:?}", egl.context);
    if egl.context.is_null() {
        panic!("egl::create_context failed: {}", egl::GetError());
    }

    let mut surface_attribs: [i32; 5] = [egl::EGL_WIDTH as i32, 16, egl::EGL_HEIGHT as i32, 16, egl::EGL_NONE as i32];
    println!("egl::create_pbuffer_surface");
    egl.tiny_surface = egl::CreatePbufferSurface(egl.display, egl.config, &mut surface_attribs[0]);
    println!("egl::create_pbuffer_surface ptr {:?}", egl.tiny_surface);
    if egl.tiny_surface.is_null() {
        egl::DestroyContext(egl.display, egl.context);
        panic!("egl::create_pbuffer_surface failed: {}", egl::GetError());
    }

    println!("egl::make_current");
    let make_current_success = egl::MakeCurrent(egl.display, egl.tiny_surface, egl.tiny_surface, egl.context);
    if make_current_success == egl::EGL_FALSE {
        egl::DestroySurface(egl.display, egl.tiny_surface);
        egl::DestroyContext(egl.display, egl.tiny_surface);
        panic!("egl::make_current failed: {}", egl::GetError());
    }
    return egl;
}

pub fn init_egl_extensions(ovr_egl: &mut OvrEgl) {
    unsafe {
        let extns_ptr = gles::glGetString(gles::GL_EXTENSIONS);
        if extns_ptr.is_null() {
            eprintln!("GL_EXTENSIONS nullptr");
        } else {
            let c_str: &CStr = CStr::from_ptr(extns_ptr as *const c_char);
            let extensions_str = c_str.to_str().unwrap();
            ovr_egl.extensions.multi_view = extensions_str.contains("GL_OVR_multiview2")
                && extensions_str.contains("GL_OVR_multiview_multisampled_render_to_texture");

            ovr_egl.extensions.EXT_texture_border_clamp = extensions_str.contains("GL_EXT_texture_border_clamp")
                && extensions_str.contains("GL_OES_texture_border_clamp");
            println!("glEXT: multiview={}", ovr_egl.extensions.multi_view);
            println!("glEXT: tex_border_clamp={}", ovr_egl.extensions.EXT_texture_border_clamp);
        }
    }
}

pub(crate) fn destroy_egl(egl: &mut OvrEgl) {
    if !egl.display.is_null() {
        egl::MakeCurrent(egl.display, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
    }
    if !egl.context.is_null() {
        egl::DestroyContext(egl.display, egl.context);
    }
    if !egl.tiny_surface.is_null() {
        egl::DestroySurface(egl.display, egl.tiny_surface);
    }
    if !egl.display.is_null() {
        egl::Terminate(egl.display);
    }
}

pub fn get_current_surface() -> EGLSurface {
    egl::GetCurrentSurface(egl::EGL_DRAW)
}