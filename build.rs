//use cc;
use cpp_build;

fn main() {
  cc::Build::new()
    .cpp(true)
    .flag("-std:c++17")
    .debug(true)
    .file("src_c/lib.cpp")
    .compile("bridge");
}
