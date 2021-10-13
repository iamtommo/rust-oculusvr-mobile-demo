#[cfg(target_os = "android")]
use ndk_glue::native_activity;
#[cfg(target_os = "android")]
use ndk::asset::Asset as AndroidAsset;

use std::ffi::CString;
use std::io;

pub struct Asset {
    #[cfg(target_os = "android")]
    pub android_asset: AndroidAsset
}
impl Asset {
    pub fn get_buffer(&mut self) -> io::Result<&[u8]> {
        #[cfg(target_os = "android")]
            {
                return self.android_asset.get_buffer();
            }
        return Result::Err(io::Error::new(
            io::ErrorKind::Other,
            "Error reading asset",
        ));
    }
}

pub fn load_asset(file: &str) -> Option<Asset> {
    #[cfg(target_os = "android")]
    {
            return Option::Some(Asset {
                android_asset: native_activity().asset_manager().open(CString::new(file).unwrap().as_c_str()).unwrap()
            });
    }
    return Option::None;
}