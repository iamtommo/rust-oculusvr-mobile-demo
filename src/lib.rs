extern crate libc;
#[macro_use] extern crate log;

use std::{fmt, mem};
use std::convert::TryInto;
use std::ffi::{c_void, CStr, CString};
use std::fmt::{Display, Formatter};
use std::os::raw::{c_char, c_ulonglong};
use std::process::exit;

use gl::types::*;
use gles3::gles;
use gles3::gles::*;
use log::{Level, LevelFilter, Metadata, Record};
use math::matrix::{float4x4, matrix4x4_identity, matrix4x4_inverse, matrix4x4_rot, matrix4x4_scale, matrix4x4_translation, matrix4x4_transpose, matrix4x4_trs, matrix4x4_mul};
use math::quaternion::quaternion;
use math::vector::float3;
use ovr_mobile_sys as ovr;
use ovr_mobile_sys::{ovrDeviceID, ovrLayerCube2, ovrLayerCylinder2, ovrMatrix4f, ovrQuatf, ovrTextureSwapChain, ovrTracking, ovrTracking2, ovrVector3f, vrapi_CreateTextureSwapChain3, vrapi_GetInputTrackingState, vrapi_GetSystemPropertyInt, vrapi_GetTextureSwapChainHandle, VRAPI_PI, VRAPI_ZNEAR};
use ovr_mobile_sys::helpers::*;
use ovr_mobile_sys::ovrFrameLayerBlend_::{VRAPI_FRAME_LAYER_BLEND_ONE, VRAPI_FRAME_LAYER_BLEND_ONE_MINUS_SRC_ALPHA, VRAPI_FRAME_LAYER_BLEND_SRC_ALPHA, VRAPI_FRAME_LAYER_BLEND_ZERO};
use ovr_mobile_sys::ovrFrameLayerEye_::VRAPI_FRAME_LAYER_EYE_MAX;
use ovr_mobile_sys::ovrFrameLayerFlags_::VRAPI_FRAME_LAYER_FLAG_CHROMATIC_ABERRATION_CORRECTION;
use ovr_mobile_sys::ovrInitializeStatus::VRAPI_INITIALIZE_SUCCESS;
use ovr_mobile_sys::ovrSuccessResult_::ovrSuccess;
use ovr_mobile_sys::ovrSystemProperty::VRAPI_SYS_PROP_MULTIVIEW_AVAILABLE;
use ovr_mobile_sys::ovrTextureType_::VRAPI_TEXTURE_TYPE_2D;

use crate::graphics::*;
use crate::input::DeviceInput;
use crate::model::SkeletalMesh;
use crate::render::gl_buffer::GlBuffer;
use crate::render::gl_geometry::{GlGeometry, make_geometry};
use crate::render::gl_geometry;
use crate::shader::ShaderProgram;

use ndk_glue::{Event, native_activity, native_window, poll_events};

use ndk_sys::JNINativeInterface;
use crate::anim::composer::SkeletalComposer;
use std::cell::Cell;
use crate::anim::layer::{SkeletalLayer, SkeletalLayerSpec};

mod graphics;
mod vrapi;
mod shader;
mod transform;
mod render;
mod model;
mod gltf;
mod egl;
mod framebuffer;
mod assets;
mod input;
mod anim;

static LOGGER: SimpleLogger = SimpleLogger;
static LOGGER_LEVEL_FILTER: LevelFilter = LevelFilter::Debug;

pub struct OvrApp {
    java: ovr::ovrJava,
    egl: egl::OvrEgl,
    ovr: Option<*mut ovr::ovrMobile>,
    resumed: bool,
    destroyed: bool,
    window_active: bool,
    frame_index: i64,
    display_time: f64,
    swap_interval: i32,
    cpu_level: i32,
    gpu_level: i32,
    main_thread_tid: i32,
    back_button_down_last_frame: bool,
    multiview: bool,
    scene: OvrScene,
    device_input: DeviceInput,
    time: UniversalTime
}

