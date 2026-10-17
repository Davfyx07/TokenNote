{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  buildInputs = with pkgs; [
    cargo
    rustc
    rust-analyzer
    clippy
    
    # GTK4 + Layer Shell
    gtk4
    gtk4-layer-shell
    pkg-config
    
    # Glib / GObject / Cairo / Pango
    glib
    gobject-introspection
    cairo
    pango
    
    # TLS / OpenSSL
    openssl
  ];

  shellHook = ''
    export PKG_CONFIG_PATH="${pkgs.gtk4.dev}/lib/pkgconfig:${pkgs.gtk4-layer-shell}/lib/pkgconfig:$PKG_CONFIG_PATH"
  '';
}
