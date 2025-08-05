# Compatibility wrapper for non-flake Nix users
{ pkgs ? import <nixpkgs> {} }:

let
  # Import flake outputs
  flake = builtins.getFlake (toString ./.);
  
  # Get the default devShell for current system
  devShell = flake.devShells.${builtins.currentSystem}.default;
in
  # For non-flake usage, just use nixpkgs directly
  if devShell != null
  then devShell
  else pkgs.mkShell {
    buildInputs = with pkgs; [
      # Rust toolchain
      rustc
      cargo
      rustfmt
      rust-analyzer
      clippy
      
      # Build dependencies
      pkg-config
      openssl
      
      # Web UI tools
      trunk
      wasm-bindgen-cli
      binaryen
      
      # Development tools
      git
      direnv
    ] ++ lib.optionals stdenv.isDarwin [
      darwin.apple_sdk.frameworks.Security
      darwin.apple_sdk.frameworks.SystemConfiguration
    ];
    
    shellHook = ''
      echo "MCP Rust Proxy development environment (non-flake)"
      echo "For better reproducibility, consider using 'nix develop' with flakes enabled"
    '';
  }