//#[cfg(target_os = "android")]
//ndk_glue::ndk_glue!(main);

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LOGGER_LEVEL_FILTER));
    info!("hi rust v8");

    unsafe {
        ndk_sys::ANativeActivity_setWindowFlags(native_activity().ptr().as_mut(), ndk_sys::AWINDOW_FLAG_KEEP_SCREEN_ON, 0);
    }

    println!("deref jvm");
    let jvm_ptr = native_activity().vm();
    let mut jvm_env = std::ptr::null_mut();
    unsafe {
        let fn_get_env = (**jvm_ptr).GetEnv.unwrap();
        fn_get_env(jvm_ptr, &mut jvm_env, ffi::JNI_VERSION_1_6.try_into().unwrap());
    }

    println!("attach jvm");
    unsafe {
        let fn_attach_thread = (**jvm_ptr).AttachCurrentThread.unwrap();
        fn_attach_thread(jvm_ptr, &mut jvm_env, std::ptr::null_mut());
    }

    println!("assign ovr jvm struct");
    let ovrj: ovr_mobile_sys::ovrJava = ovr::ovrJava {
        Vm: jvm_ptr as *mut ovr::JavaVM,
        Env: jvm_env as *mut *const ovr::JNINativeInterface_,
        ActivityObject: native_activity().activity() as ovr::jobject
    };

    println!("ovr init");
    unsafe {
        let init_params = vrapi_DefaultInitParms(&ovrj);
        let init_result: ovr::ovrInitializeStatus = ovr::vrapi_Initialize(&init_params);
        if init_result != VRAPI_INITIALIZE_SUCCESS {
            panic!("ovr init failed with code {:?}", init_result);
        }
    }

    let mut app_state: OvrApp = unsafe { mem::zeroed() };
    app_state.java = ovrj;
    app_state.resumed = false;
    app_state.destroyed = false;
    app_state.window_active = false;
    app_state.ovr = Option::None;
    app_state.frame_index = 1;
    app_state.display_time = 0f64;
    app_state.swap_interval = 1;
    app_state.cpu_level = 2;
    app_state.gpu_level = 3;
    app_state.main_thread_tid = i32::from(nix::unistd::gettid());
    app_state.device_input = input::create_device_input();
    app_state.time = UniversalTime {
        frame_time: 0.0,
        delta_time: 0.0,
        program_start_time: clock_seconds(),
        system_time: clock_seconds()
    };
    println!("thread id {}", app_state.main_thread_tid);

    // setup universe
   /* app_state.universe.set_singleton(UniversalTime {
        frame_time: 0.0,
        delta_time: 0.0,
        program_start_time: clock_seconds(),
        system_time: clock_seconds()
    });*/

    println!("create egl");
    egl::create_egl(&mut app_state.egl);
    println!("egl display {:?}", app_state.egl.display);

    println!("egl init extensions");
    egl::init_egl_extensions(&mut app_state.egl);

    let multiview_available = vrapi::get_system_property_int(&app_state.java, VRAPI_SYS_PROP_MULTIVIEW_AVAILABLE) == 1;
    app_state.multiview = app_state.egl.extensions.multi_view && multiview_available;
    println!("multiview? {}", app_state.multiview);

    println!("create ovr renderer");
    let mut renderer: OvrRenderer = unsafe { mem::zeroed() };
    ovr_renderer_create(&app_state, &mut renderer);

    // main loop
    loop {
        // update time
        let prev_time = app_state.time;
        let system_time_now = clock_seconds();
        let frame_time = system_time_now - prev_time.program_start_time;
        let delta_time = frame_time - prev_time.frame_time;
        let time = UniversalTime {
            frame_time,
            delta_time,
            program_start_time: prev_time.program_start_time,
            system_time: system_time_now
        };
        app_state.time = time;

        // poll events
        while let Some(ev) = poll_events() {
            handle_app_event(&mut app_state, ev);
            handle_vrmode_changes(&mut app_state);
        }

        // break main loop if destroyed (must check after poll events)
        if app_state.destroyed {
            break;
        }

        if app_state.ovr.is_none() {
            continue;
        }

        // create scene if not yet created
        if !app_state.scene.created_scene {
            // show loading icon
            let mut frame_flags = 0;
            frame_flags |= ovr::ovrFrameFlags::VRAPI_FRAME_FLAG_FLUSH as i32;

            let mut black_layer = vrapi_DefaultLayerBlackProjection2();
            black_layer.Header.Flags |= ovr::ovrFrameLayerFlags::VRAPI_FRAME_LAYER_FLAG_INHIBIT_SRGB_FRAMEBUFFER as u32;

            let mut icon_layer = vrapi_DefaultLayerLoadingIcon2();
            icon_layer.Header.Flags |= ovr::ovrFrameLayerFlags::VRAPI_FRAME_LAYER_FLAG_INHIBIT_SRGB_FRAMEBUFFER as u32;

            let layers: [*const ovr::ovrLayerHeader2; 2] = [&black_layer.Header, &icon_layer.Header];

            let mut frame_desc: ovr::ovrSubmitFrameDescription2 = unsafe { mem::zeroed() };
            frame_desc.Flags = frame_flags as u32;
            frame_desc.SwapInterval = 1;
            frame_desc.FrameIndex = app_state.frame_index as u64;
            frame_desc.DisplayTime = app_state.display_time;
            frame_desc.LayerCount = 2;
            frame_desc.Layers = &layers[0];

            unsafe {
                ovr::vrapi_SubmitFrame2(app_state.ovr.unwrap(), &frame_desc);
            }

            // create scene
            ovr_scene_create(&mut app_state.scene, &app_state.egl.extensions, app_state.multiview);
        }

        // This is the only place the frame index is incremented, right before
        // calling vrapi_GetPredictedDisplayTime().
        app_state.frame_index += 1;

        // Get the HMD pose, predicted for the middle of the time period during which
        // the new eye images will be displayed. The number of frames predicted ahead
        // depends on the pipeline depth of the engine and the synthesis rate.
        // The better the prediction, the less black will be pulled in at the edges.
        let predicted_display_time: f64 = unsafe { ovr::vrapi_GetPredictedDisplayTime(app_state.ovr.unwrap(), app_state.frame_index ) };
        let tracking: ovr::ovrTracking2 = unsafe { ovr::vrapi_GetPredictedTracking2(app_state.ovr.unwrap(), predicted_display_time) };

        app_state.display_time = predicted_display_time;

        input::scan_input_devices(app_state.ovr.unwrap(), &mut app_state.device_input);
        if app_state.device_input.controller_single.is_some() {
            let device = app_state.device_input.controller_single.unwrap();
            let mut device_tracking: ovrTracking = unsafe { mem::zeroed() };
            unsafe {
                if (ovrSuccess as i32) == vrapi_GetInputTrackingState(app_state.ovr.unwrap(), device,
                                                             predicted_display_time, &mut device_tracking) {
                    println!("device {:?}", device_tracking.HeadPose.Pose.Orientation);
                    app_state.scene.controller_orientation = device_tracking.HeadPose.Pose.Orientation;
                }
            }
        }

        app_state.scene.mob_composer.update(time.delta_time);
        let anim_matrices = app_state.scene.mob_composer.sample(&app_state.scene.mob_skinned_mesh);
        app_state.scene.mob_jointbuf.update((anim_matrices.len() * mem::size_of::<float4x4>()) as isize, anim_matrices.as_ptr() as *const _ as *const u8);

        // Advance the simulation based on the elapsed time since start of loop till predicted display time.
        //unsafe { ovr::ovrSimulation_Advance( &appState.Simulation, predictedDisplayTime - startTime ) };
        /*let mut universe = &mut app_state.universe;
        let mut skeletal_animation_system = universe.get_system::<SkeletalAnimationSystem>();
        universe.update_system::<SkeletalAnimationSystem>();*/
        //skeletal_animation_system.update(universe);

        unsafe {
            let mut world_layer: ovr::ovrLayerProjection2 = vrapi_DefaultLayerProjection2();
            ovr_render_frame(&mut world_layer, &mut renderer, &app_state.java,
                             &app_state.scene, &tracking,
                             &app_state.egl.extensions, app_state.ovr.unwrap());

            let mut interface_layer = mk_cylinder_layer(app_state.scene.interface_layer_cylinder_swap_chain,
            app_state.scene.interface_layer_cylinder_width, app_state.scene.interface_layer_cylinder_height, &tracking);

            let layer_headers: [*const ovr::ovrLayerHeader2; 2] = [&world_layer.Header, &interface_layer.Header];
            let mut frame_desc: ovr::ovrSubmitFrameDescription2 = mem::zeroed();
            frame_desc.Flags = 0;
            frame_desc.SwapInterval = app_state.swap_interval as u32;
            frame_desc.FrameIndex = app_state.frame_index as u64;
            frame_desc.DisplayTime = app_state.display_time;
            frame_desc.LayerCount = 2;
            frame_desc.Layers = &layer_headers[0];

            // Hand over the eye images to the time warp.
            ovr::vrapi_SubmitFrame2(app_state.ovr.unwrap(), &frame_desc);
        }
    }

    println!("destroy ovr renderer");
    ovr_renderer_destroy(&mut renderer);

    println!("destroy egl");
    egl::destroy_egl(&mut app_state.egl);

    println!("shutdown ovr");
    unsafe {
        ovr::vrapi_Shutdown();
    }

    println!("detach jvm");
    unsafe {
        let fn_detach_thread = (*(*jvm_ptr)).DetachCurrentThread.unwrap();
        fn_detach_thread(jvm_ptr);
    }
}

