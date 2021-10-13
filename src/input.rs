use ovr_mobile_sys::{ovrDeviceID, ovrInputCapabilityHeader, ovrInputStateTrackedRemote, ovrInputTrackedRemoteCapabilities, ovrMobile, vrapi_EnumerateInputDevices, vrapi_GetCurrentInputState, vrapi_GetInputDeviceCapabilities};
use ovr_mobile_sys::ovrControllerCapabilities_::ovrControllerCaps_ModelGearVR;
use ovr_mobile_sys::ovrControllerType_::{ovrControllerType_Headset, ovrControllerType_TrackedRemote};
use ovr_mobile_sys::ovrDeviceIdType_::ovrDeviceIdType_Invalid;
use ovr_mobile_sys::ovrSuccessResult_::ovrSuccess;
use std::collections::HashSet;

pub struct DeviceInput {
    pub known_devices: HashSet<ovrDeviceID>,
    pub controller_single: Option<ovrDeviceID>,
    pub controller_left: Option<ovrDeviceID>,
    pub controller_right: Option<ovrDeviceID>
}

pub fn create_device_input() -> DeviceInput {
    DeviceInput {
        known_devices: HashSet::new(),
        controller_single: None,
        controller_left: None,
        controller_right: None
    }
}

pub fn scan_input_devices(ovr: *mut ovrMobile, input: &mut DeviceInput) {
    let mut device_index = 0;
    let mut device_caps_header: ovrInputCapabilityHeader = unsafe { std::mem::zeroed() };
    loop {
        let result = unsafe { vrapi_EnumerateInputDevices(ovr, device_index, &mut device_caps_header) };
        if result < 0 {
            break;
        }
        device_index += 1;

        // device id != device index
        let device_id = device_caps_header.DeviceID;
        if device_id == (ovrDeviceIdType_Invalid as u32) {
            eprintln!("found invalid input device at index {}", device_index);
            continue;
        }
        let known_device = input.known_devices.contains(&(device_id as ovrDeviceID));
        if !known_device {
            println!("found new input device id {} type {:?}", device_id, device_caps_header.Type);
            input.known_devices.insert(device_id as ovrDeviceID);
        }

        if device_caps_header.Type == ovrControllerType_TrackedRemote {
            let mut remote_caps: ovrInputTrackedRemoteCapabilities = unsafe { std::mem::zeroed() };
            remote_caps.Header = device_caps_header;

            unsafe {
                if (ovrSuccess as i32) != vrapi_GetInputDeviceCapabilities(ovr, &mut remote_caps.Header) {
                    eprintln!("vrapi_GetInputDeviceCapabilities FAILED");
                    continue;
                }

                let is_gear_vr = (remote_caps.ControllerCapabilities & (ovrControllerCaps_ModelGearVR as u32)) != 0;

                let mut state: ovrInputStateTrackedRemote = unsafe { std::mem::zeroed() };
                state.Header.ControllerType = ovrControllerType_TrackedRemote;

                if (ovrSuccess as i32) != vrapi_GetCurrentInputState(ovr, device_id, &mut state.Header) {
                    eprintln!("vrapi_GetCurrentInputState FAILED");
                    continue;
                }

                input.controller_single = Option::Some(device_id);
            }
        }
        if device_caps_header.Type == ovrControllerType_Headset {

        }
    }
}