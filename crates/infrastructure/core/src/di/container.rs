use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use super::provider::{Provider, ProviderError};

/// A lightweight dependency injection container using trait objects
pub struct Container {
    services: Arc<RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
    providers: Arc<RwLock<HashMap<TypeId, Box<dyn Provider>>>>,
}

impl Container {
    /// Create a new empty container
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            providers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a singleton service
    pub fn register_singleton<T>(&self, service: T) -> Result<(), ProviderError>
    where
        T: Any + Send + Sync + 'static,
    {
        let mut services = self.services.write()
            .map_err(|_| ProviderError::LockError)?;
        
        services.insert(TypeId::of::<T>(), Box::new(service));
        Ok(())
    }

    /// Register a provider for lazy service creation
    pub fn register_provider<T>(&self, provider: Box<dyn Provider>) -> Result<(), ProviderError>
    where
        T: Any + Send + Sync + 'static,
    {
        let mut providers = self.providers.write()
            .map_err(|_| ProviderError::LockError)?;
        
        providers.insert(TypeId::of::<T>(), provider);
        Ok(())
    }

    /// Resolve a service from the container
    pub fn resolve<T>(&self) -> Result<Arc<T>, ProviderError>
    where
        T: Any + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();

        // First check if we have a singleton instance
        {
            let services = self.services.read()
                .map_err(|_| ProviderError::LockError)?;
            
            if let Some(service) = services.get(&type_id) {
                return service
                    .downcast_ref::<Arc<T>>()
                    .cloned()
                    .ok_or(ProviderError::TypeMismatch);
            }
        }

        // Try to create using a provider
        {
            let providers = self.providers.read()
                .map_err(|_| ProviderError::LockError)?;
            
            if let Some(provider) = providers.get(&type_id) {
                let boxed_any = provider.provide(self)?;
                
                // Downcast to the concrete type
                let service = boxed_any
                    .downcast::<T>()
                    .map_err(|_| ProviderError::TypeMismatch)?;
                
                let arc_service: Arc<T> = Arc::from(service);
                
                // Cache the created service
                let mut services = self.services.write()
                    .map_err(|_| ProviderError::LockError)?;
                services.insert(type_id, Box::new(arc_service.clone()));
                
                return Ok(arc_service);
            }
        }

        Err(ProviderError::ServiceNotFound(std::any::type_name::<T>().to_string()))
    }

    /// Create a scoped container for request-specific services
    pub fn create_scope(&self) -> Container {
        Container {
            services: Arc::new(RwLock::new(HashMap::new())),
            providers: self.providers.clone(),
        }
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for types that can provide services to other components
pub trait ServiceProvider: Send + Sync {
    /// Resolve a service by TypeId
    fn resolve_by_type_id(&self, type_id: TypeId) -> Result<Box<dyn Any + Send + Sync>, ProviderError>;
}

impl ServiceProvider for Container {
    fn resolve_by_type_id(&self, type_id: TypeId) -> Result<Box<dyn Any + Send + Sync>, ProviderError> {
        // First check if we have a singleton instance
        {
            let services = self.services.read()
                .map_err(|_| ProviderError::LockError)?;
            
            if let Some(service) = services.get(&type_id) {
                // Clone the Arc and return it as Box<dyn Any>
                if let Some(arc_any) = service.downcast_ref::<Arc<dyn Any + Send + Sync>>() {
                    return Ok(Box::new(arc_any.clone()));
                }
            }
        }

        // Try to create using a provider
        {
            let providers = self.providers.read()
                .map_err(|_| ProviderError::LockError)?;
            
            if let Some(provider) = providers.get(&type_id) {
                return provider.provide(self);
            }
        }

        Err(ProviderError::ServiceNotFound("Unknown type".to_string()))
    }
}

/// Extension trait for typed service resolution
pub trait ServiceProviderExt {
    fn get<T>(&self) -> Result<Arc<T>, ProviderError>
    where
        T: Any + Send + Sync + 'static;
}

impl<S: ServiceProvider> ServiceProviderExt for S {
    fn get<T>(&self) -> Result<Arc<T>, ProviderError>
    where
        T: Any + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let boxed_any = self.resolve_by_type_id(type_id)?;
        
        // Try to downcast to T and wrap in Arc
        if let Ok(t) = boxed_any.downcast::<T>() {
            return Ok(Arc::from(t));
        }
        
        Err(ProviderError::TypeMismatch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestService {
        value: String,
    }

    #[test]
    fn test_singleton_registration() {
        let container = Container::new();
        let service = TestService {
            value: "test".to_string(),
        };

        container.register_singleton(service.clone()).unwrap();
        
        let resolved = container.resolve::<TestService>().unwrap();
        assert_eq!(resolved.value, "test");
    }

    #[test]
    fn test_scoped_container() {
        let container = Container::new();
        let scope = container.create_scope();
        
        let service = TestService {
            value: "scoped".to_string(),
        };
        scope.register_singleton(service).unwrap();
        
        let resolved = scope.resolve::<TestService>().unwrap();
        assert_eq!(resolved.value, "scoped");
        
        // Parent container should not have the scoped service
        assert!(container.resolve::<TestService>().is_err());
    }
    
    #[test]
    fn test_service_provider_ext() {
        use crate::di::ServiceProviderExt;
        
        let container = Container::new();
        let service = TestService {
            value: "ext_test".to_string(),
        };
        
        container.register_singleton(service).unwrap();
        
        // Use the extension trait
        let resolved: Arc<TestService> = container.get().unwrap();
        assert_eq!(resolved.value, "ext_test");
    }
}