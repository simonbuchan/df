fn main() {
    cc::Build::new()
        .include("win-support")
        .include("original")
        .file("original/construct.c")
        .file("original/misc.c")
        .file("original/monotone.c")
        .file("original/tri.c")
        .flag_if_supported("/wd4131") // suppress "old-style declarator" warning
        .compile("seidel-triangulate");
}
