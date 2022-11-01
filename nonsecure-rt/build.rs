use std::{env, fs::File, io::Write, path::PathBuf};

fn main() {
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());

    let linker_scripts = vec![
        (&include_bytes!("link.x.in")[..], "link.x"),
        (
            &include_bytes!("../trustzone_memory.x.in")[..],
            "trustzone_memory.x",
        ),
        (
            &include_bytes!("../no_region_asserts.x.in")[..],
            "region_asserts.x",
        ),
    ];

    for (script_bytes, script_name) in linker_scripts {
        let mut f = File::create(out.join(script_name)).unwrap();
        f.write_all(script_bytes).unwrap();

        println!("cargo:rerun-if-changed={script_name}.in");
    }

    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=build.rs");
}
