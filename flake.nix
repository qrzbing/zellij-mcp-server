{
  description = "zellij-mcp-server - statically linked MCP server for Zellij";

  inputs = {
    nixpkgs.url     = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, flake-utils }:
    flake-utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        packages = {
          default           = pkgs.pkgsStatic.callPackage ./package.nix {};
          zellij-mcp-server = pkgs.pkgsStatic.callPackage ./package.nix {};
        };
      }
    );
}
