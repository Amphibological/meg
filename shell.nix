{ pkgs ? import <nixos> {} }:
  pkgs.mkShell {
    buildInputs = [
      pkgs.llvm_10
      pkgs.libxml2
      pkgs.valgrind
      pkgs.clang
    ];
}
