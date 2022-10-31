fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../memory.x");

    trustzone_m_tools::generate_bindings("../secure/src/main.rs", false).unwrap();
}
