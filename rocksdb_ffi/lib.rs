// RocksDB FFI bindings
// 
// This provides a direct FFI interface to RocksDB without any external dependencies.
// We'll use std::ffi instead of the libc crate to avoid network dependencies.
//
// Enhanced with column families, write batches, iterators, and advanced features.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_uchar, c_void};
use std::ptr;
use std::slice;
use std::collections::HashMap;

// Type aliases for C types
pub type SizeT = usize;

// Opaque types
#[repr(C)]
pub struct rocksdb_t {
    _unused: [u8; 0],
}

#[repr(C)]
pub struct rocksdb_options_t {
    _unused: [u8; 0],
}

#[repr(C)]
pub struct rocksdb_writeoptions_t {
    _unused: [u8; 0],
}

#[repr(C)]
pub struct rocksdb_readoptions_t {
    _unused: [u8; 0],
}

// Column Family types
#[repr(C)]
pub struct rocksdb_column_family_handle_t {
    _unused: [u8; 0],
}

#[repr(C)]
pub struct rocksdb_column_family_options_t {
    _unused: [u8; 0],
}

// Write Batch types
#[repr(C)]
pub struct rocksdb_writebatch_t {
    _unused: [u8; 0],
}

// Iterator types
#[repr(C)]
pub struct rocksdb_iterator_t {
    _unused: [u8; 0],
}

// Snapshot types
#[repr(C)]
pub struct rocksdb_snapshot_t {
    _unused: [u8; 0],
}

// Backup types
#[repr(C)]
pub struct rocksdb_backup_engine_t {
    _unused: [u8; 0],
}

