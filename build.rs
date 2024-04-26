use std::env;
use std::path::PathBuf;

#[allow(dead_code)]
fn bindgen() {
    let rapidjson_include = std::env::var("RAPIDJSON_INCLUDE");
    let rapidjson_include = match rapidjson_include {
        Ok(env_value) => std::path::PathBuf::from(env_value),
        Err(err) => {
            let msg = format!("\nCan't find RAPIDJSON_INCLUDE env var because {err:?}.\nYou can also set it in .cargo/config.toml under the [env] table.");
            println!("{msg}");
            std::process::exit(1)
        }
    };

    if !rapidjson_include.exists() {
        println!("RAPIDJSON_INCLUDE value {} does not exist.", rapidjson_include.display());
        std::process::exit(1)
    }

    let rapidjson_include : &str = rapidjson_include.to_str().expect("cannot convert path in RAPIDJSON_INCLUDE");

    // Tell cargo to look for shared libraries in the specified directory
    // println!("cargo:rustc-link-search=/path/to/lib");

    // Tell cargo to tell rustc to link the system bzip2
    // shared library.
    // println!("cargo:rustc-link-lib=bz2");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .clang_args(["-I", rapidjson_include])
        .header("wrapper.hpp")
        .allowlist_type("RustHandler")
        .allowlist_type("RustStream")
        .allowlist_function("parse")
        .vtable_generation(true)
        .generate_block(true)
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

fn main() {
    let rapidjson_include = std::env::var("RAPIDJSON_INCLUDE");
    let rapidjson_include = match rapidjson_include {
        Ok(env_value) => std::path::PathBuf::from(env_value),
        Err(err) => {
            let msg = format!("\nCan't find RAPIDJSON_INCLUDE env var because {err:?}.\nYou can also set it in .cargo/config.toml under the [env] table.");
            println!("{msg}");
            std::process::exit(1)
        }
    };

    if !rapidjson_include.exists() {
        println!("RAPIDJSON_INCLUDE value {} does not exist.", rapidjson_include.display());
        std::process::exit(1)
    }

    cxx_build::bridge("src/rapid.rs")
        .include(rapidjson_include)
        .file("wrapper.cc")
        // probably because I forgot jch prefix to wrapper.hpp in the include!
        // .include(std::path::Path::new("."))
        .compile("jch");

    println!("cargo:rerun-if-changed=src/rapid.rs");
    println!("cargo:rerun-if-changed=wrapper.cc");
    println!("cargo:rerun-if-changed=wrapper.hpp");
}
