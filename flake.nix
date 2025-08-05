{
  description = "MCP Rust Proxy - High-performance Model Context Protocol proxy server";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, crane, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Rust toolchain configuration
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
          targets = [
            "wasm32-unknown-unknown"
            "x86_64-unknown-linux-gnu"
            "aarch64-unknown-linux-gnu"
            "x86_64-apple-darwin"
            "aarch64-apple-darwin"
          ];
        };

        # Create crane lib with our custom toolchain
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Common build inputs
        commonBuildInputs = with pkgs; [
          openssl
          pkg-config
          perl # Required for OpenSSL build
        ] ++ lib.optionals stdenv.isDarwin [
          darwin.apple_sdk.frameworks.Security
          darwin.apple_sdk.frameworks.SystemConfiguration
        ];

        # Native build inputs
        nativeBuildInputs = with pkgs; [
          rustToolchain
          trunk
          wasm-bindgen-cli
          binaryen
        ];

        # Source filtering to improve caching
        src = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = path: type:
            (pkgs.lib.hasSuffix "\.html" path) ||
            (pkgs.lib.hasSuffix "\.css" path) ||
            (pkgs.lib.hasSuffix "\.js" path) ||
            (pkgs.lib.hasInfix "/yew-ui/" path) ||
            (craneLib.filterCargoSources path type);
        };

        # Common cargo args
        commonArgs = {
          inherit src;
          strictDeps = true;
          buildInputs = commonBuildInputs;
          nativeBuildInputs = nativeBuildInputs;
        };

        # Build dependencies only (for caching)
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Function to build for a specific target
        buildForTarget = target: extraArgs: craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          cargoExtraArgs = "--target ${target}";
          
          # Set environment variables for the build
          BUILD_YEW_UI = "1";
          CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER = 
            pkgs.lib.optionalString (target == "aarch64-unknown-linux-gnu") 
              "${pkgs.pkgsCross.aarch64-multiplatform.stdenv.cc}/bin/aarch64-unknown-linux-gnu-gcc";
          
          # Add cross-compilation dependencies
          depsBuildBuild = pkgs.lib.optionals (target == "aarch64-unknown-linux-gnu") [
            pkgs.pkgsCross.aarch64-multiplatform.stdenv.cc
          ];
          
          # Pre-build hook to build Yew UI
          preBuild = ''
            # Build Yew UI
            pushd yew-ui
            trunk build --release
            popd
            
            # Copy built UI to expected location
            mkdir -p yew-dist
            cp -r yew-ui/dist/* yew-dist/
          '';
        } // extraArgs);

        # Build packages for all targets
        packages = {
          default = buildForTarget "x86_64-unknown-linux-gnu" { };
          
          x86_64-linux = buildForTarget "x86_64-unknown-linux-gnu" { };
          aarch64-linux = buildForTarget "aarch64-unknown-linux-gnu" { };
          
          # macOS builds (only on Darwin)
        } // pkgs.lib.optionalAttrs pkgs.stdenv.isDarwin {
          x86_64-darwin = buildForTarget "x86_64-apple-darwin" { };
          aarch64-darwin = buildForTarget "aarch64-apple-darwin" { };
        };

        # Cross-compilation packages (Linux only)
        crossPackages = pkgs.lib.optionalAttrs pkgs.stdenv.isLinux {
          # Cross-compile from Linux to macOS
          x86_64-darwin-cross = let
            darwinPkgs = import nixpkgs {
              system = "x86_64-darwin";
              overlays = overlays;
            };
          in buildForTarget "x86_64-apple-darwin" {
            depsBuildBuild = [ pkgs.xcbuild ];
            buildInputs = commonBuildInputs ++ [
              darwinPkgs.darwin.apple_sdk.frameworks.Security
              darwinPkgs.darwin.apple_sdk.frameworks.SystemConfiguration
            ];
          };
        };

      in
      {
        packages = packages // crossPackages // {
          # Docker image
          docker = pkgs.dockerTools.buildLayeredImage {
            name = "mcp-rust-proxy";
            tag = "latest";
            contents = [
              packages.default
              pkgs.cacert
            ];
            config = {
              Cmd = [ "${packages.default}/bin/mcp-rust-proxy" ];
              ExposedPorts = {
                "3000/tcp" = {};
                "3001/tcp" = {};
              };
            };
          };
        };

        # Development shell
        devShells.default = pkgs.mkShell {
          inputsFrom = [ packages.default ];
          
          buildInputs = with pkgs; [
            # Rust development tools
            rustToolchain
            rust-analyzer
            cargo-edit
            cargo-watch
            cargo-audit
            cargo-outdated
            cargo-release
            
            # Web development
            trunk
            wasm-bindgen-cli
            binaryen
            
            # General development tools
            git
            gh
            direnv
            nix-direnv
            
            # Testing tools
            cargo-nextest
            cargo-llvm-cov
            
            # For generating flamegraphs
            cargo-flamegraph
            
            # Database tools (for testing MCP servers)
            sqlite
            postgresql
          ];

          shellHook = ''
            echo "MCP Rust Proxy development environment"
            echo "Run 'cargo build' to build the project"
            echo "Run 'BUILD_YEW_UI=1 cargo build' to include the web UI"
            echo "Run 'cargo run -- --config mcp-proxy.yaml' to start the proxy"
          '';
        };

        # Apps for easy running
        apps.default = flake-utils.lib.mkApp {
          drv = packages.default;
        };
      });
}