pub struct OvrScene {
    pub created_scene: bool,
    pub random: i64,
    pub shader_programs: [ShaderProgram; shader::NUM_PROGRAMS],
    pub scene_matrices: GlBuffer,
    pub tabletop: GlGeometry,
    pub mob_skinned_mesh: SkeletalMesh,
    pub mob: GlGeometry,
    pub mob_texture: GLuint,
    pub mob_jointbuf: GlBuffer,
    pub mob_composer: SkeletalComposer,
    pub controller: GlGeometry,
    pub controller_orientation: ovrQuatf,
    pub interface_layer_cylinder_width: i32,
    pub interface_layer_cylinder_height: i32,
    pub interface_layer_cylinder_swap_chain: *mut ovrTextureSwapChain
}

pub fn ovr_scene_clear(scene: &mut OvrScene) {
    scene.created_scene = false;
    scene.random = 2;
    // TODO
    // clear program
    // clear geo
}

fn ovr_scene_create(scene: &mut OvrScene, extns: &GlExtensions, multiview: bool) {
    println!("ovr_scene_create");
    scene.created_scene = true;

    let shader = shader::build_shader_program(shader::ProgramId::VertexColor, multiview);
    scene.shader_programs[0] = shader;

    println!("build texture shader");
    let tex_shader = shader::build_shader_program(shader::ProgramId::SingleTexture, multiview);
    scene.shader_programs[1] = tex_shader;

    let skinned_shader = shader::build_shader_program(shader::ProgramId::SingleTextureSkinned4, multiview);
    scene.shader_programs[2] = skinned_shader;

    // setup scene matrices
    // 2 view matrices + 2 projection matrices
    scene.scene_matrices = GlBuffer::create((mem::size_of::<ovr::ovrMatrix4f>() * 4) as isize, std::ptr::null());

    let tabletop_gltf_file = gltf::load_gltf(&mut assets::load_asset("tabletop.gltf").unwrap());
    let tabletop_mesh = gltf::mesh::load_mesh("tabletop", &tabletop_gltf_file);
    let controller_gltf_file = gltf::load_gltf(&mut assets::load_asset("resources/controller_gearvr.gltf").unwrap());
    let controller_mesh = gltf::mesh::load_mesh("controller_gearvr", &controller_gltf_file);

    scene.tabletop = make_geometry(&tabletop_mesh.attribs, &tabletop_mesh.indices);
    scene.controller = make_geometry(&controller_mesh.attribs, &controller_mesh.indices);

    println!("read mob mesh");
    let mut mob_asset = assets::load_asset("resources/mesh_brownbear_v2.gltf").unwrap();
    let mob_gltf_file = gltf::load_gltf(&mut mob_asset);
    scene.mob_skinned_mesh = gltf::skeletal::load_skeletal_entity("bear", &mob_gltf_file);
    scene.mob = make_geometry(&scene.mob_skinned_mesh.mesh.attribs, &scene.mob_skinned_mesh.mesh.indices);

    info!("read mob animations");
    let mut mob_anim_asset = assets::load_asset("resources/anim_bear_attack.gltf").unwrap();
    let mob_anim_gltf_file = gltf::load_gltf(&mut mob_anim_asset);
    let bear_anim = gltf::skeletal::load_animations(&scene.mob_skinned_mesh, &mob_anim_gltf_file);

    scene.mob_jointbuf = GlBuffer::create((gl_geometry::MAX_JOINTS
                        * mem::size_of::<float4x4>() as i32) as isize, std::ptr::null());
    let joint_ptr = scene.mob_jointbuf.map_buffer();
    let mut joints: Vec<float4x4> = vec![matrix4x4_identity(); gl_geometry::MAX_JOINTS as usize];
    unsafe {
        std::ptr::copy(joints.as_ptr() as *const _ as *const u8, joint_ptr, (joints.len() * mem::size_of::<float4x4>()) as usize);
    }
    scene.mob_jointbuf.unmap_buffer();
    let composer = SkeletalComposer::new(1.0,
                                         vec![SkeletalLayer { spec: SkeletalLayerSpec {
                                                                            loopanim: true,
                                                                             playback_speed: Cell::new(1.0)},
                                             anim: bear_anim }]);
    scene.mob_composer = composer;

    println!("read mob texture");
    let mut mob_texture_asset = assets::load_asset("resources/tex_brownbear_color.png").unwrap();
    let mob_texture_decoder = png::Decoder::new((&mut mob_texture_asset).get_buffer().unwrap());
    let (info, mut reader) = mob_texture_decoder.read_info().unwrap();
    let mut texbuf = vec![0; info.buffer_size()];
    reader.next_frame(&mut texbuf).unwrap();
    println!("mob texture buflen {}, colortype {:?}, bitdepth {:?}", texbuf.len(), info.color_type, info.bit_depth);

    let mut mobtexture: GLuint = 0;
    unsafe {
        glGenTextures(1, &mut mobtexture);
        glBindTexture(GL_TEXTURE_2D, mobtexture);

        glTexImage2D(GL_TEXTURE_2D, 0, GL_RGB as i32, info.width as i32, info.height as i32,
                     0, GL_RGB, GL_UNSIGNED_BYTE, texbuf.as_mut_ptr() as *const _ as *const c_void);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR as i32);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR_MIPMAP_LINEAR as i32);
        glGenerateMipmap(GL_TEXTURE_2D);
        glBindTexture(GL_TEXTURE_2D, 0);
    }
    scene.mob_texture = mobtexture;

    scene.interface_layer_cylinder_width = 512;
    scene.interface_layer_cylinder_height = 128;
    scene.interface_layer_cylinder_swap_chain = unsafe {vrapi_CreateTextureSwapChain3(VRAPI_TEXTURE_TYPE_2D,
    GL_RGBA8 as i64, scene.interface_layer_cylinder_width, scene.interface_layer_cylinder_height, 1, 1)};

    let tex_data_len = (scene.interface_layer_cylinder_width *
        scene.interface_layer_cylinder_height * mem::size_of::<u32>() as i32) as usize;
    let mut tex_data: Vec<u32> = vec![0; tex_data_len];

    for y in 0..scene.interface_layer_cylinder_height as usize {
        for x in 0..scene.interface_layer_cylinder_width as usize {
            tex_data[y * scene.interface_layer_cylinder_width as usize + x] = if (x ^ y) & 64 != 0 {
                0xFF6464F0
            } else {
                0xFFF06464
            }
        }
    }

    // If border clamp is not supported, manually clear the border.
    if extns.EXT_texture_border_clamp == false {
        for i in 0..scene.interface_layer_cylinder_width as usize {
            tex_data[i] = 0x00000000;
            tex_data[((scene.interface_layer_cylinder_height as usize - 1) * scene.interface_layer_cylinder_width as usize + i)] = 0x00000000;
        }
        for i in 0..scene.interface_layer_cylinder_height as usize {
            tex_data[i * scene.interface_layer_cylinder_width as usize] = 0x00000000;
            tex_data[(i * scene.interface_layer_cylinder_width as usize + scene.interface_layer_cylinder_width as usize - 1)] = 0x00000000;
        }
    }

    unsafe {
        let tex_id = vrapi_GetTextureSwapChainHandle(scene.interface_layer_cylinder_swap_chain, 0);
        glBindTexture(GL_TEXTURE_2D, tex_id);
        glTexSubImage2D(GL_TEXTURE_2D, 0, 0, 0, scene.interface_layer_cylinder_width, scene.interface_layer_cylinder_height, GL_RGBA,
                        GL_UNSIGNED_BYTE, tex_data.as_ptr() as *const _ as *const c_void);

        if extns.EXT_texture_border_clamp {
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_BORDER as i32);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_BORDER as i32);
            let border_color = [0f32, 0f32, 0f32, 0f32];
            glTexParameterfv(GL_TEXTURE_2D, GL_TEXTURE_BORDER_COLOR, &border_color[0]);
        } else {
            // Just clamp to edge. However, this requires manually clearing the border
            // around the layer to clear the edge texels.
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE as i32);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE as i32);
        }
        glBindTexture(GL_TEXTURE_2D, 0);
    }

}

