{
  description = "A TUI sequence diagram editor";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustToolchain = pkgs.rust-bin.stable.latest.default;
      in
      {
        packages = {
          tuigram = pkgs.rustPlatform.buildRustPackage {
            pname = "tuigram";
            version = "0.1.5";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;

            meta = with pkgs.lib; {
              description = "A TUI sequence diagram editor";
              homepage = "https://github.com/preiter93/tuigram";
              license = licenses.mit;
              maintainers = [ ];
              mainProgram = "tuigram";
            };
          };
          default = self.packages.${system}.tuigram;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            rust-analyzer
          ];
        };
      }
    );
}
