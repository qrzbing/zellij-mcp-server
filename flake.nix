{
  description = "zellij-mcp-server - MCP server for Zellij";

  inputs = {
    nixpkgs.url     = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        dynamic = pkgs.callPackage ./package.nix {};
        static = pkgs.pkgsStatic.callPackage ./package.nix {};
      in
      {
        packages = {
          # Default to dynamic (glibc) build.
          default = dynamic;

          # Explicit build variants.
          zellij-mcp-server = dynamic;
          zellij-mcp-server-dynamic = dynamic;
          zellij-mcp-server-static = static;
        };
      }
    );
}
