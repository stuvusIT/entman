with import <nixpkgs> {};

mkShell {
  buildInputs = [
    rustup
    cargo
    openssl
    pkg-config
    rust-analyzer
  ];

  shellHook = ''
    export RUSTUP_HOME="$HOME/.rustup"
    export CARGO_HOME="$HOME/.cargo"
    export PATH="$CARGO_HOME/bin:$PATH"
    
    # Installiere Nightly falls nicht vorhanden und setze es als Standard
    if ! rustup show | grep -q "nightly"; then
      rustup install nightly
    fi
    rustup default nightly
  '';

  # Optional: Toolchain explizit setzen
  RUSTUP_TOOLCHAIN = "nightly";

  # OpenSSL Konfiguration
  OPENSSL_DIR = "${pkgs.openssl.dev}";
  OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
  OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";
  PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
}

