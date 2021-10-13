# rust ovr mobile
demo oculus vr app for gearvr

# notes
setup
- rustup target add aarch64-linux-android
- cargo install cargo-apk
- install android sdk, set ANDROID_SDK_ROOT and ANDROID_NDK_ROOT

build
cargo apk build


----------------


set RUSTFLAGS="-Clink-arg=Wl,-soname=libautofighter_android.so" & cargo ndk --platform 21 --target aarch64-linux-android



-----------

soname
https://android-developers.googleblog.com/2016/06/android-changes-for-ndk-developers.html

use patchelf to set soname on dylib?

--------------

use readelf to check if so has soname

Android\Sdk\ndk\21.1.6352462\toolchains\llvm\prebuilt\windows-x86_64\aarch64-linux-android\bin>readelf.exe -d libautofighter_android.so

-------------------

zlib1g system lib needed by cargo apk
sudo apt-get install zlib1g:i386


------

ideal ui resolution 13 pixels per degree


---------

TODO ndk manifest.rs support metadata for <<meta-data android:name="com.samsung.android.vr.application.mode" android:value="vr_only"/>>
TODO launchMode="singleTask"
TODO theme style as per oculus comment
