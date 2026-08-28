{
  description = "mcps -- MCP servers (emacs, gcal, notify, pdf, smhi)";

  inputs = {
    # Same channel as the NixOS flake that consumes this. When consumed from
    # there this input is overridden by inputs.nixpkgs.follows, so it only
    # governs standalone `nix build` / `nix develop` in this repo.
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
  };

  outputs =
    { nixpkgs, ... }:
    let
      # x86_64-linux only, matching the machines that run these servers.
      # Darwin is out on the merits: emacs-mcp assumes a Linux emacsclient
      # setup, notify-mcp is D-Bus, pdf-mcp drives sioyek.
      systems = [ "x86_64-linux" ];

      # flake-utils would buy exactly this one line, at the price of an extra
      # input in every downstream lock.
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (
        pkgs:
        let
          mcps = pkgs.callPackage ./nix/package.nix { };
        in
        {
          inherit mcps;
          default = mcps;
        }
      );

      # The bridge into the NixOS flake. home-manager there runs with
      # useGlobalPkgs = true, so an overlay registered in any NixOS module
      # reaches home.packages too -- the only clean way to get a flake input
      # into a home module, since extraSpecialArgs passes only hostName.
      overlays.default = final: _prev: {
        mcps = final.callPackage ./nix/package.nix { };
      };

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          # Deliberately the same native deps as nix/package.nix. A dev shell
          # that cannot build gcal-mcp is worse than none: it fails at the end
          # of a long compile instead of at entry.
          nativeBuildInputs = [
            pkgs.rustc
            pkgs.cargo
            pkgs.clippy
            pkgs.rustfmt
            pkgs.rust-analyzer
            pkgs.just # the justfile is the day-to-day entry point
            pkgs.pkg-config
          ];

          buildInputs = [
            pkgs.openssl
          ];

          # `nix develop` prepends to PATH, so this toolchain shadows the one
          # home/albin.nix puts in the profile -- usually the same store paths
          # anyway, since both flakes track nixos-26.05.
          #
          # rust-analyzer resolves std sources through RUST_SRC_PATH; without
          # it "go to definition" into std silently does nothing.
          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };
      });

      # `nix fmt`. pkgs.nixfmt is the RFC-style formatter; nixfmt-rfc-style is
      # now just an alias that warns.
      formatter = forAllSystems (pkgs: pkgs.nixfmt);
    };
}
