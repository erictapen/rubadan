{
  lib,
  rustPlatform,
  trunk,
  rustc,
  makeBinaryWrapper,
  wasm-bindgen-cli_0_2_108,
  binaryen,
  brotli,
  ibm-plex,
  noto-fonts,
  publicUrl ? "/",
  libglvnd,
  xorg,
  libxkbcommon,
  wayland,
}:
let
  common = {
    pname = "implementation";
    version = "0.1.0";
    src = ./.;
    cargoLock = {
      lockFile = ./Cargo.lock;
      outputHashes = {
        "ecolor-0.31.1" = "sha256-8ITdHxuDcDO1NL07luNQX3/7G3+5MmelYklVkqtGOFc=";
        "fontique-0.4.0" = "sha256-4Be9ytfKCUwrUVmLtyp0quLgk3aIOLvP3zXAf+pFWuU=";
        "swash-0.2.2" = "sha256-ZNsNFI3vKw7Yz+1aaGnmFK4lwS4tbw7F6oaGIw9cpJY=";
        "zeno-0.3.2" = "sha256-41ocDmCTHa4ov6ygJRqF1metrXmlqB7Ekj9FLno5SXU=";
      };
    };

    postPatch = ''
      mkdir -p assets/fonts
      cp ${ibm-plex}/share/fonts/opentype/* assets/fonts/
      cp ${noto-fonts}/share/fonts/noto/NotoSansSymbols2-Regular.otf assets/fonts/
    '';
  };
in
{
  web = rustPlatform.buildRustPackage (
    common
    // {

      env.TRUNK_BUILD_PUBLIC_URL = publicUrl;

      nativeBuildInputs = [
        trunk
        rustc.llvmPackages.lld
        wasm-bindgen-cli_0_2_108
        binaryen
        brotli
      ];

      buildPhase = ''
        trunk build \
          --offline \
          --frozen \
          --release \
          --dist $out
      '';
      installPhase = ''
        find $out -type f \
          -exec gzip --best --keep --force {} ';' \
          -exec brotli --best --keep --force {} ';' \
      '';
      checkPhase = "cargo test";
    }
  );
  native = rustPlatform.buildRustPackage (
    common
    // rec {

      buildInputs = [
        libglvnd
        xorg.libX11
        xorg.libXcursor
        xorg.libXi
        xorg.libxcb
        libxkbcommon
        wayland
      ];

      nativeBuildInputs = [
        rustc.llvmPackages.lld
        makeBinaryWrapper
      ];

      postInstall = ''
        wrapProgram $out/bin/implementation \
          --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath buildInputs}"
      '';

    }
  );
}
