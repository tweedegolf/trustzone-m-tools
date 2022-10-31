use std::{env, fs::File, io::Write, path::PathBuf};

const LINKER_SCRIPTS: &[(&'static [u8], &'static str)] =
    &[(include_bytes!("../trustzone_memory.x.in"), "trustzone_memory.x")];

fn main() {
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());

    for (script_bytes, script_name) in LINKER_SCRIPTS {
        let mut f = File::create(out.join(script_name)).unwrap();
        f.write_all(script_bytes).unwrap();

        println!("cargo:rerun-if-changed={script_name}.in");
    }

    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=build.rs");
}
