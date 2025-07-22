#[cfg(test)]
pub use crate::routes::create_test_router;

#[cfg(test)]
pub async fn create_test_app_state() -> crate::state::AppState {
    use std::sync::Arc;

    let storage = Arc::new(crate::MemoryExtendedStorage::new());

    crate::state::AppState::new(storage)
}
