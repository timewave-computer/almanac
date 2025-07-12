fn main() {
    #[cfg(feature = "rocks")]
    {
        // Link against our custom RocksDB library
        println!("cargo:rustc-link-lib=rocksdb");
        
        // Add library search paths for nix environment
        if let Ok(out_dir) = std::env::var("ROCKSDB_LIB_DIR") {
            if !out_dir.is_empty() {
                println!("cargo:rustc-link-search=native={}", out_dir);
            }
        }
        
        // For nix development environment
        if let Ok(nix_store) = std::env::var("NIX_STORE") {
            if !nix_store.is_empty() {
                println!("cargo:rustc-link-search=native={}", nix_store);
            }
        }
        
        // Common library search paths on Unix systems
        println!("cargo:rustc-link-search=native=/usr/local/lib");
        println!("cargo:rustc-link-search=native=/usr/lib");
        println!("cargo:rustc-link-search=native=/opt/local/lib");
        
        // Tell cargo to rerun if environment variables change
        println!("cargo:rerun-if-env-changed=ROCKSDB_LIB_DIR");
        println!("cargo:rerun-if-env-changed=NIX_STORE");
        
        // Rerun if the FFI wrapper changes
        println!("cargo:rerun-if-changed=../../rocksdb_ffi/lib.rs");
    }
} 