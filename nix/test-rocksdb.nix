let
  pkgs = import <nixpkgs> {};
in
  pkgs.callPackage ./rocksdb.nix {} 