fn mk_center_eye(tracking: &ovrTracking2) -> ovrMatrix4f {
    let neutral_head_center = ovrVector3f { x: 0f32, y: 0f32, z: 0f32 };// foot pos
    let input = ovrMatrix4f_CreateTranslation(neutral_head_center.x, neutral_head_center.y, neutral_head_center.z);

    let transform = vrapi_GetTransformFromPose(&tracking.HeadPose.Pose);
    let center_eye_transform = ovrMatrix4f_Multiply(&input, &transform);
    let center_eye_view_matrix = ovrMatrix4f_Inverse(&center_eye_transform);

    return center_eye_view_matrix;
}

fn mk_eye_view_matrix(eye: usize, tracking: &ovrTracking2) -> ovrMatrix4f {
    let head_rotation = ovrMatrix4f_CreateFromQuaternion(&tracking.HeadPose.Pose.Orientation);

    // convert the eye view to world-space and remove translation
    let mut eye_view_rot = tracking.Eye[eye].ViewMatrix;
    eye_view_rot.M[0][3] = 0f32;
    eye_view_rot.M[1][3] = 0f32;
    eye_view_rot.M[2][3] = 0f32;
    let eye_rotation = ovrMatrix4f_Inverse(&eye_view_rot);

    // compute the rotation transform from head to eye (in case of rotation screens)
    let head_rot_inv = ovrMatrix4f_Inverse(&head_rotation);
    let head_eye_rotation = ovrMatrix4f_Multiply(&head_rot_inv, &eye_rotation);

    // add ipd translation from head to eye
    let eye_shift: f32 = (if eye == 0 {
        -0.5f32
    } else {
        0.5f32
    }) * 0.0640f32;// ipd
    let head_eye_translation = ovrMatrix4f_CreateTranslation(eye_shift, 0f32, 0f32);

    // the full transform from head to eye in world
    let head_eye_transform = ovrMatrix4f_Multiply(&head_eye_translation, &head_eye_rotation);

    // compute the new eye-pose using the input center eye view
    let center_eye_pose_m = ovrMatrix4f_Inverse(&mk_center_eye(&tracking));
    let eye_pose_m = ovrMatrix4f_Multiply(&center_eye_pose_m, &head_eye_transform);

    let eye_view = ovrMatrix4f_Inverse(&eye_pose_m);
    return eye_view;
}

