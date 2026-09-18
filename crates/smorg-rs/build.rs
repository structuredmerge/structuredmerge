mod build_support;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build_support.rs");
    println!("cargo:rerun-if-env-changed=SMORG_BUILD_REVISION");
    println!("cargo:rerun-if-env-changed=SMORG_BUILD_SOURCE_STATE");
    let environment = std::env::vars_os()
        .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)))
        .collect();
    let source = build_support::render(&environment).expect("invalid CLI build identity");
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo OUT_DIR"));
    std::fs::write(output.join("smorg_build.rs"), source)
        .expect("write generated CLI build identity");
}
