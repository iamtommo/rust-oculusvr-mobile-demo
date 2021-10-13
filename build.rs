fn main() {
    // link libstd++.so for vrapi
    println!("cargo:rustc-link-lib=stdc++");

    /*let target = env::var("TARGET").unwrap();
    let android = target.contains("android");

    // Export shared libraries search path.
    if android {
        let abi = if target.contains("aarch64") {
            "arm64-v8a"
        } else {
            "armeabi-v7a"
        };

        println!("cargo:rustc-link-search={}/lib/{}/Release", env!("CARGO_MANIFEST_DIR"), abi);
        println!("cargo:rustc-link-lib=dylib=vrapi");
    }*/
}