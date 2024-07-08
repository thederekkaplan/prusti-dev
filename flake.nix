{
  description = "A static verifier for Rust, based on the Viper verification infrastructure.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    crane.url = "github:ipetkov/crane";
    crane.inputs.nixpkgs.follows = "nixpkgs";

    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";

    viper.url = "git+https://github.com/thederekkaplan/viperserver.git?submodules=1";
    viper.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, crane, rust-overlay, viper }: {
    packages = nixpkgs.lib.genAttrs 
      ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"] 
      (system: let 
        pkgs = import nixpkgs { 
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        viper-server = viper.packages."${system}".default;

        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain;
        
        craneLib = (crane.mkLib pkgs).overrideToolchain rust;

        rustToolchainFilter = path: type: builtins.match ".*rust-toolchain$" path != null;
        testOutputFilter = path: type: builtins.match ".*\\.std(out|err)" path != null;
        craneFilter = path: type: 
          (rustToolchainFilter path type) || 
          (testOutputFilter path type) || 
          (craneLib.filterCargoSources path type);
      in rec {
        vendoredDeps = craneLib.vendorMultipleCargoDeps {
          cargoLockList = [./Cargo.lock ./prusti-contracts/Cargo.lock];
        };

        default = craneLib.buildPackage rec {
          src = pkgs.lib.cleanSourceWith {
            src = ./.;
            filter = craneFilter;
            name = "source";
          };

          cargoVendorDir = vendoredDeps;

          buildInputs = [ 
            viper-server
            pkgs.jdk11
            pkgs.openssl
            pkgs.zlib
            pkgs.libiconv
            pkgs.curl
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.Security
          ];

          rpath = pkgs.lib.makeLibraryPath buildInputs;

          preBuild = ''
            export JAVA_HOME="${pkgs.jdk11}"
            export RUST_SYSROOT="${rust}"
            export VIPER_HOME="${viper-server}/server"
            export Z3_EXE="${pkgs.z3_4_12}/bin/z3"
            export LD_LIBRARY_PATH="${rpath}"
            export DYLD_LIBRARY_PATH="${pkgs.jdk11}/lib/jli:${rpath}"
          '';

          env = {
            JAVA_HOME="${pkgs.jdk11}";
            RUST_SYSROOT = "${rust}";
            VIPER_HOME = "${viper-server}/server";
            Z3_EXE = "${pkgs.z3_4_12}/bin/z3";
            LD_LIBRARY_PATH = "${rpath}";
            DYLD_LIBRARY_PATH="${pkgs.jdk11}/lib/jli:${rpath}";
          };

          # SIP on recent Macs resets DYLD_LIBRARY_PATH, so we have to set it again
          shellHook = ''export DYLD_LIBRARY_PATH="${pkgs.jdk11}/lib/jli:${rpath}"'';
        };
      });
  };


    # utils.lib.eachDefaultSystem (system:
    #   let
    #     pkgs = import nixpkgs {
    #       inherit system;
    #       overlays = [ rust-overlay.overlays.default ];
    #     };

    #     viper-server = viper.packages."${system}".default;

    #     rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain;

    #     naersk-lib = naersk.lib."${system}".override {
    #       cargo = rust;
    #       rustc = rust;
    #     };

    #     prusti-version = "${self.tag or "${self.lastModifiedDate}.${self.shortRev or "dirty"}"}";
    #   in rec {
    #     packages = {
    #       prusti = naersk-lib.buildPackage {
    #         name = "prusti";
    #         version = "${prusti-version}";
    #         root = ./.;
    #         buildInputs = [
    #           pkgs.pkg-config
    #           pkgs.wget
    #           pkgs.gcc
    #           pkgs.openssl
    #           pkgs.jdk11
    #           pkgs.zlib
    #           viper-server
    #           packages.ow2_asm
    #         ];
    #         nativeBuildInputs = [
    #           pkgs.makeWrapper
    #         ];

    #         copyTarget = true;
    #         compressTarget = false;
    #         singleStep = true;

    #         cargoBuildOptions = x: x ++ ["-Z bindeps" "-v"];

    #         override = _: {
    #           preBuild = ''
    #             export RUST_SYSROOT="${rust}"
    #             export LD_LIBRARY_PATH="${pkgs.jdk11}/lib/jli:$LD_LIBRARY_PATH"
    #             export DYLD_LIBRARY_PATH="${pkgs.jdk11}/lib/jli:$DYLD_LIBRARY_PATH"
    #             export VIPER_HOME="${viper-server}/server"
    #             export Z3_EXE="${viper-server}/z3/bin/z3"
    #             export ASM_JAR="${packages.ow2_asm}/asm.jar"
    #           '';
    #         };
    #         overrideMain = _: {
    #           postInstall = ''
    #             rm $out/bin/test-crates

    #             for f in $(find $out/bin/ $out/libexec/ -type f -executable); do
    #               wrapProgram $f \
    #                 --set RUST_SYSROOT "${rust}" \
    #                 --set JAVA_HOME "${pkgs.jdk11}/lib/openjdk" \
    #                 --set LD_LIBRARY_PATH "${pkgs.jdk11}/lib/jli:$LD_LIBRARY_PATH" \
    #                 --set DYLD_LIBRARY_PATH "${pkgs.jdk11}/lib/jli:$DYLD_LIBRARY_PATH" \
    #                 --set VIPER_HOME "${viper-server}/server" \
    #                 --set Z3_EXE "${viper-server}/z3/bin/z3"
    #             done

    #             mkdir $out/bin/deps
    #             cp $out/target/release/libprusti_contracts.rlib $out/bin
    #             cp $out/target/release/deps/libprusti_contracts_internal-* $out/bin/deps
    #             rm -rf $out/target
    #             rm $out/bin/deps/*.{rlib,rmeta}
    #           '';
    #         };
    #       };

    #       ow2_asm = pkgs.stdenv.mkDerivation rec {
    #         name = "asm";
    #         version = "3.3.1";
    #         src = pkgs.fetchurl {
    #           url = "https://repo.maven.apache.org/maven2/${name}/${name}/${version}/${name}-${version}.jar";
    #           hash = "sha256-wrOSdfjpUbx0dQCAoSZs2rw5OZvF4T1kK/LTRkSd9/M=";
    #         };
    #         dontUnpack = true;
    #         dontBuild = true;
    #         installPhase = ''
    #           mkdir $out
    #           cp ${src} $out/asm.jar
    #         '';
    #       };
    #     };

    #     checks = {
    #       # prusti-test = naersk-lib.buildPackage {
    #       #   name = "prusti-test";
    #       #   version = "${prusti-version}";
    #       #   root = ./.;
    #       #   checkInputs = [
    #       #     pkgs.pkg-config
    #       #     pkgs.wget
    #       #     pkgs.gcc
    #       #     pkgs.openssl
    #       #     pkgs.jdk11
    #       #     packages.viper
    #       #     packages.ow2_asm
    #       #   ];

    #       #   doCheck = true;

    #       #   override = _: {
    #       #     preBuild = ''
    #       #       export LD_LIBRARY_PATH="${pkgs.jdk11}/lib/openjdk/lib/server"
    #       #       export VIPER_HOME="${packages.viper}/backends"
    #       #       export Z3_EXE="${packages.viper}/z3/bin/z3"
    #       #       export ASM_JAR="${packages.ow2_asm}/asm.jar"
    #       #     '';
    #       #     preCheck = ''
    #       #       export RUST_SYSROOT="${rust}"
    #       #       export JAVA_HOME="${pkgs.jdk11}/lib/openjdk"
    #       #       export LD_LIBRARY_PATH="${pkgs.jdk11}/lib/openjdk/lib/server"
    #       #       export VIPER_HOME="${packages.viper}/backends"
    #       #       export Z3_EXE="${packages.viper}/z3/bin/z3"
    #       #     '';
    #       #   };
    #       # };

    #       prusti-simple-test = pkgs.runCommand "prusti-simple-test" {
    #         buildInputs = [
    #           defaultPackage
    #           rust
    #         ];
    #       }
    #       ''
    #         cargo new --name example $out/example
    #         sed -i '1s/^/use prusti_contracts::*;\n/;s/println.*$/assert!(true);/' $out/example/src/main.rs
    #         cargo-prusti --manifest-path=$out/example/Cargo.toml
    #       '';
    #     };

    #     defaultPackage = packages.prusti;
    #   }
    # );
}
