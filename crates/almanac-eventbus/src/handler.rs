use async_trait::async_trait;
use std::any::TypeId;
use std::sync::Arc;

use super::event::Event;

/// Result type for event handlers
pub type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

/// Type alias for event filters
pub type EventFilter = Box<dyn Fn(&dyn Event) -> bool + Send + Sync>;

/// Type alias for event loggers
pub type EventLogger = Box<dyn Fn(&dyn Event, &str) + Send + Sync>;

/// Trait for handling events
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Handle an event
    async fn handle(&self, event: Arc<dyn Event>) -> HandlerResult;

    /// Get the event types this handler can handle
    fn handles(&self) -> Vec<TypeId>;
}

/// A boxed event handler
pub type BoxedHandler = Box<dyn EventHandler>;

/// Function-based event handler
pub struct FnHandler<F> {
    handler_fn: F,
    event_types: Vec<TypeId>,
}

impl<F> FnHandler<F> {
    pub fn new(handler_fn: F, event_types: Vec<TypeId>) -> Self {
        Self {
            handler_fn,
            event_types,
        }
    }
}

#[async_trait]
impl<F> EventHandler for FnHandler<F>
where
    F: Fn(Arc<dyn Event>) -> HandlerResult + Send + Sync,
{
    async fn handle(&self, event: Arc<dyn Event>) -> HandlerResult {
        (self.handler_fn)(event)
    }

    fn handles(&self) -> Vec<TypeId> {
        self.event_types.clone()
    }
}

/// Async function-based event handler
pub struct AsyncFnHandler<F> {
    handler_fn: Arc<F>,
    event_types: Vec<TypeId>,
}

impl<F> AsyncFnHandler<F> {
    pub fn new(handler_fn: F, event_types: Vec<TypeId>) -> Self {
        Self {
            handler_fn: Arc::new(handler_fn),
            event_types,
        }
    }
}

#[async_trait]
impl<F, Fut> EventHandler for AsyncFnHandler<F>
where
    F: Fn(Arc<dyn Event>) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = HandlerResult> + Send,
{
    async fn handle(&self, event: Arc<dyn Event>) -> HandlerResult {
        (self.handler_fn)(event).await
    }

    fn handles(&self) -> Vec<TypeId> {
        self.event_types.clone()
    }
}

/// Handler that filters events before processing
pub struct FilteredHandler<H> {
    inner: H,
    filter: EventFilter,
}

impl<H> FilteredHandler<H> {
    pub fn new<F>(inner: H, filter: F) -> Self
    where
        F: Fn(&dyn Event) -> bool + Send + Sync + 'static,
    {
        Self {
            inner,
            filter: Box::new(filter),
        }
    }
}

#[async_trait]
impl<H: EventHandler> EventHandler for FilteredHandler<H> {
    async fn handle(&self, event: Arc<dyn Event>) -> HandlerResult {
        if (self.filter)(event.as_ref()) {
            self.inner.handle(event).await
        } else {
            Ok(())
        }
    }

    fn handles(&self) -> Vec<TypeId> {
        self.inner.handles()
    }
}

/// Handler that logs events
pub struct LoggingHandler<H> {
    inner: H,
    logger: EventLogger,
}

impl<H> LoggingHandler<H> {
    pub fn new<L>(inner: H, logger: L) -> Self
    where
        L: Fn(&dyn Event, &str) + Send + Sync + 'static,
    {
        Self {
            inner,
            logger: Box::new(logger),
        }
    }
}

#[async_trait]
impl<H: EventHandler> EventHandler for LoggingHandler<H> {
    async fn handle(&self, event: Arc<dyn Event>) -> HandlerResult {
        (self.logger)(event.as_ref(), "handling");
        let result = self.inner.handle(event.clone()).await;
        match &result {
            Ok(_) => (self.logger)(event.as_ref(), "handled"),
            Err(e) => (self.logger)(event.as_ref(), &format!("error: {}", e)),
        }
        result
    }

    fn handles(&self) -> Vec<TypeId> {
        self.inner.handles()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventMetadata, SystemStartedEvent};

    #[tokio::test]
    async fn test_fn_handler() {
        let handler = FnHandler::new(
            |event: Arc<dyn Event>| {
                let _ = event.event_type();
                Ok(())
            },
            vec![TypeId::of::<SystemStartedEvent>()],
        );

        let event = Arc::new(SystemStartedEvent {
            metadata: EventMetadata::new("test"),
        }) as Arc<dyn Event>;

        assert!(handler.handle(event).await.is_ok());
    }

    #[tokio::test]
    async fn test_async_fn_handler() {
        let handler = AsyncFnHandler::new(
            |event: Arc<dyn Event>| async move {
                let _ = event.event_type();
                Ok(())
            },
            vec![TypeId::of::<SystemStartedEvent>()],
        );

        let event = Arc::new(SystemStartedEvent {
            metadata: EventMetadata::new("test"),
        }) as Arc<dyn Event>;

        assert!(handler.handle(event).await.is_ok());
    }
}