// External C functions from RocksDB
#[link(name = "rocksdb")]
extern "C" {
    // Database operations
    pub fn rocksdb_open(
        options: *const rocksdb_options_t,
        name: *const c_char,
        errptr: *mut *mut c_char,
    ) -> *mut rocksdb_t;
    
    pub fn rocksdb_open_column_families(
        options: *const rocksdb_options_t,
        name: *const c_char,
        num_column_families: c_int,
        column_family_names: *const *const c_char,
        column_family_options: *const *const rocksdb_column_family_options_t,
        column_family_handles: *mut *mut rocksdb_column_family_handle_t,
        errptr: *mut *mut c_char,
    ) -> *mut rocksdb_t;
    
    pub fn rocksdb_create_column_family(
        db: *mut rocksdb_t,
        column_family_options: *const rocksdb_column_family_options_t,
        column_family_name: *const c_char,
        errptr: *mut *mut c_char,
    ) -> *mut rocksdb_column_family_handle_t;
    
    pub fn rocksdb_drop_column_family(
        db: *mut rocksdb_t,
        handle: *mut rocksdb_column_family_handle_t,
        errptr: *mut *mut c_char,
    );
    
    pub fn rocksdb_column_family_handle_destroy(handle: *mut rocksdb_column_family_handle_t);
    
    pub fn rocksdb_close(db: *mut rocksdb_t);
    
    // Options
    pub fn rocksdb_options_create() -> *mut rocksdb_options_t;
    pub fn rocksdb_options_destroy(options: *mut rocksdb_options_t);
    pub fn rocksdb_options_set_create_if_missing(options: *mut rocksdb_options_t, val: c_uchar);
    pub fn rocksdb_options_set_create_missing_column_families(options: *mut rocksdb_options_t, val: c_uchar);
    pub fn rocksdb_options_set_compression(options: *mut rocksdb_options_t, val: c_int);
    pub fn rocksdb_options_set_write_buffer_size(options: *mut rocksdb_options_t, val: SizeT);
    pub fn rocksdb_options_set_max_write_buffer_number(options: *mut rocksdb_options_t, val: c_int);
    pub fn rocksdb_options_set_block_size(options: *mut rocksdb_options_t, val: SizeT);
    pub fn rocksdb_options_set_max_open_files(options: *mut rocksdb_options_t, val: c_int);
    pub fn rocksdb_options_set_use_fsync(options: *mut rocksdb_options_t, val: c_int);
    pub fn rocksdb_options_set_level0_file_num_compaction_trigger(options: *mut rocksdb_options_t, val: c_int);
    pub fn rocksdb_options_set_level0_slowdown_writes_trigger(options: *mut rocksdb_options_t, val: c_int);
    pub fn rocksdb_options_set_level0_stop_writes_trigger(options: *mut rocksdb_options_t, val: c_int);
    pub fn rocksdb_options_set_target_file_size_base(options: *mut rocksdb_options_t, val: u64);
    pub fn rocksdb_options_set_target_file_size_multiplier(options: *mut rocksdb_options_t, val: c_int);
    pub fn rocksdb_options_set_max_bytes_for_level_base(options: *mut rocksdb_options_t, val: u64);
    pub fn rocksdb_options_set_max_bytes_for_level_multiplier(options: *mut rocksdb_options_t, val: f64);
    
    // Column Family Options
    pub fn rocksdb_column_family_options_create() -> *mut rocksdb_column_family_options_t;
    pub fn rocksdb_column_family_options_destroy(options: *mut rocksdb_column_family_options_t);
    pub fn rocksdb_column_family_options_set_compression(options: *mut rocksdb_column_family_options_t, val: c_int);
    pub fn rocksdb_column_family_options_set_write_buffer_size(options: *mut rocksdb_column_family_options_t, val: SizeT);
    pub fn rocksdb_column_family_options_set_max_write_buffer_number(options: *mut rocksdb_column_family_options_t, val: c_int);
    pub fn rocksdb_column_family_options_set_block_size(options: *mut rocksdb_column_family_options_t, val: SizeT);
    
    // Write and Read Options
    pub fn rocksdb_writeoptions_create() -> *mut rocksdb_writeoptions_t;
    pub fn rocksdb_writeoptions_destroy(options: *mut rocksdb_writeoptions_t);
    pub fn rocksdb_writeoptions_set_sync(options: *mut rocksdb_writeoptions_t, val: c_uchar);
    pub fn rocksdb_writeoptions_disable_wal(options: *mut rocksdb_writeoptions_t, val: c_int);
    
    pub fn rocksdb_readoptions_create() -> *mut rocksdb_readoptions_t;
    pub fn rocksdb_readoptions_destroy(options: *mut rocksdb_readoptions_t);
    pub fn rocksdb_readoptions_set_verify_checksums(options: *mut rocksdb_readoptions_t, val: c_uchar);
    pub fn rocksdb_readoptions_set_fill_cache(options: *mut rocksdb_readoptions_t, val: c_uchar);
    pub fn rocksdb_readoptions_set_snapshot(options: *mut rocksdb_readoptions_t, snapshot: *const rocksdb_snapshot_t);
    
    // Read/Write operations
    pub fn rocksdb_get(
        db: *mut rocksdb_t,
        readoptions: *const rocksdb_readoptions_t,
        key: *const c_char,
        keylen: SizeT,
        vallen: *mut SizeT,
        errptr: *mut *mut c_char,
    ) -> *mut c_char;
    
    pub fn rocksdb_get_cf(
        db: *mut rocksdb_t,
        readoptions: *const rocksdb_readoptions_t,
        column_family: *mut rocksdb_column_family_handle_t,
        key: *const c_char,
        keylen: SizeT,
        vallen: *mut SizeT,
        errptr: *mut *mut c_char,
    ) -> *mut c_char;
    
    pub fn rocksdb_put(
        db: *mut rocksdb_t,
        writeoptions: *const rocksdb_writeoptions_t,
        key: *const c_char,
        keylen: SizeT,
        val: *const c_char,
        vallen: SizeT,
        errptr: *mut *mut c_char,
    );
    
    pub fn rocksdb_put_cf(
        db: *mut rocksdb_t,
        writeoptions: *const rocksdb_writeoptions_t,
        column_family: *mut rocksdb_column_family_handle_t,
        key: *const c_char,
        keylen: SizeT,
        val: *const c_char,
        vallen: SizeT,
        errptr: *mut *mut c_char,
    );
    
    pub fn rocksdb_delete(
        db: *mut rocksdb_t,
        writeoptions: *const rocksdb_writeoptions_t,
        key: *const c_char,
        keylen: SizeT,
        errptr: *mut *mut c_char,
    );
    
    pub fn rocksdb_delete_cf(
        db: *mut rocksdb_t,
        writeoptions: *const rocksdb_writeoptions_t,
        column_family: *mut rocksdb_column_family_handle_t,
        key: *const c_char,
        keylen: SizeT,
        errptr: *mut *mut c_char,
    );
    
    // Write Batch operations
    pub fn rocksdb_writebatch_create() -> *mut rocksdb_writebatch_t;
    pub fn rocksdb_writebatch_destroy(batch: *mut rocksdb_writebatch_t);
    pub fn rocksdb_writebatch_clear(batch: *mut rocksdb_writebatch_t);
    pub fn rocksdb_writebatch_count(batch: *mut rocksdb_writebatch_t) -> c_int;
    
    pub fn rocksdb_writebatch_put(
        batch: *mut rocksdb_writebatch_t,
        key: *const c_char,
        keylen: SizeT,
        val: *const c_char,
        vallen: SizeT,
    );
    
    pub fn rocksdb_writebatch_put_cf(
        batch: *mut rocksdb_writebatch_t,
        column_family: *mut rocksdb_column_family_handle_t,
        key: *const c_char,
        keylen: SizeT,
        val: *const c_char,
        vallen: SizeT,
    );
    
    pub fn rocksdb_writebatch_delete(
        batch: *mut rocksdb_writebatch_t,
        key: *const c_char,
        keylen: SizeT,
    );
    
    pub fn rocksdb_writebatch_delete_cf(
        batch: *mut rocksdb_writebatch_t,
        column_family: *mut rocksdb_column_family_handle_t,
        key: *const c_char,
        keylen: SizeT,
    );
    
    pub fn rocksdb_write(
        db: *mut rocksdb_t,
        writeoptions: *const rocksdb_writeoptions_t,
        batch: *mut rocksdb_writebatch_t,
        errptr: *mut *mut c_char,
    );
    
    // Iterator operations
    pub fn rocksdb_create_iterator(
        db: *mut rocksdb_t,
        readoptions: *const rocksdb_readoptions_t,
    ) -> *mut rocksdb_iterator_t;
    
    pub fn rocksdb_create_iterator_cf(
        db: *mut rocksdb_t,
        readoptions: *const rocksdb_readoptions_t,
        column_family: *mut rocksdb_column_family_handle_t,
    ) -> *mut rocksdb_iterator_t;
    
    pub fn rocksdb_iter_destroy(iter: *mut rocksdb_iterator_t);
    pub fn rocksdb_iter_valid(iter: *const rocksdb_iterator_t) -> c_uchar;
    pub fn rocksdb_iter_seek_to_first(iter: *mut rocksdb_iterator_t);
    pub fn rocksdb_iter_seek_to_last(iter: *mut rocksdb_iterator_t);
    pub fn rocksdb_iter_seek(iter: *mut rocksdb_iterator_t, key: *const c_char, keylen: SizeT);
    pub fn rocksdb_iter_seek_for_prev(iter: *mut rocksdb_iterator_t, key: *const c_char, keylen: SizeT);
    pub fn rocksdb_iter_next(iter: *mut rocksdb_iterator_t);
    pub fn rocksdb_iter_prev(iter: *mut rocksdb_iterator_t);
    
    pub fn rocksdb_iter_key(iter: *const rocksdb_iterator_t, keylen: *mut SizeT) -> *const c_char;
    pub fn rocksdb_iter_value(iter: *const rocksdb_iterator_t, vallen: *mut SizeT) -> *const c_char;
    
    pub fn rocksdb_iter_get_error(iter: *const rocksdb_iterator_t, errptr: *mut *mut c_char);
    
    // Snapshot operations
    pub fn rocksdb_create_snapshot(db: *mut rocksdb_t) -> *const rocksdb_snapshot_t;
    pub fn rocksdb_release_snapshot(db: *mut rocksdb_t, snapshot: *const rocksdb_snapshot_t);
    
    // Backup operations
    pub fn rocksdb_backup_engine_open(
        options: *const rocksdb_options_t,
        path: *const c_char,
        errptr: *mut *mut c_char,
    ) -> *mut rocksdb_backup_engine_t;
    
    pub fn rocksdb_backup_engine_create_new_backup(
        backup_engine: *mut rocksdb_backup_engine_t,
        db: *mut rocksdb_t,
        errptr: *mut *mut c_char,
    );
    
    pub fn rocksdb_backup_engine_close(backup_engine: *mut rocksdb_backup_engine_t);
    
    // Memory management
    pub fn rocksdb_free(ptr: *mut c_void);
    
    // List column families
    pub fn rocksdb_list_column_families(
        options: *const rocksdb_options_t,
        name: *const c_char,
        lencf: *mut SizeT,
        errptr: *mut *mut c_char,
    ) -> *mut *mut c_char;
    
    pub fn rocksdb_list_column_families_destroy(list: *mut *mut c_char, len: SizeT);
    
    // Compaction
    pub fn rocksdb_compact_range(
        db: *mut rocksdb_t,
        start_key: *const c_char,
        start_key_len: SizeT,
        limit_key: *const c_char,
        limit_key_len: SizeT,
    );
    
    pub fn rocksdb_compact_range_cf(
        db: *mut rocksdb_t,
        column_family: *mut rocksdb_column_family_handle_t,
        start_key: *const c_char,
        start_key_len: SizeT,
        limit_key: *const c_char,
        limit_key_len: SizeT,
    );
    
    // Flush
    pub fn rocksdb_flush(
        db: *mut rocksdb_t,
        flushoptions: *const rocksdb_options_t,
        errptr: *mut *mut c_char,
    );
    
    pub fn rocksdb_flush_cf(
        db: *mut rocksdb_t,
        flushoptions: *const rocksdb_options_t,
        column_family: *mut rocksdb_column_family_handle_t,
        errptr: *mut *mut c_char,
    );
}

