# RocksDB derivation for Almanac project
# This creates a custom rocksdb build optimized for our use case
{ pkgs, lib, stdenv }:

let
  darwinDeploymentTarget = "11.0";
in
stdenv.mkDerivation rec {
  pname = "rocksdb-almanac";
  version = "8.10.0";

  src = pkgs.fetchFromGitHub {
    owner = "facebook";
    repo = "rocksdb";
    rev = "v${version}";
    sha256 = "sha256-KGsYDBc1fz/90YYNGwlZ0LUKXYsP1zyhP29TnRQwgjQ=";
  };

  nativeBuildInputs = with pkgs; [
    cmake
    pkg-config
  ];

  buildInputs = with pkgs; [
    zlib
    bzip2
    lz4
    zstd
    snappy
  ] ++ lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.Security
    darwin.apple_sdk.frameworks.SystemConfiguration
  ];

  cmakeFlags = [
    "-DWITH_ZLIB=ON"
    "-DWITH_BZ2=ON"
    "-DWITH_LZ4=ON"
    "-DWITH_ZSTD=ON"
    "-DWITH_SNAPPY=ON"
    "-DUSE_RTTI=ON"
    "-DROCKSDB_BUILD_SHARED=ON"
    "-DWITH_GFLAGS=OFF"
    "-DWITH_TESTS=OFF"
    "-DWITH_BENCHMARK_TOOLS=OFF"
    "-DWITH_TOOLS=OFF"
    "-DFAIL_ON_WARNINGS=OFF"
  ] ++ lib.optionals stdenv.isDarwin [
    "-DCMAKE_OSX_DEPLOYMENT_TARGET=${darwinDeploymentTarget}"
  ];

  # Generate pkg-config file for easier linking
  postInstall = ''
    mkdir -p $out/lib/pkgconfig
    cat > $out/lib/pkgconfig/rocksdb.pc << EOF
prefix=$out
exec_prefix=\''${prefix}
libdir=\''${exec_prefix}/lib
includedir=\''${prefix}/include

Name: rocksdb
Description: A library that provides an embeddable, persistent key-value store for fast storage
Version: ${version}
Libs: -L\''${libdir} -lrocksdb
Libs.private: -lz -lbz2 -llz4 -lzstd -lsnappy
Cflags: -I\''${includedir}
EOF
  '';

  meta = with lib; {
    description = "RocksDB build optimized for Almanac project";
    homepage = "https://rocksdb.org/";
    license = licenses.gpl2;
    platforms = platforms.unix;
  };
} 