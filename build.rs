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
        .cpp(true)
        .std("c++20")
        .compile("rapid");

    println!("cargo:rerun-if-changed=src/rapid.rs");
    println!("cargo:rerun-if-changed=wrapper.cc");
    println!("cargo:rerun-if-changed=wrapper.hpp");
}