fn mk_eye_view_matrix2(view_matrix: ovrMatrix4f) -> ovrMatrix4f {
    /*let mut eye_view_matrix = tracking.Eye[eye].ViewMatrix;
    eye_view_matrix.M[0][3] = 0f32;
    eye_view_matrix.M[1][3] = 1.6750f32;
    eye_view_matrix.M[2][3] = 0f32;
    return eye_view_matrix;*/
    let translation = ovrMatrix4f_CreateTranslation(0f32, -1.6750f32, 0f32);
    return ovrMatrix4f_Multiply(&view_matrix, &translation);
}

fn mk_cylinder_model_matrix(texture_width: i32, texture_height: i32, translation: ovrVector3f,
                        rotateYaw: f32, rotatePitch: f32, radius: f32, density: f32) -> ovrMatrix4f {
    let scale_matrix = ovrMatrix4f_CreateScale(radius, radius * (texture_height as f32) * VRAPI_PI as f32 / density, radius);
    let trans_matrix = ovrMatrix4f_CreateTranslation(translation.x, translation.y, translation.z);
    let rot_x_matrix = ovrMatrix4f_CreateRotation(rotatePitch, 0f32, 0f32);
    let rot_y_matrix = ovrMatrix4f_CreateRotation(0f32, rotateYaw, 0f32);

    let m0 = ovrMatrix4f_Multiply(&trans_matrix, &scale_matrix);
    let m1 = ovrMatrix4f_Multiply(&rot_x_matrix, &m0);
    let m2 = ovrMatrix4f_Multiply(&rot_y_matrix, &m1);

    return m2;
}

