use std::env;
use std::path::PathBuf;

fn feature_export() -> bool {
    cfg!(feature = "export")
}

fn feature_parallel() -> bool {
    cfg!(feature = "parallel")
}

fn feature_static() -> bool {
    cfg!(feature = "static")
}

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap();

    let mut cmake_config = cmake::Config::new("vendor/manifold");

    cmake_config
        .define("BUILD_SHARED_LIBS", if feature_static() { "OFF" } else { "ON" } )
        .define("MANIFOLD_TEST", "OFF")
        .define("MANIFOLD_CBIND", "ON")
        .define("MANIFOLD_CROSS_SECTION", "ON")
        .define("MANIFOLD_PAR", if feature_parallel() { "ON" } else { "OFF" })
        .define("MANIFOLD_EXPORT", if feature_export() { "ON" } else { "OFF" })
        .out_dir(out_dir.clone());

    if target_os == "windows" {
        cmake_config.cxxflag("/EHsc");
    }

    let dst = cmake_config.build();

    if feature_export() {
        println!("cargo:rustc-link-lib=assimp");
    }
    if feature_parallel() {
        println!("cargo:rustc-link-lib=tbb");
    }

    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib={}=manifold", if feature_static() { "static" } else { "dylib" });
    println!("cargo:rustc-link-lib={}=manifoldc", if feature_static() { "static" } else { "dylib" });

    match (
        target_arch.as_str(),
        target_os.as_str(),
        target_env.as_str(),
    ) {
        (_, "linux", _) | (_, "windows", "gnu") | (_, "android", _) => {
            println!("cargo:rustc-link-lib=dylib=stdc++")
        }
        (_, "macos", _) | (_, "ios", _) => println!("cargo:rustc-link-lib=dylib=c++"),
        (_, "windows", "msvc") => {}
        ("wasm32", "emscripten", _) => {
            println!("cargo:rustc-link-arg=--no-entry");
        }
        _ => unimplemented!(
            "target_os: {}, target_env: {}",
            target_os.as_str(),
            target_env.as_str()
        ),
    }

    generate_bindings(&out_dir)
}

fn generate_bindings(out_dir: &PathBuf) {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    let mut bindings_builder = bindgen::Builder::default()
        .header("vendor/manifold/bindings/c/include/manifold/manifoldc.h")
        .clang_arg("-Ivendor/manifold/bindings/c/include");

    if feature_export() {
        bindings_builder = bindings_builder.clang_arg("-DMANIFOLD_EXPORT");
    }

    let mut bindings_builder =
        bindings_builder.parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    if target_arch == "wasm32" && target_os == "emscripten" {
        // Workaround for bug:
        // https://github.com/rust-lang/rust-bindgen/issues/751
        bindings_builder = bindings_builder.clang_arg("-fvisibility=default");
    }

    bindings_builder
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
