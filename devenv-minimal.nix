{ pkgs, ... }:

{
  # Minimal configuration for testing
  packages = with pkgs; [
    cargo
    rustc
    libiconv
  ];

  languages.rust = {
    enable = true;
    channel = "stable";
  };

  enterShell = ''
    echo "Minimal devenv loaded"
  '';
}
