{
  description = "A static verifier for Rust, based on the Viper verification infrastructure.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    crane.url = "github:ipetkov/crane";
    rust-overlay.url = "github:oxalica/rust-overlay";
    viper.url = "git+https://github.com/thederekkaplan/viperserver.git?submodules=1";
  };

  outputs = inputs@{ self, nixpkgs, flake-parts, crane, rust-overlay, viper }: 
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];

      perSystem = { pkgs, system, ... }: let
        viper-server = viper.packages."${system}".default;

        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain;
        craneLib = (crane.mkLib pkgs).overrideToolchain rust;

        rustToolchainFilter = path: type: builtins.match ".*rust-toolchain$" path != null;
        testOutputFilter = path: type: builtins.match ".*\\.std(out|err)" path != null;
        craneFilter = path: type: 
          (rustToolchainFilter path type) || 
          (testOutputFilter path type) || 
          (craneLib.filterCargoSources path type);

        vendoredDeps = craneLib.vendorMultipleCargoDeps {
          cargoLockList = [./Cargo.lock ./prusti-contracts/Cargo.lock];
        };

        ow2_asm = pkgs.stdenv.mkDerivation rec {
          name = "asm";
          version = "3.3.1";
          src = pkgs.fetchurl {
            url = "https://repo.maven.apache.org/maven2/${name}/${name}/${version}/${name}-${version}.jar";
            hash = "sha256-wrOSdfjpUbx0dQCAoSZs2rw5OZvF4T1kK/LTRkSd9/M=";
          };
          dontUnpack = true;
          dontBuild = true;
          installPhase = ''mkdir $out && cp ${src} $out/asm.jar'';
        };

        mapAndConcat = f: attrset: sep: builtins.concatStringsSep sep 
          (builtins.attrValues (builtins.mapAttrs f attrset));

        base = rec {
          pname = "prusti";
          version = "${self.tag or "${self.lastModifiedDate}.${self.shortRev or "dirty"}"}";

          src = pkgs.lib.cleanSourceWith {
            src = ./.;
            filter = craneFilter;
            name = "source";
          };

          cargoVendorDir = vendoredDeps;

          buildInputs = [
            pkgs.jdk11
            rust
            viper-server
            ow2_asm
            pkgs.openssl
            pkgs.zlib
            pkgs.libiconv
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.Security
          ];

          nativeBuildInputs = [
            pkgs.makeWrapper
          ];

          env = let rpath = pkgs.lib.makeLibraryPath buildInputs; in {
            JAVA_HOME = "${pkgs.jdk11}";
            RUST_SYSROOT = "${rust}";
            VIPER_HOME = "${viper-server}/server";
            Z3_EXE = "${viper-server}/z3/bin/z3";
            LD_LIBRARY_PATH = "${rpath}";
            DYLD_LIBRARY_PATH = "${pkgs.jdk11}/lib/jli:${rpath}";
            ASM_JAR = "${ow2_asm}/asm.jar";
          };

          preBuild = mapAndConcat (name: value: ''export ${name}="${value}"'') env "; ";

          postInstall = ''
            for f in $(find $out/bin/ -type f -executable); do
              wrapProgram $f ${mapAndConcat (name: value: ''--set-default ${name} "${value}"'') env " "}
            done
          '';

          # SIP on recent Macs resets DYLD_LIBRARY_PATH, so we have to set it again
          shellHook = ''export DYLD_LIBRARY_PATH="${env.DYLD_LIBRARY_PATH}"'';
        };
      in {
        _module.args.pkgs = import nixpkgs { 
          inherit system; 
          overlays = [ rust-overlay.overlays.default ];
        };

        packages.default = craneLib.buildPackage (base // {doCheck = false;});
        packages.rust = rust;
        checks.default = craneLib.buildPackage (base // {doCheck = true; pnameSuffix = "-test";});
      };
    };
}
