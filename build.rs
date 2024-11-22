use std::env;
use std::path::PathBuf;

fn feat_export() -> bool {
    cfg!(feature = "export")
}

fn feat_parallel() -> bool {
    cfg!(feature = "parallel")
}

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    compile_and_link_clipper2();

    let mut cc_build = cc::Build::new();
    cc_build
        .cpp(true)
        .warnings(false)
        .include("vendor/manifold/include")
        .include("vendor/Clipper2/CPP/Clipper2Lib/include")
        .include("vendor/manifold/bindings/c/include");

    if feat_parallel() {
        cc_build
            .include("vendor/oneTBB/include")
            .define("MANIFOLD_PAR", "1");
    } else {
        cc_build.define("MANIFOLD_PAR", "-1");
    }

    cc_build
        .warnings(false)
        .flag_if_supported("-std:c++17") // MSVC
        .flag_if_supported("-std=c++17");

    let mut cc_build_files = vec![
        "vendor/manifold/src/boolean3.cpp",
        "vendor/manifold/src/boolean_result.cpp",
        "vendor/manifold/src/constructors.cpp",
        "vendor/manifold/src/csg_tree.cpp",
        "vendor/manifold/src/edge_op.cpp",
        "vendor/manifold/src/face_op.cpp",
        "vendor/manifold/src/impl.cpp",
        "vendor/manifold/src/manifold.cpp",
        "vendor/manifold/src/polygon.cpp",
        "vendor/manifold/src/properties.cpp",
        "vendor/manifold/src/quickhull.cpp",
        "vendor/manifold/src/sdf.cpp",
        "vendor/manifold/src/smoothing.cpp",
        "vendor/manifold/src/sort.cpp",
        "vendor/manifold/src/subdivision.cpp",
        "vendor/manifold/src/cross_section/cross_section.cpp",
        "vendor/manifold/bindings/c/manifoldc.cpp",
        "vendor/manifold/bindings/c/conv.cpp",
        "vendor/manifold/bindings/c/box.cpp",
        "vendor/manifold/bindings/c/cross.cpp",
        "vendor/manifold/bindings/c/rect.cpp",
    ];

    if feat_export() {
        let mut assimp_config = std::fs::read_to_string("vendor/assimp/include/assimp/config.h.in")
            .expect("assimp config.h.in not found!");
        let replace_cmakedefine = regex::Regex::new(r"(?m)^#cmakedefine").unwrap();
        assimp_config = replace_cmakedefine
            .replace_all(&*assimp_config, "// #cmakedefine")
            .to_string();

        let assimp_include_folder = out_dir.join("assimp_include");
        let assimp_include_assimp_folder = assimp_include_folder.join("assimp");
        // Create {OUT_DIR}/assimp_include/assimp
        std::fs::create_dir_all(&assimp_include_assimp_folder)
            .expect("unable to create assimp config directory!");
        // Put file in {OUT_DIR}/assimp_include/assimp/config.h
        std::fs::write(assimp_include_assimp_folder.join("config.h"), assimp_config)
            .expect("could not write assimp config.h!");

        cc_build_files.push("vendor/manifold/src/meshIO/meshIO.cpp");
        cc_build
            .include(assimp_include_folder)
            .define("MANIFOLD_EXPORT", "ON")
            .include("vendor/assimp/include");
    }

    cc_build.files(cc_build_files).compile("manifold");

    println!("cargo:rustc-link-lib=manifold");

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap();

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

fn compile_and_link_clipper2() {
    cc::Build::new()
        .cpp(true)
        .include("vendor/Clipper2/CPP/Clipper2Lib/include")
        .include("vendor/Clipper2/CPP/Utils")
        .files([
            "vendor/Clipper2/CPP/Clipper2Lib/src/clipper.engine.cpp",
            "vendor/Clipper2/CPP/Clipper2Lib/src/clipper.offset.cpp",
            "vendor/Clipper2/CPP/Clipper2Lib/src/clipper.rectclip.cpp",
            "vendor/Clipper2/CPP/Utils/clipper.svg.cpp",
        ])
        .flag_if_supported("-std:c++17") // MSVC
        .flag_if_supported("-std=c++17")
        .compile("clipper2");
    println!("cargo:rustc-link-lib=clipper2");
}

fn generate_bindings(out_dir: &PathBuf) {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    let mut bindings_builder = bindgen::Builder::default()
        .header("vendor/manifold/bindings/c/include/manifold/manifoldc.h")
        .clang_arg("-Ivendor/manifold/bindings/c/include");

    if feat_export() {
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
