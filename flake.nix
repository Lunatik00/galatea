{
  description = "Galatea - probador virtual con IA generativa";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, crane }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
          config = { allowUnfree = true; android_sdk.accept_license = true; };
        };

        # Versión fijada a propósito. Elegida el 2026-09-26: la más reciente
        # estable del rust-overlay fijado. Actualízala junto con flake.lock.
        cadenaRust = pkgs.rust-bin.stable."1.98.1".default.override {
          extensions = [ "rust-src" "clippy" "rustfmt" ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain cadenaRust;

        # Los recursos y las fixtures de pruebas van dentro del binario o los leen
        # las pruebas: tienen que sobrevivir el filtro.
        fuentes = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = ruta: tipo:
            !(pkgs.lib.hasPrefix (toString ./web) ruta)
            && (
              (builtins.match ".*\\.(html|css|js|toml|json|png|jpg|md)$" ruta != null)
              || (builtins.match ".*/(recursos|pruebas)(/.*)?$" ruta != null)
              || (craneLib.filterCargoSources ruta tipo)
            );
          name = "fuentes";
        };

        # El cliente Svelte, compilado sin red: las dependencias vienen fijadas por
        # package-lock.json y npmDepsHash. Cambiar dependencias = actualizar el hash con
        #   nix run nixpkgs#prefetch-npm-deps -- web/package-lock.json
        web = pkgs.buildNpmPackage {
          pname = "galatea-web";
          version = "0.1.0";
          src = pkgs.lib.cleanSourceWith {
            src = ./web;
            filter = ruta: tipo:
              !(tipo == "directory" && builtins.elem (baseNameOf ruta) [ "node_modules" "android" "dist" ]);
            name = "web";
          };
          npmDepsHash = "sha256-gfB8Nuo8+ylqaB/hW+5a+ywEoJwJCgU4O96QXYnVQ7Y=";
          nodejs = pkgs.nodejs_22;
          npmBuildScript = "build";
          doCheck = true;
          checkPhase = "npm test";
          installPhase = "cp -r dist $out";
        };

        argosComunes = {
          src = fuentes;
          strictDeps = true;
          pname = "galatea";
          version = "0.1.0";
          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = [ ];
        };

        artefactos = craneLib.buildDepsOnly argosComunes;

        galatea = craneLib.buildPackage (argosComunes // {
          cargoArtifacts = artefactos;
          doCheck = false;
        });

        # Capacitor 8: AGP 8.13, compileSdk 36, JDK 21, Gradle 8.14.
        # AGP 8.13 compila con build-tools 35.0.0 (su valor por defecto, también en los
        # módulos que regenera `cap sync`); 36.0.0 aporta el aapt2 que usa GRADLE_OPTS.
        versionBuildTools = "36.0.0";
        android = pkgs.androidenv.composeAndroidPackages {
          platformVersions = [ "36" ];
          buildToolsVersions = [ "35.0.0" versionBuildTools ];
          includeEmulator = true;
          includeSystemImages = true;
          systemImageTypes = [ "google_apis" ];
          abiVersions = [ "x86_64" ];
        };
        sdkAndroid = android.androidsdk;
        casaAndroid = "${sdkAndroid}/libexec/android-sdk";
      in
      {
        packages.default = galatea;
        packages.galatea = galatea;
        packages.web = web;
        apps.default = flake-utils.lib.mkApp { drv = galatea; };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ galatea ];
          packages = with pkgs; [
            cadenaRust
            rust-analyzer
            (python3.withPackages (ps: [ ps.pillow ]))
            dejavu_fonts
            nodejs_22
            playwright-test
            curl
            jq
            openssl # scripts/apk.sh muestra el sujeto y la vigencia de la CA
            sdkAndroid
            jdk21
            gradle_8
          ];
          RUST_LOG = "info,galatea=debug";
          ANDROID_HOME = casaAndroid;
          ANDROID_SDK_ROOT = casaAndroid;
          JAVA_HOME = "${pkgs.jdk21}";
          # El aapt2 que baja Gradle no corre en NixOS: se usa el del SDK de nixpkgs.
          GRADLE_OPTS = "-Dorg.gradle.project.android.aapt2FromMavenOverride=${casaAndroid}/build-tools/${versionBuildTools}/aapt2";
          PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers}";
          PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS = "true";
          # El resolvedor ESM de Node ignora NODE_PATH: `import "@playwright/test"`
          # necesita un node_modules real que apunte al paquete de nixpkgs.
          shellHook = ''
            mkdir -p node_modules
            ln -sfn ${pkgs.playwright-test}/lib/node_modules/@playwright node_modules/@playwright
            ln -sfn ${pkgs.playwright-test}/lib/node_modules/playwright-core node_modules/playwright-core
          '';
        };

        checks = {
          inherit galatea web;
          clippy = craneLib.cargoClippy (argosComunes // {
            cargoArtifacts = artefactos;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });
          formato = craneLib.cargoFmt { inherit (argosComunes) pname version; src = fuentes; };
          pruebas = craneLib.cargoTest (argosComunes // { cargoArtifacts = artefactos; });
        };
      }
    );
}
