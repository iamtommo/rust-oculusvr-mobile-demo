use ovr_mobile_sys::*;
use math::matrix::float4x4;

pub fn get_system_property_int(java: *const ovrJava, prop_type: ovrSystemProperty) -> i32 {
    unsafe {
        vrapi_GetSystemPropertyInt(java, prop_type) as i32
    }
}

pub fn get_system_property_float(java: *const ovrJava, prop_type: ovrSystemProperty) -> f32 {
    unsafe {
        vrapi_GetSystemPropertyFloat(java, prop_type) as f32
    }
}