fn mk_cylinder_layer(cylinder_swap_chain: *mut ovrTextureSwapChain, texture_width: i32, texture_height: i32,
                        tracking: &ovrTracking2) -> ovrLayerCylinder2 {
    let mut layer = vrapi_DefaultLayerCylinder2();
    let fade_level = 1f32;
    layer.Header.ColorScale.x = fade_level;
    layer.Header.ColorScale.y = fade_level;
    layer.Header.ColorScale.z = fade_level;
    layer.Header.ColorScale.w = fade_level;
    layer.Header.SrcBlend = VRAPI_FRAME_LAYER_BLEND_SRC_ALPHA;
    layer.Header.DstBlend = VRAPI_FRAME_LAYER_BLEND_ONE_MINUS_SRC_ALPHA;

    //layer.Header.Flags = VRAPI_FRAME_LAYER_FLAG_CLIP_TO_TEXTURE_RECT

    layer.HeadPose = tracking.HeadPose;

    let density = 4500f32;
    let rotate_yaw = 0f32;
    let rotate_pitch = -0.35f32;//0f32;
    let radius = 3f32;
    let translation = ovrVector3f { x: 0f32, y: 0f32, z: 0f32 };

    let cylinder_transform = mk_cylinder_model_matrix(texture_width, texture_height,
                translation, rotate_yaw, rotate_pitch, radius, density);

    let circ_scale = density * 0.5f32 / texture_width as f32;
    let circ_bias = -circ_scale * (0.5f32 * (1f32 - 1f32 / circ_scale));

    for eye in 0..VRAPI_FRAME_LAYER_EYE_MAX as usize {
        let model_view_matrix = ovrMatrix4f_Multiply(&tracking.Eye[eye].ViewMatrix, &cylinder_transform);
        layer.Textures[eye].TexCoordsFromTanAngles = ovrMatrix4f_Inverse(&model_view_matrix);
        layer.Textures[eye].ColorSwapChain = cylinder_swap_chain;
        layer.Textures[eye].SwapChainIndex = 0;

        // Texcoord scale and bias is just a representation of the aspect ratio. The positioning
        // of the cylinder is handled entirely by the TexCoordsFromTanAngles matrix.

        let tex_scale_x = circ_scale;
        let tex_bias_x = circ_bias;
        let tex_scale_y = 0.5f32;
        let tex_bias_y = -tex_scale_y * (0.5f32 * (1f32 - (1f32 / tex_scale_y)));

        layer.Textures[eye].TextureMatrix.M[0][0] = tex_scale_x;
        layer.Textures[eye].TextureMatrix.M[0][2] = tex_bias_x;
        layer.Textures[eye].TextureMatrix.M[1][1] = tex_scale_y;
        layer.Textures[eye].TextureMatrix.M[1][2] = tex_bias_y;

        layer.Textures[eye].TextureRect.width = 1f32;
        layer.Textures[eye].TextureRect.height = 1f32;
    }

    return layer;
}

