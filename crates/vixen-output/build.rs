fn main() {
    println!("cargo:rerun-if-changed=native/rds_http.c");
    cc::Build::new()
        .file("native/rds_http.c")
        .std("c11")
        .warnings(true)
        .compile("vixen_rds_http");
    println!("cargo:rustc-link-lib=curl");
}
