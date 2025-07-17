use std::sync::Arc;
use thiserror::Error;

use super::container::{ServiceProvider, ServiceProviderExt};
use super::provider::ProviderError;

#[derive(Error, Debug)]
pub enum FactoryError {
    #[error("Factory creation failed: {0}")]
    CreationError(String),
    
    #[error("Missing dependency: {0}")]
    MissingDependency(String),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}

impl From<FactoryError> for ProviderError {
    fn from(err: FactoryError) -> Self {
        ProviderError::CreationError(err.to_string())
    }
}

/// Type alias for builder functions
pub type BuilderFn<T> = Box<dyn Fn(&dyn ServiceProvider) -> Result<T, FactoryError> + Send + Sync>;

/// Type alias for typed builder functions
pub type TypedBuilderFn<T, P> = Box<dyn Fn(&P) -> Result<T, FactoryError> + Send + Sync>;

/// Factory trait for complex object construction
pub trait Factory<T>: Send + Sync {
    fn create(&self, provider: &dyn ServiceProvider) -> Result<T, FactoryError>;
}

/// Builder pattern for complex service construction
pub struct ServiceBuilder<T> {
    builder_fn: BuilderFn<T>,
}

impl<T> ServiceBuilder<T> {
    pub fn new<F>(builder_fn: F) -> Self
    where
        F: Fn(&dyn ServiceProvider) -> Result<T, FactoryError> + Send + Sync + 'static,
    {
        Self {
            builder_fn: Box::new(builder_fn),
        }
    }

    pub fn build(&self, provider: &dyn ServiceProvider) -> Result<T, FactoryError> {
        (self.builder_fn)(provider)
    }
}

/// Type-safe builder pattern for complex service construction
pub struct ServiceBuilderTyped<T, P> {
    builder_fn: TypedBuilderFn<T, P>,
    _phantom: std::marker::PhantomData<P>,
}

impl<T, P> ServiceBuilderTyped<T, P>
where
    P: ServiceProviderExt,
{
    pub fn new<F>(builder_fn: F) -> Self
    where
        F: Fn(&P) -> Result<T, FactoryError> + Send + Sync + 'static,
    {
        Self {
            builder_fn: Box::new(builder_fn),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn build(&self, provider: &P) -> Result<T, FactoryError> {
        (self.builder_fn)(provider)
    }
}

/// Factory for creating API server instances
pub struct ApiServerFactory;

impl ApiServerFactory {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ApiServerFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Factory for creating storage instances
pub struct StorageFactory {
    storage_type: StorageType,
}

#[derive(Clone, Copy)]
pub enum StorageType {
    Postgres,
    RocksDb,
    Memory,
}

impl StorageFactory {
    pub fn new(storage_type: StorageType) -> Self {
        Self { storage_type }
    }
    
    pub fn storage_type(&self) -> StorageType {
        self.storage_type
    }
}

/// Factory for creating WebSocket connections
pub struct WebSocketFactory {
    max_connections: usize,
    buffer_size: usize,
}

impl WebSocketFactory {
    pub fn new(max_connections: usize, buffer_size: usize) -> Self {
        Self {
            max_connections,
            buffer_size,
        }
    }
    
    pub fn max_connections(&self) -> usize {
        self.max_connections
    }
    
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }
}

/// Composite factory for creating complex services with multiple dependencies
pub struct CompositeFactory<T> {
    factories: Vec<Arc<dyn Factory<T>>>,
}

impl<T> CompositeFactory<T> {
    pub fn new() -> Self {
        Self {
            factories: Vec::new(),
        }
    }

    pub fn add_factory(mut self, factory: Arc<dyn Factory<T>>) -> Self {
        self.factories.push(factory);
        self
    }
}

impl<T> Default for CompositeFactory<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestService {
        value: String,
    }

    struct TestFactory;

    impl Factory<TestService> for TestFactory {
        fn create(&self, _provider: &dyn ServiceProvider) -> Result<TestService, FactoryError> {
            Ok(TestService {
                value: "created".to_string(),
            })
        }
    }

    #[test]
    fn test_service_builder_typed() {
        use crate::di::Container;
        
        let container = Container::new();
        let builder: ServiceBuilderTyped<TestService, Container> = ServiceBuilderTyped::new(|_provider| {
            Ok(TestService {
                value: "built".to_string(),
            })
        });

        let service = builder.build(&container).unwrap();
        assert_eq!(service.value, "built");
    }

    #[test]
    fn test_factory_pattern() {
        use crate::di::Container;
        
        let container = Container::new();
        let factory = TestFactory;
        
        let service = factory.create(&container).unwrap();
        assert_eq!(service.value, "created");
    }
}