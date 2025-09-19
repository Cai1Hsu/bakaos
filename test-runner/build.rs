use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use serde_json::Value;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=linker");

    if is_baremetal() {
        apply_linker_script();
    } else {
        pass_coverage_output_file();
    }

    generate_link_workaround();
}

fn is_baremetal() -> bool {
    std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "none"
}

fn get_target_arch() -> String {
    std::env::var("CARGO_CFG_TARGET_ARCH").unwrap()
}

fn apply_linker_script() {
    let arch = get_target_arch();
    let linker_script = format!("test-runner/linker/{}-virt.ld", arch);

    println!("cargo:rustc-link-arg=-T{}", linker_script);
}

fn get_target_directory() -> String {
    let command = std::process::Command::new("cargo")
        .args([
            "metadata",
            "--offline",
            "--no-deps",
            "--format-version",
            "1",
        ])
        .output()
        .expect("Failed to get target directory");

    let output = String::from_utf8_lossy(&command.stdout);

    let json: Value = serde_json::from_str(&output).expect("Failed to parse JSON output");

    json["target_directory"].as_str().unwrap().to_string()
}

fn get_target_triplet() -> String {
    std::env::var("TARGET").unwrap()
}

fn get_build_profile() -> String {
    std::env::var("PROFILE").unwrap()
}

fn pass_coverage_output_file() {
    const KEY: &str = "COVERAGE_OUTPUT_FILE";

    if std::env::var(KEY).is_ok() {
        return; // do not override existing value
    }

    let target_dir = get_target_directory();

    let file = PathBuf::new().join(target_dir).join(format!(
        "coverage-{}-{}.profraw",
        get_build_profile(),
        get_target_triplet(),
    ));

    println!("cargo:rustc-env={}={}", KEY, file.to_string_lossy());
}

fn collect_local_dependencies() -> Vec<Dependency> {
    let current_crate_name = std::env::var("CARGO_PKG_NAME").unwrap();

    let command = std::process::Command::new("cargo")
        .args([
            "metadata",
            "--offline",
            "--no-deps",
            "--format-version",
            "1",
        ])
        .output()
        .expect("Failed to get workspace members");

    let output = String::from_utf8_lossy(&command.stdout);

    let metadata: WorkspaceMetadata =
        serde_json::from_str(&output).expect("Failed to parse JSON output");

    // Select current package
    let current_package = metadata
        .packages
        .iter()
        .find(|pkg| pkg.name == current_crate_name)
        .expect("Current package not found");

    current_package
        .dependencies
        .iter()
        .filter(|d| d.path.is_some() && d.target.is_none())
        .cloned()
        .collect()
}

fn generate_link_workaround() {
    let dependencies = collect_local_dependencies();
    let workaround = generate_link_workaround_string(&dependencies);

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated.rs");

    generate_file(&dest_path, workaround.as_bytes());
}

fn generate_link_workaround_string(crates: &[Dependency]) -> String {
    let imports = crates
        .iter()
        .map(|d| {
            format!(
                "    extern crate {};",
                d.rename.as_ref().unwrap_or(&d.name).replace('-', "_")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"
#[doc(hidden)]
#[rustfmt::skip]
mod _generated {{
{imports}
}}"#,
    )
}

fn generate_file<P: AsRef<Path>>(path: P, text: &[u8]) {
    let mut file = File::create(path).unwrap();
    file.write_all(text).unwrap()
}

#[derive(Deserialize)]
struct WorkspaceMetadata {
    packages: Vec<Package>,
}

#[derive(Deserialize, Clone)]
struct Package {
    name: String,
    dependencies: Vec<Dependency>,
}

#[derive(Deserialize, Clone)]
struct Dependency {
    name: String,
    rename: Option<String>,
    path: Option<String>,
    target: Option<String>,
}
