use std::fs;
use std::process::Command;

fn main() {
    for name in &["shader.vert.glsl", "shader.frag.glsl"] {
        let status = Command::new(r"C:\VulkanSDK\1.2.162.0\Bin\glslangValidator.exe")
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
