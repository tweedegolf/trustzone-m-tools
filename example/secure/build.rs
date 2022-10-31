fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../memory.x");

    trustzone_m_tools::generate_bindings("../non-secure/src/main.rs", true).unwrap();
}
