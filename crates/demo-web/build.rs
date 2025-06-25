use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Copy the HTML file to the output directory for easy access
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("../../../index.html");

    if let Ok(_) = fs::copy("index.html", &dest_path) {
        println!("cargo:warning=Copied index.html to output directory");
    }

    println!("cargo:rerun-if-changed=index.html");
}
