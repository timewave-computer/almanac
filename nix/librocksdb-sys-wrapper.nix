{ lib, stdenv, rocksdb, zlib, bzip2, lz4, zstd, snappy, pkg-config }:

stdenv.mkDerivation rec {
  pname = "librocksdb-sys-wrapper";
  version = "0.11.0+8.1.1";
  
  src = builtins.fetchTarball {
    url = "https://crates.io/api/v1/crates/librocksdb-sys/${version}/download";
    sha256 = "0000000000000000000000000000000000000000000000000000000000000000"; # Will be filled automatically
  };
  
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ rocksdb zlib bzip2 lz4 zstd snappy ];
  
  # Override the build script to just link to system rocksdb
  postPatch = ''
    cat > build.rs << 'EOF'
fn main() {
    println!("cargo:rustc-link-lib=dylib=rocksdb");
    println!("cargo:rustc-link-search=native=${rocksdb}/lib");
    println!("cargo:include=${rocksdb}/include");
    
    // Link compression libraries
    println!("cargo:rustc-link-lib=z");
    println!("cargo:rustc-link-lib=bz2");
    println!("cargo:rustc-link-lib=lz4");
    println!("cargo:rustc-link-lib=zstd");
    println!("cargo:rustc-link-lib=snappy");
}
EOF
  '';
  
  buildPhase = ''
    # This is handled by cargo, we just need the source with our custom build.rs
  '';
  
  installPhase = ''
    mkdir -p $out
    cp -r . $out/
  '';
} 