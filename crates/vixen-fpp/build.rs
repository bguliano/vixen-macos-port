fn main() {
    println!("cargo:rerun-if-changed=native/fpp_http.c");
    cc::Build::new().file("native/fpp_http.c").std("c11").warnings(true).compile("vixen_fpp_http");
    println!("cargo:rustc-link-lib=curl");
}