// Compression types
#[derive(Debug, Clone, Copy)]
pub enum CompressionType {
    None = 0,
    Snappy = 1,
    Zlib = 2,
    Bz2 = 3,
    Lz4 = 4,
    Lz4hc = 5,
    Xpress = 6,
    Zstd = 7,
}

// Configuration for column families
#[derive(Debug, Clone)]
pub struct ColumnFamilyConfig {
    pub name: String,
    pub compression: CompressionType,
    pub write_buffer_size: usize,
    pub max_write_buffer_number: i32,
    pub block_size: usize,
}

impl Default for ColumnFamilyConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            compression: CompressionType::Snappy,
            write_buffer_size: 64 * 1024 * 1024, // 64MB
            max_write_buffer_number: 3,
            block_size: 4 * 1024, // 4KB
        }
    }
}

// Write batch operations
pub struct WriteBatch {
    inner: *mut rocksdb_writebatch_t,
}

impl WriteBatch {
    pub fn new() -> Self {
        let inner = unsafe { rocksdb_writebatch_create() };
        Self { inner }
    }
    
    pub fn put(&mut self, key: &[u8], value: &[u8]) {
        unsafe {
            rocksdb_writebatch_put(
                self.inner,
                key.as_ptr() as *const c_char,
                key.len(),
                value.as_ptr() as *const c_char,
                value.len(),
            );
        }
    }
    
