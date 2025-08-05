use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=yew-ui/src");
    println!("cargo:rerun-if-changed=yew-ui/index.html");
    println!("cargo:rerun-if-changed=yew-ui/style.css");
    println!("cargo:rerun-if-changed=yew-ui/Cargo.toml");

    // Only build in release mode or when explicitly requested
    let profile = env::var("PROFILE").unwrap_or_default();
    let force_build = env::var("BUILD_YEW_UI").unwrap_or_default();
    
    if profile == "release" || force_build == "1" {
        build_yew_ui();
    } else {
        println!("cargo:warning=Skipping Yew UI build in debug mode. Set BUILD_YEW_UI=1 to force build.");
    }
}

fn build_yew_ui() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let yew_ui_dir = Path::new(&manifest_dir).join("yew-ui");
    
    // Check if trunk is installed
    let trunk_check = Command::new("trunk")
        .arg("--version")
        .output();
    
    if trunk_check.is_err() {
        println!("cargo:warning=Trunk is not installed.");
        println!("cargo:warning=Please install it manually: cargo install --locked trunk");
        println!("cargo:warning=Then run: ./build-yew-ui.sh");
        println!("cargo:warning=Skipping Yew UI build.");
        return;
    }
    
    // Build the Yew UI
    println!("cargo:warning=Building Yew UI...");
    
    let output = Command::new("trunk")
        .current_dir(&yew_ui_dir)
        .args(&["build", "--release"])
        .output();
    
    match output {
        Ok(result) => {
            if !result.status.success() {
                println!("cargo:warning=Yew UI build failed:");
                println!("cargo:warning={}", String::from_utf8_lossy(&result.stderr));
                println!("cargo:warning=You can build it manually with: ./build-yew-ui.sh");
                return;
            }
        }
        Err(e) => {
            println!("cargo:warning=Failed to execute trunk build: {}", e);
            println!("cargo:warning=You can build it manually with: ./build-yew-ui.sh");
            return;
        }
    }
    
    // Copy built files to a location accessible by the binary
    let dist_dir = yew_ui_dir.join("dist");
    let target_dir = Path::new(&manifest_dir).join("yew-dist");
    
    // Create target directory
    if let Err(e) = std::fs::create_dir_all(&target_dir) {
        println!("cargo:warning=Failed to create yew-dist directory: {}", e);
        return;
    }
    
    // Copy all files from dist to yew-dist
    if let Err(e) = copy_dir_all(&dist_dir, &target_dir) {
        println!("cargo:warning=Failed to copy dist files: {}", e);
        return;
    }
    
    println!("cargo:warning=Yew UI build completed successfully");
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);
        
        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    
    Ok(())
}