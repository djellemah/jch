fn main() {
    let rapidjson_include = std::env::var("RAPIDJSON_INCLUDE");
    let rapidjson_include = match rapidjson_include {
        Ok(env_value) => std::path::PathBuf::from(env_value),
        Err(_) => "rapidjson/include".into(),
    };

    if !rapidjson_include.exists() {
        println!(r#"
            RAPIDJSON_INCLUDE env var path "{}" does not exist.
            Possibly you need to install the rapidjson dev library.
            You can also set RAPIDJSON_INCLUDE in .cargo/config.toml under the [env] table.
        "#, rapidjson_include.display());
        std::process::exit(1)
    }

    // switch on simd for rapidjson if possible
    let wrapper_defs_dir = &env::var_os("OUT_DIR").unwrap();
    let wrapper_defs_dir = Path::new(&wrapper_defs_dir);
    let dest_path = wrapper_defs_dir.join("wrapper_defs.h");
    use std::env;
    use std::path::Path;
    let target_env = std::env::var("TARGET");
    let define_value = match target_env {
        // srsly. There must be a better way.
        Ok(s) if s == "x86_64-unknown-linux-gnu" => "#define RAPIDJSON_SSE42".into(),
        Ok(s) if s == "x86_64-pc-windows-msvc" => "#define RAPIDJSON_SSE42".into(),
        Ok(s) if s == "aarch64-apple-darwin" => "#define RAPIDJSON_NEON".into(),
        wut => {
            let msg = format!("rapidjson SIMD not turned on because we don't know how to do that for target triple {:?}", wut);
            println!("cargo::warning={msg}");
            format!("// whaddya mean {:?}", wut)
        },
    };
    std::fs::write(&dest_path, define_value).unwrap();

    // Now tell cxx.rs what to do
    cxx_build::bridge("src/rapid.rs")
        .include(rapidjson_include)
        .include(wrapper_defs_dir)
        .file("src/wrapper.cc")
        .cpp(true)
        .std("c++20")
        .compile("rapid");

    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=src/rapid.rs");
    println!("cargo::rerun-if-changed=src/wrapper.cc");
    println!("cargo::rerun-if-changed=src/wrapper.h");
    println!("cargo::rerun-if-env-changed=RAPIDJSON_INCLUDE");
}