    pub fn put_cf(&mut self, cf: &ColumnFamilyHandle, key: &[u8], value: &[u8]) {
        unsafe {
            rocksdb_writebatch_put_cf(
                self.inner,
                cf.inner,
                key.as_ptr() as *const c_char,
                key.len(),
                value.as_ptr() as *const c_char,
                value.len(),
            );
        }
    }
    
    pub fn delete(&mut self, key: &[u8]) {
        unsafe {
            rocksdb_writebatch_delete(
                self.inner,
                key.as_ptr() as *const c_char,
                key.len(),
            );
        }
    }
    
    pub fn delete_cf(&mut self, cf: &ColumnFamilyHandle, key: &[u8]) {
        unsafe {
            rocksdb_writebatch_delete_cf(
                self.inner,
                cf.inner,
                key.as_ptr() as *const c_char,
                key.len(),
            );
        }
    }
    
    pub fn clear(&mut self) {
        unsafe {
            rocksdb_writebatch_clear(self.inner);
        }
    }
    
    pub fn count(&self) -> i32 {
        unsafe { rocksdb_writebatch_count(self.inner) }
    }
}

impl Drop for WriteBatch {
    fn drop(&mut self) {
        unsafe {
            rocksdb_writebatch_destroy(self.inner);
        }
    }
}

// Column family handle
pub struct ColumnFamilyHandle {
    inner: *mut rocksdb_column_family_handle_t,
}

impl ColumnFamilyHandle {
    pub fn new(inner: *mut rocksdb_column_family_handle_t) -> Self {
        Self { inner }
    }
}

impl Drop for ColumnFamilyHandle {
    fn drop(&mut self) {
        unsafe {
            rocksdb_column_family_handle_destroy(self.inner);
        }
    }
}

// Iterator for range queries
pub struct DBIterator {
    inner: *mut rocksdb_iterator_t,
}

impl DBIterator {
    pub fn new(inner: *mut rocksdb_iterator_t) -> Self {
        Self { inner }
    }
    
    pub fn seek_to_first(&mut self) {
        unsafe { rocksdb_iter_seek_to_first(self.inner) }
    }
    
    pub fn seek_to_last(&mut self) {
        unsafe { rocksdb_iter_seek_to_last(self.inner) }
    }
    
    pub fn seek(&mut self, key: &[u8]) {
        unsafe {
            rocksdb_iter_seek(self.inner, key.as_ptr() as *const c_char, key.len());
        }
    }
    
    pub fn seek_for_prev(&mut self, key: &[u8]) {
        unsafe {
            rocksdb_iter_seek_for_prev(self.inner, key.as_ptr() as *const c_char, key.len());
        }
    }
    
    pub fn next(&mut self) {
        unsafe { rocksdb_iter_next(self.inner) }
    }
    
    pub fn prev(&mut self) {
        unsafe { rocksdb_iter_prev(self.inner) }
    }
    
    pub fn valid(&self) -> bool {
        unsafe { rocksdb_iter_valid(self.inner) != 0 }
    }
    
