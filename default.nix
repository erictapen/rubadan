{
  lib,
  rustPlatform,
  trunk,
  rustc,
  wasm-bindgen-cli,
  binaryen,
  ibm-plex,
  noto-fonts,
  publicUrl ? "/",
}:
rustPlatform.buildRustPackage {
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

  env.TRUNK_BUILD_PUBLIC_URL = publicUrl;

  nativeBuildInputs = [
    trunk
    rustc.llvmPackages.lld
    wasm-bindgen-cli
    binaryen
  ];

  buildPhase = ''
    mkdir -p assets/fonts
    cp ${ibm-plex}/share/fonts/opentype/* assets/fonts/
    cp ${noto-fonts}/share/fonts/noto/NotoSansSymbols2-Regular.otf assets/fonts/
    trunk build \
      --offline \
      --frozen \
      --release \
      --dist $out
  '';
  installPhase = "echo 'Skipping installPhase'";
  checkPhase = "echo 'Skipping installPhase'";
}
