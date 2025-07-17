use std::any::Any;
use thiserror::Error;

use super::container::ServiceProvider;

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("Service not found: {0}")]
    ServiceNotFound(String),
    
    #[error("Type mismatch during resolution")]
    TypeMismatch,
    
    #[error("Failed to acquire lock")]
    LockError,
    
    #[error("Provider creation failed: {0}")]
    CreationError(String),
    
    #[error("Dependency resolution failed: {0}")]
    DependencyError(String),
}

/// Trait for providing service instances
pub trait Provider: Send + Sync {
    /// Create an instance of the service
    fn provide(&self, container: &dyn ServiceProvider) -> Result<Box<dyn Any + Send + Sync>, ProviderError>;
}

/// A provider that creates new instances each time
pub struct TransientProvider<T, F>
where
    T: Any + Send + Sync + 'static,
    F: Fn(&dyn ServiceProvider) -> Result<T, ProviderError> + Send + Sync,
{
    factory: F,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, F> TransientProvider<T, F>
where
    T: Any + Send + Sync + 'static,
    F: Fn(&dyn ServiceProvider) -> Result<T, ProviderError> + Send + Sync,
{
    pub fn new(factory: F) -> Self {
        Self {
            factory,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T, F> Provider for TransientProvider<T, F>
where
    T: Any + Send + Sync + 'static,
    F: Fn(&dyn ServiceProvider) -> Result<T, ProviderError> + Send + Sync,
{
    fn provide(&self, container: &dyn ServiceProvider) -> Result<Box<dyn Any + Send + Sync>, ProviderError> {
        let instance = (self.factory)(container)?;
        Ok(Box::new(instance))
    }
}

/// Provider traits for external dependencies
pub trait StorageProvider: Send + Sync {
    type Storage: Send + Sync;
    
    fn provide_storage(&self) -> Result<Self::Storage, ProviderError>;
}

pub trait NetworkProvider: Send + Sync {
    type Client: Send + Sync;
    
    fn provide_client(&self) -> Result<Self::Client, ProviderError>;
}

pub trait TimeProvider: Send + Sync {
    fn now(&self) -> std::time::SystemTime;
    fn now_millis(&self) -> u64;
}

pub trait ConfigProvider: Send + Sync {
    fn get_string(&self, key: &str) -> Option<String>;
    fn get_int(&self, key: &str) -> Option<i64>;
    fn get_bool(&self, key: &str) -> Option<bool>;
}

pub trait LoggerProvider: Send + Sync {
    fn log(&self, level: LogLevel, message: &str);
    fn log_with_context(&self, level: LogLevel, message: &str, context: &[(String, String)]);
}

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

pub trait MetricsProvider: Send + Sync {
    fn increment(&self, metric: &str, tags: &[(String, String)]);
    fn gauge(&self, metric: &str, value: f64, tags: &[(String, String)]);
    fn histogram(&self, metric: &str, value: f64, tags: &[(String, String)]);
}