    pub fn key(&self) -> Option<Vec<u8>> {
        if !self.valid() {
            return None;
        }
        
        let mut keylen: SizeT = 0;
        let key_ptr = unsafe { rocksdb_iter_key(self.inner, &mut keylen) };
        
        if key_ptr.is_null() {
            return None;
        }
        
        let key_slice = unsafe { slice::from_raw_parts(key_ptr as *const u8, keylen) };
        Some(key_slice.to_vec())
    }
    
    pub fn value(&self) -> Option<Vec<u8>> {
        if !self.valid() {
            return None;
        }
        
        let mut vallen: SizeT = 0;
        let val_ptr = unsafe { rocksdb_iter_value(self.inner, &mut vallen) };
        
        if val_ptr.is_null() {
            return None;
        }
        
        let val_slice = unsafe { slice::from_raw_parts(val_ptr as *const u8, vallen) };
        Some(val_slice.to_vec())
    }
    
    pub fn key_value(&self) -> Option<(Vec<u8>, Vec<u8>)> {
        if let (Some(key), Some(value)) = (self.key(), self.value()) {
            Some((key, value))
        } else {
            None
        }
    }
    
    pub fn check_error(&self) -> Result<(), String> {
        let mut err: *mut c_char = ptr::null_mut();
        unsafe {
            rocksdb_iter_get_error(self.inner, &mut err);
        }
        
        if !err.is_null() {
            let err_str = unsafe {
                let err_cstr = CStr::from_ptr(err);
                let result = err_cstr.to_string_lossy().into_owned();
                rocksdb_free(err as *mut c_void);
                result
            };
            return Err(err_str);
        }
        
        Ok(())
    }
}

impl Drop for DBIterator {
    fn drop(&mut self) {
        unsafe {
            rocksdb_iter_destroy(self.inner);
        }
    }
}

// Enhanced DB with column family support
pub struct DB {
    inner: *mut rocksdb_t,
    column_families: HashMap<String, ColumnFamilyHandle>,
    default_cf: Option<ColumnFamilyHandle>,
}

impl DB {
    pub fn open(path: &str) -> Result<Self, String> {
        let options = unsafe { rocksdb_options_create() };
        unsafe {
            rocksdb_options_set_create_if_missing(options, 1);
            rocksdb_options_set_create_missing_column_families(options, 1);
        }
        
        let path_cstr = CString::new(path).unwrap();
        let mut err: *mut c_char = ptr::null_mut();
        
        let db = unsafe {
            rocksdb_open(options, path_cstr.as_ptr(), &mut err)
        };
        
        unsafe { rocksdb_options_destroy(options) };
        
        if !err.is_null() {
            let err_str = unsafe {
                let err_cstr = CStr::from_ptr(err);
                let result = err_cstr.to_string_lossy().into_owned();
                rocksdb_free(err as *mut c_void);
                result
            };
            return Err(err_str);
        }
        
        Ok(DB { 
            inner: db,
            column_families: HashMap::new(),
            default_cf: None,
        })
    }
    
    pub fn open_with_column_families(path: &str, column_families: Vec<ColumnFamilyConfig>) -> Result<Self, String> {
        let options = unsafe { rocksdb_options_create() };
        unsafe {
            rocksdb_options_set_create_if_missing(options, 1);
            rocksdb_options_set_create_missing_column_families(options, 1);
        }
        
        let path_cstr = CString::new(path).unwrap();
        
        // Prepare column family names and options
        let mut cf_names: Vec<CString> = Vec::new();
        let mut cf_name_ptrs: Vec<*const c_char> = Vec::new();
        let mut cf_options: Vec<*mut rocksdb_column_family_options_t> = Vec::new();
        let mut cf_handles: Vec<*mut rocksdb_column_family_handle_t> = Vec::new();
        
        for cf_config in &column_families {
            let cf_name = CString::new(cf_config.name.clone()).unwrap();
            cf_name_ptrs.push(cf_name.as_ptr());
            cf_names.push(cf_name);
            
            let cf_opts = unsafe { rocksdb_column_family_options_create() };
            unsafe {
                rocksdb_column_family_options_set_compression(cf_opts, cf_config.compression as c_int);
                rocksdb_column_family_options_set_write_buffer_size(cf_opts, cf_config.write_buffer_size);
                rocksdb_column_family_options_set_max_write_buffer_number(cf_opts, cf_config.max_write_buffer_number);
                rocksdb_column_family_options_set_block_size(cf_opts, cf_config.block_size);
            }
            cf_options.push(cf_opts);
            cf_handles.push(ptr::null_mut());
        }
        
        let mut err: *mut c_char = ptr::null_mut();
        let db = unsafe {
            rocksdb_open_column_families(
                options,
                path_cstr.as_ptr(),
                column_families.len() as c_int,
                cf_name_ptrs.as_ptr(),
                cf_options.as_ptr() as *const *const rocksdb_column_family_options_t,
                cf_handles.as_mut_ptr(),
                &mut err,
            )
        };
        
        // Cleanup options
        unsafe { rocksdb_options_destroy(options) };
        for cf_opts in cf_options {
            unsafe { rocksdb_column_family_options_destroy(cf_opts) };
        }
        
        if !err.is_null() {
            let err_str = unsafe {
                let err_cstr = CStr::from_ptr(err);
                let result = err_cstr.to_string_lossy().into_owned();
                rocksdb_free(err as *mut c_void);
                result
            };
            return Err(err_str);
        }
        
        // Create column family handles map
        let mut cf_map = HashMap::new();
        let mut default_cf = None;
        
        for (i, cf_config) in column_families.iter().enumerate() {
            let handle = ColumnFamilyHandle::new(cf_handles[i]);
            
            if cf_config.name == "default" {
                default_cf = Some(ColumnFamilyHandle::new(cf_handles[i]));
            }
            
            cf_map.insert(cf_config.name.clone(), handle);
        }
        
        Ok(DB { 
            inner: db,
            column_families: cf_map,
            default_cf,
        })
    }
    
