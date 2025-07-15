fn main() {
    if cfg!(feature = "bincode") {
        println!("cargo:rustc-env=BINCODE_FEATURE_ENABLED=1");
    }
}
