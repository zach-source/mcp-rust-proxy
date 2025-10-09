{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:

{
  # Enable devenv
  devenv.debug = false;

  # Package dependencies
  packages = with pkgs; [
    # Core build tools
    gcc
    gnumake
    pkg-config

    # Rust toolchain
    rustc
    cargo
    rustfmt
    clippy
    cargo-tauri

    # For building Yew UI
    trunk
    wasm-bindgen-cli

    # System libraries needed for Tauri
    libiconv

    # Additional tools
    nodejs_20
    nodePackages.npm

    # For development
    rust-analyzer
  ];

  # Environment variables
  env = {
    RUST_BACKTRACE = "1";
    RUST_LOG = "debug";

    # Set library paths for macOS
    LIBRARY_PATH = "${pkgs.libiconv}/lib";
  };

  # Rust configuration
  languages.rust = {
    enable = true;
    channel = "stable";
    components = [
      "rustfmt"
      "clippy"
      "rust-src"
    ];
    targets = [ "wasm32-unknown-unknown" ];
  };

  # Node.js for frontend tooling
  languages.javascript = {
    enable = true;
    package = pkgs.nodejs_20;
  };

  # Git hooks for code quality (disabled for now to avoid hanging)
  # git-hooks.hooks = {
  #   rustfmt.enable = true;
  #   clippy.enable = true;
  # };

  # Shell hooks
  enterShell = ''
    echo "🚀 MCP Rust Proxy Tauri Development Environment"
    echo ""
    echo "Available commands:"
    echo "  cargo tauri dev    - Run in development mode"
    echo "  cargo tauri build  - Build for production"
    echo "  cargo fmt --all    - Format all Rust code"
    echo "  cargo test         - Run tests"
    echo ""
    echo "Environment ready! The following have been configured:"
    echo "  ✓ Rust toolchain with wasm32-unknown-unknown target"
    echo "  ✓ Tauri CLI"
    echo "  ✓ Trunk for Yew UI"
    echo "  ✓ All required system libraries"
    echo ""
  '';

  # Scripts for common tasks
  scripts = {
    dev.exec = ''
      cd tauri-app
      cargo tauri dev
    '';

    build.exec = ''
      cd tauri-app
      cargo tauri build
    '';

    build-dmg.exec = ''
      cd tauri-app
      cargo tauri build --bundles dmg
    '';

    format.exec = ''
      cargo fmt --all
    '';

    test.exec = ''
      cargo test --all
    '';

    clean.exec = ''
      cargo clean
      cd tauri-app && cargo clean
      cd ../yew-ui && cargo clean
    '';
  };

  # Process management
  processes = {
    # Optional: Run the proxy server directly for testing
    proxy-server.exec = ''
      cargo run --package mcp-proxy-server -- --config configs/examples/basic.yaml
    '';
  };
}