    pub fn create_column_family(&mut self, config: ColumnFamilyConfig) -> Result<(), String> {
        let cf_opts = unsafe { rocksdb_column_family_options_create() };
        unsafe {
            rocksdb_column_family_options_set_compression(cf_opts, config.compression as c_int);
            rocksdb_column_family_options_set_write_buffer_size(cf_opts, config.write_buffer_size);
            rocksdb_column_family_options_set_max_write_buffer_number(cf_opts, config.max_write_buffer_number);
            rocksdb_column_family_options_set_block_size(cf_opts, config.block_size);
        }
        
        let name_cstr = CString::new(config.name.clone()).unwrap();
        let mut err: *mut c_char = ptr::null_mut();
        
        let handle = unsafe {
            rocksdb_create_column_family(
                self.inner,
                cf_opts,
                name_cstr.as_ptr(),
                &mut err,
            )
        };
        
        unsafe { rocksdb_column_family_options_destroy(cf_opts) };
        
        if !err.is_null() {
            let err_str = unsafe {
                let err_cstr = CStr::from_ptr(err);
                let result = err_cstr.to_string_lossy().into_owned();
                rocksdb_free(err as *mut c_void);
                result
            };
            return Err(err_str);
        }
        
        self.column_families.insert(config.name.clone(), ColumnFamilyHandle::new(handle));
        Ok(())
    }
    
    pub fn drop_column_family(&mut self, name: &str) -> Result<(), String> {
        if let Some(handle) = self.column_families.remove(name) {
            let mut err: *mut c_char = ptr::null_mut();
            unsafe {
                rocksdb_drop_column_family(self.inner, handle.inner, &mut err);
            }
            
            if !err.is_null() {
                let err_str = unsafe {
                    let err_cstr = CStr::from_ptr(err);
                    let result = err_cstr.to_string_lossy().into_owned();
                    rocksdb_free(err as *mut c_void);
                    result
                };
                return Err(err_str);
            }
        }
        Ok(())
    }
    
    pub fn get_column_family(&self, name: &str) -> Option<&ColumnFamilyHandle> {
        self.column_families.get(name)
    }
    
    pub fn list_column_families(&self) -> Vec<String> {
        self.column_families.keys().cloned().collect()
    }
    
    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<(), String> {
        let mut err: *mut c_char = ptr::null_mut();
        
        unsafe {
            rocksdb_put(
                self.inner,
                ptr::null(), // write options
                key.as_ptr() as *const c_char,
                key.len(),
                value.as_ptr() as *const c_char,
                value.len(),
                &mut err,
            );
        }
        
        if !err.is_null() {
            let err_str = unsafe {
                let err_cstr = CStr::from_ptr(err);
                let result = err_cstr.to_string_lossy().into_owned();
                rocksdb_free(err as *mut c_void);
                result
            };
            return Err(err_str);
        }
        
        Ok(())
    }
    