fn ovr_render_frame(layer: &mut ovr::ovrLayerProjection2, renderer: &mut OvrRenderer, java: &ovr::ovrJava,
                    scene: &OvrScene, tracking: &ovr::ovrTracking2, extns: &GlExtensions, ovr: *mut ovr::ovrMobile) {

    /*let eye_view_matrices: [ovrMatrix4f; 2] = [
        mk_eye_view_matrix(0, &tracking),
        mk_eye_view_matrix(1, &tracking)
    ];
    let eye_projection_matrices: [ovrMatrix4f; 2] = [
        ovrMatrix4f_CreateProjectionFov(renderer.eyefov_x, renderer.eyefov_y, 0f32, 0f32, VRAPI_ZNEAR as f32, 0f32),
        ovrMatrix4f_CreateProjectionFov(renderer.eyefov_x, renderer.eyefov_y, 0f32, 0f32, VRAPI_ZNEAR as f32, 0f32),
    ];*/

    let tracking_eye_view_matrices = [
        mk_eye_view_matrix2(tracking.Eye[0].ViewMatrix),
        mk_eye_view_matrix2(tracking.Eye[1].ViewMatrix)
    ];

    let transposed_eye_view_matrices = [
        ovrMatrix4f_Transpose(&tracking_eye_view_matrices[0]),
        ovrMatrix4f_Transpose(&tracking_eye_view_matrices[1]),
    ];

    let transposed_eye_projection_matrices = [
        ovrMatrix4f_Transpose(&tracking.Eye[0].ProjectionMatrix),
        ovrMatrix4f_Transpose(&tracking.Eye[1].ProjectionMatrix)
    ];

    // update scene matrices
    // 2 view matrices + 2 projection matrices
    let scene_matrices_ptr = scene.scene_matrices.map_buffer();
    unsafe {
        std::ptr::copy(&transposed_eye_view_matrices as *const _ as *const u8,
                       scene_matrices_ptr, mem::size_of::<ovrMatrix4f>() * 2);
        std::ptr::copy(&transposed_eye_projection_matrices as *const _ as *const u8,
                       scene_matrices_ptr.offset((2 * mem::size_of::<ovrMatrix4f>()) as isize), mem::size_of::<ovrMatrix4f>() * 2);
    }
    scene.scene_matrices.unmap_buffer();

    layer.HeadPose = tracking.HeadPose;
    for eye in 0..ovr::ovrFrameLayerEye::VRAPI_FRAME_LAYER_EYE_MAX as usize {
        let framebuffer = if renderer.num_buffers == 1 {
            &renderer.frame_buffers[0]
        } else {
            &renderer.frame_buffers[eye]
        };
        layer.Textures[eye].ColorSwapChain = framebuffer.color_texture_swap_chain;
        layer.Textures[eye].SwapChainIndex = framebuffer.texture_swap_chain_index;
        layer.Textures[eye].TexCoordsFromTanAngles = ovrMatrix4f_TanAngleMatrixFromProjection(&tracking.Eye[eye].ProjectionMatrix);
    }
    layer.Header.Flags |= ovr::ovrFrameLayerFlags::VRAPI_FRAME_LAYER_FLAG_CHROMATIC_ABERRATION_CORRECTION as u32;

    // render eye images
    for eye in 0..renderer.num_buffers as usize {
        // NOTE: In the non-mv case, latency can be further reduced by updating the sensor prediction
        // for each eye (updates orientation, not position)
        let mut framebuffer = &mut renderer.frame_buffers[eye];
        framebuffer::ovr_framebuffer_set_current(framebuffer);

        unsafe {
            glEnable(GL_SCISSOR_TEST);
            glDepthMask(GL_TRUE);
            glEnable(GL_DEPTH_TEST);
            glDepthFunc(GL_LEQUAL);
            glEnable(GL_CULL_FACE);
            glCullFace(GL_BACK);
            glViewport(0, 0, framebuffer.width, framebuffer.height);
            glScissor(0, 0, framebuffer.width, framebuffer.height);
            glClearColor(0f32, 0f32, 0f32, 1.0f32);//(0.125f32, 0.125f32, 0.125f32, 1.0f32);
            glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);

            let program = &scene.shader_programs[0];
            glUseProgram(program.program);
            graphics::bind_scene_matrices_ubo(eye as i32, &program, scene.scene_matrices);
            /*glBindBufferBase(GL_UNIFORM_BUFFER,
                             scene.shader_programs[0].uniform_binding[shader::ProgramUniformIndex::UniformSceneMatrices as usize] as u32,
                             scene.scene_matrices);
            // NOTE: will not be present when multiview path is enabled.
            let l = scene.shader_programs[0].uniform_location[shader::ProgramUniformIndex::UniformViewId as usize];
            if l >= 0 {
                glUniform1i(l, eye as i32);
            }*/

            let model_matrix = ovrMatrix4f_Transpose(&ovrMatrix4f_CreateIdentity());
            glUniformMatrix4fv(program.uniform_location[shader::ProgramUniformIndex::UniformModelMatrix as usize],
            1, GL_FALSE, &model_matrix as *const _ as *const GLfloat);

            // draw tabletop
            glBindVertexArray(scene.tabletop.vertex_array_object);
            glDrawElements(GL_TRIANGLES, scene.tabletop.index_count as GLsizei, GL_UNSIGNED_SHORT, std::ptr::null());
            glBindVertexArray(0);

            // draw controller
            let controller_model_matrix = ovrMatrix4f_Transpose(&ovrMatrix4f_CreateFromQuaternion(&scene.controller_orientation));
            glUniformMatrix4fv(program.uniform_location[shader::ProgramUniformIndex::UniformModelMatrix as usize],
                               1, GL_FALSE, &controller_model_matrix as *const _ as *const GLfloat);
            glBindVertexArray(scene.controller.vertex_array_object);
            glDrawElements(GL_TRIANGLES, scene.controller.index_count as GLsizei, GL_UNSIGNED_SHORT, std::ptr::null());
            glBindVertexArray(0);

            // draw bear
            let mob_program = &scene.shader_programs[2];
            glUseProgram(mob_program.program);
            graphics::bind_scene_matrices_ubo(eye as i32, &mob_program, scene.scene_matrices);

            //let bear_model_matrix = matrix4x4_transpose(&matrix4x4_mul(&matrix4x4_translation(&float3::new(3.0, 0.0, 0.0)), &scene.mob_skinned_mesh.skeletal_entity.local_to_world));
            let bear_model_matrix = matrix4x4_transpose(&matrix4x4_translation(&float3::new(3.0, 0.0, 0.0)));

            glUniformMatrix4fv(mob_program.uniform_location[shader::ProgramUniformIndex::UniformModelMatrix as usize],
                               1, GL_FALSE, &bear_model_matrix as *const _ as *const GLfloat);

            // joints
            glBindBufferBase(GL_UNIFORM_BUFFER,
                             program.uniform_binding[shader::ProgramUniformIndex::UniformJointMatrices as usize] as u32,
                             scene.mob_jointbuf.buffer);

            glActiveTexture(GL_TEXTURE0);
            glBindTexture(GL_TEXTURE_2D, scene.mob_texture);
            glBindVertexArray(scene.mob.vertex_array_object);
            glDrawElements(GL_TRIANGLES, scene.mob.index_count as GLsizei, GL_UNSIGNED_SHORT, std::ptr::null());
            glBindTexture(GL_TEXTURE_2D, 0);
            glBindVertexArray(0);

            glUseProgram(0);

            // Explicitly clear the border texels to black when GL_CLAMP_TO_BORDER is not available.
            if !extns.EXT_texture_border_clamp {
                // Clear to fully opaque black.
                glClearColor(0.0f32, 0.0f32, 0.0f32, 1.0f32);
                // bottom
                glScissor(0, 0, framebuffer.width, 1);
                glClear(GL_COLOR_BUFFER_BIT);
                // top
                glScissor(0, framebuffer.height - 1, framebuffer.width, 1);
                glClear(GL_COLOR_BUFFER_BIT);
                // left
                glScissor(0, 0, 1, framebuffer.height);
                glClear( GL_COLOR_BUFFER_BIT);
                // right
                glScissor(framebuffer.width - 1, 0, 1, framebuffer.height);
                glClear(GL_COLOR_BUFFER_BIT);
            }

            framebuffer::ovr_framebuffer_resolve(framebuffer);
            framebuffer::ovr_framebuffer_advance(&mut framebuffer);
        }

        framebuffer::ovr_framebuffer_set_none();
    }
}

