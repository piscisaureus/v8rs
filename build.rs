
use cc;

fn main() {
    cc::Build::new()
        .cpp(true)
        .debug(true)
        .file("src_c/lib.cpp")
        .compile("bridge");
}