fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=linker");

    if is_baremetal() {
        apply_linker_script();
    }
}

fn is_baremetal() -> bool {
    std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "none"
}

fn target_arch() -> String {
    std::env::var("CARGO_CFG_TARGET_ARCH").unwrap()
}

fn apply_linker_script() {
    let arch = target_arch();
    let linker_script = format!("test-runner/linker/{}-virt.ld", arch);

    println!("cargo:rustc-link-arg=-T{}", linker_script);
}
