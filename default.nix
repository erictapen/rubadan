# SPDX-FileCopyrightText: 2026 Kerstin Humm <kerstin@erictapen.name>
#
# SPDX-License-Identifier: GPL-3.0-or-later

{
  lib,
  rustPlatform,
  trunk,
  rustc,
  makeBinaryWrapper,
  wasm-bindgen-cli_0_2_126,
  binaryen,
  brotli,
  ibm-plex,
  noto-fonts,
  libglvnd,
  libx11,
  libxcursor,
  libxi,
  libxcb,
  libxkbcommon,
  wayland,
}:
let
  wasm-bindgen-cli = wasm-bindgen-cli_0_2_126;
  common = {
    pname = "rubadan";
    version = "0.1.0";
    src = ./.;
    cargoLock = {
      lockFile = ./Cargo.lock;
    };

    postPatch = ''
      mkdir -p assets/fonts
      cp ${ibm-plex}/share/fonts/opentype/* assets/fonts/
      cp ${noto-fonts}/share/fonts/noto/NotoSansSymbols2-Regular.otf assets/fonts/
    '';

    passthru = { inherit wasm-bindgen-cli; };

    meta.mainProgram = "rubadan";
  };
in
{
  web = rustPlatform.buildRustPackage (
    common
    // rec {

      env.TRUNK_BUILD_PUBLIC_URL = "";

      nativeBuildInputs = [
        trunk
        rustc.llvmPackages.lld
        wasm-bindgen-cli
        binaryen
        brotli
      ];

      buildFeatures = [ "demo" ];

      buildPhase = ''
        trunk build \
          --offline \
          --frozen \
          --features=${lib.concatStringsSep "," buildFeatures} \
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
        libx11
        libxcursor
        libxi
        libxcb
        libxkbcommon
        wayland
      ];

      nativeBuildInputs = [
        rustc.llvmPackages.lld
        makeBinaryWrapper
      ];

      postInstall = ''
        wrapProgram $out/bin/rubadan \
          --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath buildInputs}"
      '';

    }
  );
}
