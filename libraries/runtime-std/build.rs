use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug)]
struct Module {
    unstable: bool,
    cfgs: Vec<String>,
}

fn rust_src_root() -> PathBuf {
    let sysroot = String::from_utf8(
        Command::new("rustc")
            .arg("--print")
            .arg("sysroot")
            .output()
            .expect("rustc --print sysroot failed")
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();

    Path::new(&sysroot)
        .join("lib")
        .join("rustlib")
        .join("src")
        .join("rust")
        .join("library")
}

fn collect_modules(crate_name: &str, src_root: &Path) -> HashMap<String, Module> {
    let root = src_root.join(crate_name).join("src");
    let lib_rs = root.join("lib.rs");
    let contents =
        fs::read_to_string(&lib_rs).unwrap_or_else(|_| panic!("cannot read {:?}", lib_rs));

    let mut modules = HashMap::new();

    let regex = regex::Regex::new(
        r#"(?m)^(?:\S.*)?pub\s+(?:mod\s+|use\s+(?:[a-zA-Z_][a-zA-Z0-9_]*::)*)([a-zA-Z_][a-zA-Z0-9_]*)\s*;"#,
    )
    .unwrap();

    for cap in regex.captures_iter(&contents) {
        let module = &cap[1];
        let mut unstable = false;

        let mut path = root.join(format!("{module}.rs"));
        if !path.is_file() {
            path = root.join(module).join("mod.rs");
        }
        if let Ok(code) = fs::read_to_string(&path) {
            unstable = code.contains("#![unstable");
        }

        modules.insert(
            module.to_string(),
            Module {
                unstable,
                cfgs: vec![],
            },
        );
    }

    modules
}

fn generate_module(name: &str, namespaces: &[(String, &Module)]) -> Option<String> {
    const SKIP_MODULES: &[&str] = &["prelude", "cfg_select"];

    if SKIP_MODULES.contains(&name) {
        return None;
    }

    let mut out = format!("pub mod {name} {{\n");

    for (ns_name, module) in namespaces {
        let mut cfgs = vec![];
        if ns_name != "core" {
            cfgs.push(format!("feature = \"{ns_name}\""));
        }
        if module.unstable {
            cfgs.push("feature = \"unstable\"".to_string());
        }
        cfgs.extend(module.cfgs.iter().cloned());

        if cfgs.len() == 1 {
            out.push_str(&format!("    #[cfg({})]\n", cfgs[0]));
        } else if !cfgs.is_empty() {
            out.push_str(&format!("    #[cfg(all({}))]\n", cfgs.join(", ")));
        }
        out.push_str(&format!("    pub use ::{ns_name}::{name}::*;\n"));
    }

    match name {
        "collections" => {
            let prefix = "    #[cfg(all(feature = \"alloc\", feature = \"compat_hash\"))] pub use hashbrown::";
            out.push_str(&(prefix.to_string() + "HashMap;\n"));
            out.push_str(&(prefix.to_string() + "HashSet;\n"));
        }
        "ffi" => {
            let prefix = "    #[cfg(feature = \"alloc\")] pub use cstr_core::";
            out.push_str(&(prefix.to_string() + "CStr;\n"));
        }
        _ => {}
    }

    out.push_str("}\n");
    Some(out)
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/generated/mod.rs");

    let out_dir = PathBuf::from("src/generated");
    let dest = out_dir.join("mod.rs");

    let src_root = rust_src_root();
    let mut core = collect_modules("core", &src_root);
    let mut alloc = collect_modules("alloc", &src_root);

    let core_unstables = [
        "async_iter",
        "contracts",
        "bstr",
        "f128",
        "f16",
        "io",
        "pat",
        "random",
        "range",
        "ub_checks",
        "unsafe_binder",
    ];
    for m in core_unstables {
        core.entry(m.to_string())
            .or_insert(Module {
                unstable: true,
                cfgs: vec![],
            })
            .unstable = true;
    }

    alloc
        .entry("sync".to_string())
        .or_insert(Module {
            unstable: false,
            cfgs: vec![],
        })
        .cfgs
        .push("not(target_os = \"none\")".to_string());
    alloc
        .entry("task".to_string())
        .or_insert(Module {
            unstable: false,
            cfgs: vec![],
        })
        .cfgs
        .push("not(target_os = \"none\")".to_string());

    let core_keys: BTreeSet<_> = core.keys().cloned().collect();
    let alloc_keys: BTreeSet<_> = alloc.keys().cloned().collect();

    let mut generated = BTreeMap::new();

    for m in core_keys.intersection(&alloc_keys) {
        generated.insert(
            m.clone(),
            generate_module(
                m,
                &[
                    ("core".to_string(), &core[m]),
                    ("alloc".to_string(), &alloc[m]),
                ],
            ),
        );
    }

    for m in core_keys.difference(&alloc_keys) {
        generated.insert(
            m.clone(),
            generate_module(m, &[("core".to_string(), &core[m])]),
        );
    }

    for m in alloc_keys.difference(&core_keys) {
        generated.insert(
            m.clone(),
            generate_module(m, &[("alloc".to_string(), &alloc[m])]),
        );
    }

    let mut file = String::new();
    file.push_str("//! Generated by build.rs\n");
    file.push_str("mod manual_fix;\n");
    file.push_str("pub use manual_fix::*;\n");

    for (_name, code) in generated {
        if let Some(code) = code {
            file.push_str(&code);
        }
    }

    fs::write(&dest, file).unwrap();
}
