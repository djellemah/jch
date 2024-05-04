fn main() {
    let rapidjson_include = std::env::var("RAPIDJSON_INCLUDE");
    let rapidjson_include = match rapidjson_include {
        Ok(env_value) => std::path::PathBuf::from(env_value),
        Err(_) => "rapidjson/include".into(),
    };

    if !rapidjson_include.exists() {
        println!(r#"
            RAPIDJSON_INCLUDE env var path "{}" does not exist.
            You can also set RAPIDJSON_INCLUDE it in .cargo/config.toml under the [env] table.
        "#, rapidjson_include.display());
        std::process::exit(1)
    }

    cxx_build::bridge("src/rapid.rs")
        .include(rapidjson_include)
        .file("src/wrapper.cc")
        .cpp(true)
        .std("c++20")
        .compile("rapid");

    println!("cargo::rerun-if-changed=src/rapid.rs");
    println!("cargo::rerun-if-changed=src/wrapper.cc");
    println!("cargo::rerun-if-changed=src/wrapper.h");
    println!("cargo::rerun-if-env-changed=RAPIDJSON_INCLUDE");
}
