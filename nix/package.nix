# The whole mcps workspace as a single package: five MCP server binaries out
# of one cargo invocation.
#
# One derivation rather than five was deliberate. The crates share a workspace
# and a lockfile, and four of the five dependency graphs are near-identical
# (rmcp + tokio + serde is most of the build), so five separate derivations
# would each vendor and compile that graph -- five times the build, bought
# with the ability to install smhi-mcp without emacs-mcp. Not worth it.
#
# Kept callPackage-able rather than inlined in flake.nix so the overlay and any
# bare `pkgs.callPackage ./nix/package.nix { }` get the same derivation from
# the same file.
{
  lib,
  rustPlatform,
  pkg-config,
  openssl,
}:

rustPlatform.buildRustPackage {
  pname = "mcps";
  version = "0.1.0";

  # An explicit allow-list, not `../.`. Inside a flake git already hides
  # target/, but `pkgs.callPackage` against a plain path -- which the overlay
  # makes possible -- has no such protection. Leaving flake.nix, README.org and
  # the justfile out also means editing them never invalidates a Rust build.
  #
  # lib.fileset over lib.cleanSourceWith: an allow-list of paths cannot
  # silently start matching something new. lib.fileset.gitTracked was rejected
  # -- it needs a .git directory, absent once this flake is an input.
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../emacs-mcp
      ../gcal-mcp
      ../notify-mcp
      ../pdf-mcp
      ../smhi-mcp
    ];
  };

  # The committed lockfile is the source of truth, so there is no cargoHash to
  # bump. outputHashes is needed because rmcp is a git dependency rather than a
  # crates.io one -- Cargo.lock pins the rev but records no content hash, so
  # Nix needs its own. One entry covers rmcp-macros too: importCargoLock keys
  # by git checkout, and both crates come out of the same tree.
  #
  # When the rmcp rev in Cargo.toml/Cargo.lock moves, re-run:
  #   nix run nixpkgs#nix-prefetch-git -- \
  #     --url https://github.com/modelcontextprotocol/rust-sdk --rev <rev>
  # and paste the "hash" field below.
  cargoLock = {
    lockFile = ../Cargo.lock;
    outputHashes = {
      "rmcp-1.7.0" = "sha256-eI+tdnfbP2EmwmXr9DmyI4/n8YaU+Zw2StMPEBDu0qs=";
    };
  };

  nativeBuildInputs = [
    # openssl-sys finds libssl/libcrypto through pkg-config.
    pkg-config
  ];

  buildInputs = [
    # reqwest (gcal-mcp, smhi-mcp) is on its default features, which means
    # default-tls -> hyper-tls -> native-tls -> openssl-sys. A real link-time
    # dep: libssl.so ends up in the runtime closure. Switching those two crates
    # to features = ["rustls-tls"] would drop this line and the closure with
    # it, but that is a change to the Rust code, not to packaging.
    openssl
  ];

  # Deliberately absent:
  #   * dbus     -- notify-mcp uses notify-rust 4, which speaks D-Bus through
  #                 zbus. zbus is pure Rust and talks to the bus socket
  #                 directly, so nothing links libdbus-1.
  #   * clang /  -- no crate here uses bindgen.
  #     bindgenHook
  #   * installShellFiles -- none of the five binaries has a CLI. They read
  #                 configuration from the environment (EMACS_SOCKET_NAME) and
  #                 otherwise just speak MCP over stdio, so there is nothing to
  #                 generate completions for.

  # No cargoBuildFlags: the root Cargo.toml is a virtual manifest with no
  # default-members, so a bare `cargo build` already builds all five members.
  #
  # doCheck is left at its default true. Every test that needs a running Emacs,
  # a notification daemon, sioyek, a user config file or the network is marked
  # #[ignore], and `cargo test` skips those -- what remains parses string
  # fixtures offline.

  meta = {
    description = "MCP servers for Emacs, Google Calendar, desktop notifications, PDFs and SMHI weather";
    homepage = "https://github.com/ast/mcps";
    license = with lib.licenses; [
      mit
      asl20
    ];
    # emacs-mcp shells out to emacsclient, notify-mcp needs a freedesktop
    # notification daemon, pdf-mcp drives sioyek. Nothing here is portable off
    # a Linux desktop.
    platforms = lib.platforms.linux;
    # No mainProgram on purpose. Five stdio JSON-RPC servers have no meaningful
    # default, and `nix run` on any of them would just block on stdin waiting
    # for an MCP client. Name the binary explicitly if you ever want one.
  };
}
