use std::process::Command;

fn main() {
    let src_mtime = std::fs::read_dir("src").unwrap()
        .filter(|s| s.as_ref().unwrap().path().extension().as_deref() == Some(std::ffi::OsStr::new("glsl")))
        .map(|s| s.as_ref().unwrap().metadata().unwrap().modified().unwrap())
        .max()
        .unwrap();
    let dst_mtime = std::fs::read_dir(std::env::var("OUT_DIR").unwrap()).unwrap()
        .filter(|s| s.as_ref().unwrap().path().extension().as_deref() == Some(std::ffi::OsStr::new("spv")))
        .map(|s| s.as_ref().unwrap().metadata().unwrap().modified().unwrap())
        .max();

    if dst_mtime.unwrap_or(std::time::SystemTime::UNIX_EPOCH) < src_mtime {
        compile_shaders();
    }
}

fn compile_shaders() {
    for name in &["shader.vert.glsl", "shader.frag.glsl"] {
        let path = std::env::var("VULKAN_SDK").expect("must have env var $VULKAN_SDK");
        let status = Command::new(
            std::path::Path::new(&path).join(r"Bin/glslangValidator.exe")
        )
            .arg("-V")
            .arg("-o")
            .arg(format!(
                "{}/{}.spv",
                std::env::var("OUT_DIR").unwrap(),
                name
            ))
            .arg(format!("src/{}", name))
            .status()
            .unwrap();
        assert!(status.success(), "{}", status);
    }
}
