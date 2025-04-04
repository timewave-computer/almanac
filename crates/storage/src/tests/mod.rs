/// Storage benchmarking and testing
pub mod common;
pub mod rocks_benchmark;
pub mod postgres_benchmark;
pub mod sync_benchmark;

pub use common::*;

#[cfg(test)]
mod test_imports {
    use super::*;
    
    // Import all test modules here to make them visible for testing
    #[allow(unused_imports)]
    use rocks_benchmark::*;
    #[allow(unused_imports)]
    use postgres_benchmark::*;
    #[allow(unused_imports)]
    use sync_benchmark::*;
} 