{
  lib,
  rustc,
  rustPlatform,
  perl,
}:

rustPlatform.buildRustPackage {
  pname   = "zellij-mcp-server";
  version = "0.1.1";

  src = lib.cleanSource ./.;

  cargoLock.lockFile = ./Cargo.lock;

  nativeBuildInputs = [
    perl
  ];

  CARGO_BUILD_RUSTFLAGS = lib.optionalString
    rustc.stdenv.hostPlatform.isMusl
    "-C target-feature=+crt-static";

  cargoBuildFlags = [ "--bin" "zellij-mcp-server" ];

  doCheck = false;

  meta = with lib; {
    description = "MCP server for the Zellij terminal multiplexer, written in Rust";
    homepage    = "https://github.com/qrzbing/zellij-mcp-server";
    license     = licenses.mit;
    mainProgram = "zellij-mcp-server";
    platforms   = platforms.linux;
  };
}
