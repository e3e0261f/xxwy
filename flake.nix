{
  description = "Xianxia Game Dev Shell";
  inputs.nixpkgs.url = "flake:nixpkgs";

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in {
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          # Rust 开发工具链
          rustc
          cargo
          clippy
          rustfmt
          
          # 网页编译目标 (WebAssembly)
          wasm-bindgen-cli
          
          # 项目构建工具
          trunk
          
          # 系统依赖
          pkg-config
          libxkbcommon
          libGL
          wayland
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
        ];

        # 这一步非常重要：显式告诉 cargo 你要编译的目标
        RUSTUP_TOOLCHAIN = "stable";
      };
    };
}
