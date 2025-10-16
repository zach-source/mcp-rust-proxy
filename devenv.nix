{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:

{
  # https://devenv.sh/basics/
  env.GREET = "MCP Rust Proxy Development Environment";

  # https://devenv.sh/packages/
  packages = with pkgs; [
    git
    gh
    trunk
    wasm-bindgen-cli
    binaryen
    cargo-edit
    cargo-watch
    cargo-audit
    cargo-outdated
    cargo-nextest
    cargo-flamegraph
    sqlite
    postgresql
  ];

  # https://devenv.sh/languages/
  languages.rust = {
    enable = true;
    channel = "stable";

    # Ensure we have proper target support
    targets = [
      "wasm32-unknown-unknown"
      "x86_64-unknown-linux-gnu"
      "aarch64-unknown-linux-gnu"
      "x86_64-apple-darwin"
      "aarch64-apple-darwin"
    ];
  };

  # https://devenv.sh/processes/
  # processes.cargo-watch.exec = "cargo-watch";

  # https://devenv.sh/services/
  # services.postgres.enable = true;

  # https://devenv.sh/scripts/
  scripts.hello.exec = ''
    echo hello from $GREET
  '';

  enterShell = ''
    echo "🦀 MCP Rust Proxy Development Environment"
    echo ""
    echo "Quick commands:"
    echo "  cargo build              - Build debug binary"
    echo "  cargo test               - Run all tests"
    echo "  BUILD_YEW_UI=1 cargo build - Build with web UI"
    echo "  cargo run -- --config mcp-proxy-config.yaml - Run proxy"
    echo ""
  '';

  # https://devenv.sh/tasks/
  # tasks = {
  #   "myproj:setup".exec = "mytool build";
  #   "devenv:enterShell".after = [ "myproj:setup" ];
  # };

  # https://devenv.sh/tests/
  enterTest = ''
    echo "Running tests..."
    cargo test --all-features
  '';

  # https://devenv.sh/pre-commit-hooks/
  # pre-commit.hooks.shellcheck.enable = true;

  # See full reference at https://devenv.sh/reference/options/
}
