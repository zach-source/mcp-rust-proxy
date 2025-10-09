use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // Skip backend build for now - will handle separately
    // let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    // if target_os != "android" && target_os != "ios" {
    //     build_backend_binary();
    // }

    tauri_build::build()
}

fn build_backend_binary() {
    println!("cargo:rerun-if-changed=../../crates/mcp-proxy-server");

    let out_dir = env::var("OUT_DIR").unwrap();
    let target = env::var("TARGET").unwrap();
    let profile = env::var("PROFILE").unwrap();

    // Determine the binary name based on the platform
    let binary_name = if cfg!(windows) {
        "mcp-proxy-server.exe"
    } else {
        "mcp-proxy-server"
    };

    println!("Building mcp-proxy-server for target: {}", target);

    // Build the mcp-proxy-server binary
    let mut cmd = Command::new("cargo");
    cmd.args(&[
        "build",
        "--package",
        "mcp-proxy-server",
        "--target",
        &target,
    ]);

    // Use release mode if we're building in release
    if profile == "release" {
        cmd.arg("--release");
    }

    let output = cmd
        .current_dir("../..")
        .output()
        .expect("Failed to build backend binary");

    if !output.status.success() {
        panic!(
            "Backend build failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Create binaries directory
    let binaries_dir = Path::new(&out_dir).join("../../../binaries");
    fs::create_dir_all(&binaries_dir).expect("Failed to create binaries directory");

    // Copy the built binary to the binaries directory
    let source_path = format!(
        "../../target/{}/{}/{}",
        target,
        if profile == "release" {
            "release"
        } else {
            "debug"
        },
        binary_name
    );
    let dest_path = binaries_dir.join(binary_name);

    println!("Copying {} to {:?}", source_path, dest_path);

    fs::copy(&source_path, &dest_path).expect(&format!(
        "Failed to copy backend binary from {} to {:?}",
        source_path, dest_path
    ));

    // Make the binary executable on Unix platforms
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&dest_path)
            .expect("Failed to get binary metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dest_path, perms).expect("Failed to set binary permissions");
    }

    println!("Backend binary built and copied successfully");
}
