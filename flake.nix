{
  description = "An interpreted language with continuations";

  outputs = { self, nixpkgs }:
    let
      pkgs = nixpkgs.legacyPackages.x86_64-linux;
    in
    rec {
      packages.x86_64-linux.tuzuku = pkgs.rustPlatform.buildRustPackage
        rec {
          pname = "tuzuku";
          version = "0.1.0";

          src = self;
          cargoLock.lockFile = "${src}/Cargo.lock";
        };
      defaultPackage.x86_64-linux = packages.x86_64-linux.tuzuku;

      devShell.x86_64-linux = pkgs.mkShell {
        buildInputs = [
          pkgs.nixpkgs-fmt
          pkgs.cargo-insta
        ];
      };
    };
}