#[cfg(target_os = "android")]
fn handle_app_event(app: &mut OvrApp, ev: Event) {
    if ev == Event::Resume {
        app.resumed = true;
        println!("->resume");
    }
    if ev == Event::Pause {
        app.resumed = false;
        println!("->pause");
    }
    if ev == Event::Destroy {
        app.destroyed = true;
        println!("->destroy");
    }
    if ev == Event::WindowCreated {
        app.window_active = true;
        println!("->window_created");
    }
    if ev == Event::WindowDestroyed {
        app.window_active = false;
        println!("->window_destroyed");
    }
}

#[cfg(target_os = "android")]
fn handle_vrmode_changes(app: &mut OvrApp) {
    if app.resumed && app.window_active {
        if app.ovr.is_none() {
            let mut params = vrapi_DefaultModeParms(&app.java);
            // No need to reset the FLAG_FULLSCREEN window flag when using a View
            params.Flags &= !(ovr::ovrModeFlags::VRAPI_MODE_FLAG_RESET_WINDOW_FULLSCREEN as u32);
            params.Flags |= ovr::ovrModeFlags::VRAPI_MODE_FLAG_NATIVE_WINDOW as u32;
            params.Display = unsafe { *(&mut app.egl.display) as c_ulonglong };
            //params.WindowSurface = unsafe { *(&mut app.egl.tiny_surface) as c_ulonglong };
            params.WindowSurface = unsafe { (*(&mut native_window().as_ref().unwrap().ptr().as_ptr())) as c_ulonglong };
            params.ShareContext = unsafe { *(&mut app.egl.context) as c_ulonglong };
            /*params.Display = (&app.egl.display) as *const _ as c_ulonglong;
            params.WindowSurface = native_window().as_ref().unwrap().ptr().as_ptr() as *const _ as c_ulonglong;
            params.ShareContext = (&app.egl.context) as *const _ as c_ulonglong;*/

            println!("vrapi_EnterVrMode()");
            let ovr = unsafe { ovr::vrapi_EnterVrMode(&params) };
            println!("eglGetCurrentSurface(EGL_DRAW) = {:?}", egl::get_current_surface());
            if ovr.is_null() {
                eprintln!("vrapi_EnterVrMode FAILED invalid ANativeWindow");
            } else {
                app.ovr = Option::Some(ovr);
                println!("raising perf");
                unsafe {
                    ovr::vrapi_SetClockLevels(app.ovr.unwrap(), app.cpu_level, app.gpu_level);
                    ovr::vrapi_SetPerfThread(app.ovr.unwrap(), ovr::ovrPerfThreadType::VRAPI_PERF_THREAD_TYPE_MAIN, app.main_thread_tid as u32);
                }
            }
        }
    } else {
        if app.ovr.is_some() {
            println!("vrapi_LeaveVrMode()");
            unsafe { ovr::vrapi_LeaveVrMode(app.ovr.unwrap()); }
            app.ovr = Option::None;
        }
    }
}

mod sys {
    use libc::{c_int, timespec};

    extern "C" {
        pub fn clock_gettime(clk_id: c_int, tp: *mut timespec) -> c_int;
    }
    pub const CLOCK_PROCESS_CPUTIME_ID: c_int = 2;
}

pub fn clock_seconds() -> f64 {
    unsafe {
        let mut now = std::mem::uninitialized();
        sys::clock_gettime(sys::CLOCK_PROCESS_CPUTIME_ID, &mut now);
        //return (now.tv_sec * 1e9 + now.tv_nsec) * 0.000000001;
        return ((now.tv_sec as f64) * 1e9f64 + (now.tv_nsec as f64)) * 0.000000001f64;
    }
}

struct SimpleLogger;
impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= LOGGER_LEVEL_FILTER
    }
    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }
    fn flush(&self) {}
}

#[derive(Copy, Clone, Debug)]
struct UniversalTime {
    /// time since program start at the beginning of this frame in seconds
    pub frame_time: f64,
    /// time since beginning of last frame in seconds
    pub delta_time: f64,
    /// system time at start of program in seconds
    pub program_start_time: f64,
    /// system time in seconds
    pub system_time: f64
}

impl Display for UniversalTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Time[frame_time={}, delta_time={}, program_start_time={}, system_time={}]",
            self.frame_time, self.delta_time, self.program_start_time, self.system_time)
    }
}