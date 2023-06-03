use std::{path::{Path, PathBuf}, env, fs};
fn get_output_path() -> PathBuf {
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let build_type = env::var("PROFILE").unwrap();
    return PathBuf::from(Path::new(&manifest).join("target").join(&build_type));
}

fn main() {
    println!("cargo:rerun-if-changed=config.json");
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let output_path = get_output_path();

    let input_path = Path::new(&manifest).join("config.json");
    let output_path = Path::new(&output_path).join("config.json");
    fs::copy(input_path, output_path).unwrap();
}