    pub fn put_cf(&self, cf: &ColumnFamilyHandle, key: &[u8], value: &[u8]) -> Result<(), String> {
        let mut err: *mut c_char = ptr::null_mut();
        
        unsafe {
            rocksdb_put_cf(
                self.inner,
                ptr::null(), // write options
                cf.inner,
                key.as_ptr() as *const c_char,
                key.len(),
                value.as_ptr() as *const c_char,
                value.len(),
                &mut err,
            );
        }
        
        if !err.is_null() {
            let err_str = unsafe {
                let err_cstr = CStr::from_ptr(err);
                let result = err_cstr.to_string_lossy().into_owned();
                rocksdb_free(err as *mut c_void);
                result
            };
            return Err(err_str);
        }
        
        Ok(())
    }
    
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, String> {
        let mut err: *mut c_char = ptr::null_mut();
        let mut vallen: SizeT = 0;
        
        let value_ptr = unsafe {
            rocksdb_get(
                self.inner,
                ptr::null(), // read options
                key.as_ptr() as *const c_char,
                key.len(),
                &mut vallen,
                &mut err,
            )
        };
        
        if !err.is_null() {
            let err_str = unsafe {
                let err_cstr = CStr::from_ptr(err);
                let result = err_cstr.to_string_lossy().into_owned();
                rocksdb_free(err as *mut c_void);
                result
            };
            return Err(err_str);
        }
        
        if value_ptr.is_null() {
            return Ok(None);
        }
        
        let value = unsafe {
            let slice = std::slice::from_raw_parts(value_ptr as *const u8, vallen);
            let result = slice.to_vec();
            rocksdb_free(value_ptr as *mut c_void);
            result
        };
        
        Ok(Some(value))
    }
    
    pub fn get_cf(&self, cf: &ColumnFamilyHandle, key: &[u8]) -> Result<Option<Vec<u8>>, String> {
        let mut err: *mut c_char = ptr::null_mut();
        let mut vallen: SizeT = 0;
        
        let value_ptr = unsafe {
            rocksdb_get_cf(
                self.inner,
                ptr::null(), // read options
                cf.inner,
                key.as_ptr() as *const c_char,
                key.len(),
                &mut vallen,
                &mut err,
            )
        };
        
        if !err.is_null() {
            let err_str = unsafe {
                let err_cstr = CStr::from_ptr(err);
                let result = err_cstr.to_string_lossy().into_owned();
                rocksdb_free(err as *mut c_void);
                result
            };
            return Err(err_str);
        }
        
        if value_ptr.is_null() {
            return Ok(None);
        }
        
        let value = unsafe {
            let slice = std::slice::from_raw_parts(value_ptr as *const u8, vallen);
            let result = slice.to_vec();
            rocksdb_free(value_ptr as *mut c_void);
            result
        };
        
        Ok(Some(value))
    }
    
    pub fn delete(&self, key: &[u8]) -> Result<(), String> {
        let mut err: *mut c_char = ptr::null_mut();
        
        unsafe {
            rocksdb_delete(
                self.inner,
                ptr::null(), // write options
                key.as_ptr() as *const c_char,
                key.len(),
                &mut err,
            );
        }
        
        if !err.is_null() {
            let err_str = unsafe {
                let err_cstr = CStr::from_ptr(err);
                let result = err_cstr.to_string_lossy().into_owned();
                rocksdb_free(err as *mut c_void);
                result
            };
            return Err(err_str);
        }
        
        Ok(())
    }
    
    pub fn delete_cf(&self, cf: &ColumnFamilyHandle, key: &[u8]) -> Result<(), String> {
        let mut err: *mut c_char = ptr::null_mut();
        
        unsafe {
            rocksdb_delete_cf(
                self.inner,
                ptr::null(), // write options
                cf.inner,
                key.as_ptr() as *const c_char,
                key.len(),
                &mut err,
            );
        }
        
        if !err.is_null() {
            let err_str = unsafe {
                let err_cstr = CStr::from_ptr(err);
                let result = err_cstr.to_string_lossy().into_owned();
                rocksdb_free(err as *mut c_void);
                result
            };
            return Err(err_str);
        }
        
        Ok(())
    }
    
    pub fn write(&self, batch: &WriteBatch) -> Result<(), String> {
        let mut err: *mut c_char = ptr::null_mut();
        
        unsafe {
            rocksdb_write(
                self.inner,
                ptr::null(), // write options
                batch.inner,
                &mut err,
            );
        }
        
        if !err.is_null() {
            let err_str = unsafe {
                let err_cstr = CStr::from_ptr(err);
                let result = err_cstr.to_string_lossy().into_owned();
                rocksdb_free(err as *mut c_void);
                result
            };
            return Err(err_str);
        }
        
        Ok(())
    }
    
    pub fn create_iterator(&self) -> DBIterator {
        let iter = unsafe {
            rocksdb_create_iterator(self.inner, ptr::null())
        };
        DBIterator::new(iter)
    }
    
    pub fn create_iterator_cf(&self, cf: &ColumnFamilyHandle) -> DBIterator {
        let iter = unsafe {
            rocksdb_create_iterator_cf(self.inner, ptr::null(), cf.inner)
        };
        DBIterator::new(iter)
    }
    
