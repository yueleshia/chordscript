{ nixpkgs ? import <nixpkgs> {}}:
nixpkgs.mkShell {
  buildInputs = with nixpkgs; [
    wasm-pack pkgconfig openssl
  ];
  NIX_SSL_CERT_FILE = "/etc/ssl/certs/ca-bundle.crt";
}
