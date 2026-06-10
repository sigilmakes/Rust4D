{
  description = "Rust4D - 4D game engine development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        # Runtime libraries wgpu/winit/cpal need to dlopen
        runtimeLibs = with pkgs; [
          vulkan-loader
          wayland
          libxkbcommon
          # X11 fallback for winit
          libx11
          libxcursor
          libxi
          libxrandr
          # audio (kira -> cpal -> alsa)
          alsa-lib
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            # Rust toolchain
            rustc
            cargo
            clippy
            rustfmt
            rust-analyzer

            # Build deps
            pkg-config
            alsa-lib

            # GPU debugging / verification
            vulkan-tools           # vulkaninfo
            vulkan-validation-layers
            mesa                   # lavapipe software rasterizer for headless runs

            # Visual verification tooling
            grim                   # Wayland screenshots (Hyprland)
            slurp                  # region selection
            imagemagick            # compare/montage/identify for image diffing
          ];

          buildInputs = runtimeLibs;

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;

          shellHook = ''
            echo "Rust4D dev shell — $(rustc --version)"
            # Use the system Vulkan ICDs (NixOS exposes them via /run/opengl-driver)
            if [ -d /run/opengl-driver/share/vulkan/icd.d ] && [ -z "''${VK_DRIVER_FILES:-}" ]; then
              export VK_DRIVER_FILES=$(ls /run/opengl-driver/share/vulkan/icd.d/*.json | tr '\n' ':' | sed 's/:$//')
            fi
          '';
        };
      });
}