    pub fn prefix_scan(&self, prefix: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
        let mut results = Vec::new();
        let mut iter = self.create_iterator();
        
        iter.seek(prefix);
        
        while iter.valid() {
            if let Some((key, value)) = iter.key_value() {
                if key.starts_with(prefix) {
                    results.push((key, value));
                } else {
                    break;
                }
            }
            iter.next();
        }
        
        results
    }
    
    pub fn prefix_scan_cf(&self, cf: &ColumnFamilyHandle, prefix: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
        let mut results = Vec::new();
        let mut iter = self.create_iterator_cf(cf);
        
        iter.seek(prefix);
        
        while iter.valid() {
            if let Some((key, value)) = iter.key_value() {
                if key.starts_with(prefix) {
                    results.push((key, value));
                } else {
                    break;
                }
            }
            iter.next();
        }
        
        results
    }
    
    pub fn range_scan(&self, start: &[u8], end: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
        let mut results = Vec::new();
        let mut iter = self.create_iterator();
        
        iter.seek(start);
        
        while iter.valid() {
            if let Some((key, value)) = iter.key_value() {
                if key.as_slice() < end {
                    results.push((key, value));
                } else {
                    break;
                }
            }
            iter.next();
        }
        
        results
    }
    
    pub fn range_scan_cf(&self, cf: &ColumnFamilyHandle, start: &[u8], end: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
        let mut results = Vec::new();
        let mut iter = self.create_iterator_cf(cf);
        
        iter.seek(start);
        
        while iter.valid() {
            if let Some((key, value)) = iter.key_value() {
                if key.as_slice() < end {
                    results.push((key, value));
                } else {
                    break;
                }
            }
            iter.next();
        }
        
        results
    }
    
    pub fn flush(&self) -> Result<(), String> {
        let mut err: *mut c_char = ptr::null_mut();
        
        unsafe {
            rocksdb_flush(self.inner, ptr::null(), &mut err);
        }
        
        if !err.is_null() {
            let err_str = unsafe {
                let err_cstr = CStr::from_ptr(err);
                let result = err_cstr.to_string_lossy().into_owned();
                rocksdb_free(err as *mut c_void);
                result
            };
            return Err(err_str);
        }
        
        Ok(())
    }
    
    pub fn flush_cf(&self, cf: &ColumnFamilyHandle) -> Result<(), String> {
        let mut err: *mut c_char = ptr::null_mut();
        
        unsafe {
            rocksdb_flush_cf(self.inner, ptr::null(), cf.inner, &mut err);
        }
        
        if !err.is_null() {
            let err_str = unsafe {
                let err_cstr = CStr::from_ptr(err);
                let result = err_cstr.to_string_lossy().into_owned();
                rocksdb_free(err as *mut c_void);
                result
            };
            return Err(err_str);
        }
        
        Ok(())
    }
    
    pub fn compact_range(&self, start: Option<&[u8]>, end: Option<&[u8]>) {
        let start_ptr = start.map(|s| s.as_ptr() as *const c_char).unwrap_or(ptr::null());
        let start_len = start.map(|s| s.len()).unwrap_or(0);
        let end_ptr = end.map(|e| e.as_ptr() as *const c_char).unwrap_or(ptr::null());
        let end_len = end.map(|e| e.len()).unwrap_or(0);
        
        unsafe {
            rocksdb_compact_range(self.inner, start_ptr, start_len, end_ptr, end_len);
        }
    }
    
    pub fn compact_range_cf(&self, cf: &ColumnFamilyHandle, start: Option<&[u8]>, end: Option<&[u8]>) {
        let start_ptr = start.map(|s| s.as_ptr() as *const c_char).unwrap_or(ptr::null());
        let start_len = start.map(|s| s.len()).unwrap_or(0);
        let end_ptr = end.map(|e| e.as_ptr() as *const c_char).unwrap_or(ptr::null());
        let end_len = end.map(|e| e.len()).unwrap_or(0);
        
        unsafe {
            rocksdb_compact_range_cf(self.inner, cf.inner, start_ptr, start_len, end_ptr, end_len);
        }
    }
}

impl Drop for DB {
    fn drop(&mut self) {
        unsafe {
            rocksdb_close(self.inner);
        }
    }
}

unsafe impl Send for DB {}
unsafe impl Sync for DB {}

unsafe impl Send for ColumnFamilyHandle {}
unsafe impl Sync for ColumnFamilyHandle {}

unsafe impl Send for WriteBatch {}
unsafe impl Sync for WriteBatch {}

unsafe impl Send for DBIterator {}
unsafe impl Sync for DBIterator {} 