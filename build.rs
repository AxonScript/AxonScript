// this build.rs script configures the build to work correctly with llvm
// it sets the path to search for native libraries, links against the LLVM shared library,
// and defines an environment variable needed by the llvm-sys crate
// you need to have llvm installed on your system,
// and if it's not in the default location, update the paths below accordingly


fn main() {
    
    //println!("cargo:rustc-link-search=native=/usr/lib");
    //println!("cargo:rustc-link-lib=dylib=LLVM-20");
    //println!("cargo:rustc-env=LLVM_SYS_201_PREFIX=/usr");
}
