use std::env;
use std::path::PathBuf;

fn main() {
    // Tell cargo to rerun this build script if these files change
    println!("cargo:rerun-if-changed=ONEcode/ONElib.c");
    println!("cargo:rerun-if-changed=ONEcode/ONElib.h");

    // Compile the C library
    cc::Build::new()
        .file("ONEcode/ONElib.c")
        .include("ONEcode")
        .flag("-fPIC")
        .flag("-fno-strict-aliasing")
        .flag("-DNDEBUG")
        .warnings(true)
        .extra_warnings(true)
        .opt_level(3)
        .compile("ONE");

    // Link against pthread (required by ONElib.c)
    println!("cargo:rustc-link-lib=pthread");

    // Generate bindings
    let bindings = bindgen::Builder::default()
        .header("ONEcode/ONElib.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Types to whitelist
        .allowlist_type("OneFile")
        .allowlist_type("OneSchema")
        .allowlist_type("OneProvenance")
        .allowlist_type("OneReference")
        .allowlist_type("OneCounts")
        .allowlist_type("OneStat")
        .allowlist_type("OneType")
        .allowlist_type("OneField")
        .allowlist_type("OneCodec")
        .allowlist_type("I8")
        .allowlist_type("I16")
        .allowlist_type("I32")
        .allowlist_type("I64")
        .allowlist_type("U8")
        // Functions to whitelist
        .allowlist_function("oneFile.*")
        .allowlist_function("oneSchema.*")
        .allowlist_function("one.*")
        // Variables to whitelist
        .allowlist_var("DNAcodec")
        .allowlist_var("oneTypeString")
        // Generate rust enums for C enums
        .rustified_enum("OneType")
        // Use core instead of std for no_std support (future)
        .use_core()
        .clang_arg("-I./ONEcode")
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to $OUT_DIR/bindings.rs
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
