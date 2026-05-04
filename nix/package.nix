{
  version,
  lib,
  installShellFiles,
  rustPlatform,
  buildFeatures ? [ ],
}:

rustPlatform.buildRustPackage {
  pname = "pm2kg";

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../src
      ../crates
      ../Cargo.lock
      ../Cargo.toml
    ];
  };

  inherit buildFeatures;
  inherit version;

  # inject version from nix into the build
  env.NIX_RELEASE_VERSION = version;

  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [
    installShellFiles

    rustPlatform.bindgenHook
  ];

  buildInputs = [ ];

  meta = with lib; {
    description = "CLI tool that converts the PubMed and MeSH dataset into a CSV-based Knowledge Graph representation (Neo4J)";
    mainProgram = "pm2kg";
    homepage = "https://github.com/c2fc2f/PubMed-MeSH-to-KG";
    license = licenses.mit;
    maintainers = [ maintainers.c2fc2f ];
  };
}
