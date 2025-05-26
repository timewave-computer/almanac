/// PostgreSQL repository module
/// Contains implementations for various data repositories
// Re-export repositories
pub mod event_repository;
pub mod contract_schema_repository;

// Re-export repository implementations
pub use event_repository::{EventRepository, PostgresEventRepository};
pub use contract_schema_repository::{
    ContractSchemaRepository, PostgresContractSchemaRepository
};

// Add SqlX macros with offline mode
// This is a convenience re-export to ensure all macros use offline mode
#[cfg(feature = "offline")]
#[macro_export]
macro_rules! sqlx_query {
    ($query:expr) => {
        ::sqlx::query!($query)
    };
    ($query:expr, $($args:tt)*) => {
        ::sqlx::query!($query, $($args)*)
    };
}

#[cfg(feature = "offline")]
#[macro_export]
macro_rules! sqlx_query_as {
    ($query:expr, $type:ty) => {
        ::sqlx::query_as!($query, $type)
    };
    ($query:expr, $type:ty, $($args:tt)*) => {
        ::sqlx::query_as!($query, $type, $($args)*)
    };
}

// Default to regular macros when offline mode is not enabled
#[cfg(not(feature = "offline"))]
#[macro_export]
macro_rules! sqlx_query {
    ($query:expr) => {
        ::sqlx::query!($query)
    };
    ($query:expr, $($args:tt)*) => {
        ::sqlx::query!($query, $($args)*)
    };
}

#[cfg(not(feature = "offline"))]
#[macro_export]
macro_rules! sqlx_query_as {
    ($query:expr, $type:ty) => {
        ::sqlx::query_as!($query, $type)
    };
    ($query:expr, $type:ty, $($args:tt)*) => {
        ::sqlx::query_as!($query, $type, $($args)*)
    };
} 