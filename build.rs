use std::path::PathBuf;
// extern crate cbindgen;

// use std::env;

fn main() {
    // let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    // cbindgen::Builder::new()
    //     .with_crate(crate_dir)
    //     .generate()
    //     .expect("Unable to generate bindings")
    //     .write_to_file("bindings.h");

    // Tell cargo to look for shared libraries in the specified directory
    println!("cargo:rustc-link-search=/path/to/lib");

    // Tell cargo to tell rustc to link the system bzip2
    // shared library.
    println!("cargo:rustc-link-lib=bz2");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("include/xaio.h")
        .allowlist_type("xaio.*")
        .allowlist_function("xaio.*")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from("./");
    bindings
        .write_to_file(out_path.join("src/capi/xaio.rs"))
        .expect("Couldn't write bindings!");
}
