//! Template system for ethereum contract code generation
//! 
//! Contains Handlebars templates for generating different types of code.

use handlebars::Handlebars;
use indexer_core::Result;

/// Template manager for ethereum contract code generation
pub struct EthereumTemplateManager {
    handlebars: Handlebars<'static>,
}

impl EthereumTemplateManager {
    /// Create a new template manager and register all templates
    pub fn new() -> Result<Self> {
        let mut handlebars = Handlebars::new();
        
        // Register built-in templates
        Self::register_templates(&mut handlebars)?;
        
        Ok(Self { handlebars })
    }

    /// Register all built-in templates
    fn register_templates(handlebars: &mut Handlebars) -> Result<()> {
        // Client templates
        handlebars.register_template_string("client_mod", include_str!("client_mod.hbs"))
            .map_err(|e| indexer_core::Error::Config(format!("Failed to register client_mod template: {}", e)))?;
        
        handlebars.register_template_string("client_view", include_str!("client_view.hbs"))
            .map_err(|e| indexer_core::Error::Config(format!("Failed to register client_view template: {}", e)))?;
        
        handlebars.register_template_string("client_transaction", include_str!("client_transaction.hbs"))
            .map_err(|e| indexer_core::Error::Config(format!("Failed to register client_transaction template: {}", e)))?;

        handlebars.register_template_string("client_types", include_str!("client_types.hbs"))
            .map_err(|e| indexer_core::Error::Config(format!("Failed to register client_types template: {}", e)))?;

        // Storage templates
        handlebars.register_template_string("storage_postgres", include_str!("storage_postgres.hbs"))
            .map_err(|e| indexer_core::Error::Config(format!("Failed to register storage_postgres template: {}", e)))?;

        // API templates
        handlebars.register_template_string("api_rest", include_str!("api_rest.hbs"))
            .map_err(|e| indexer_core::Error::Config(format!("Failed to register api_rest template: {}", e)))?;

        Ok(())
    }

    /// Render a template with the given data
    pub fn render(&self, template_name: &str, data: &serde_json::Value) -> Result<String> {
        self.handlebars.render(template_name, data)
            .map_err(|e| indexer_core::Error::Config(format!("Failed to render template {}: {}", template_name, e)))
    }

    /// Get list of available templates
    pub fn available_templates(&self) -> Vec<String> {
        self.handlebars.get_templates().keys().cloned().collect()
    }
}

impl Default for EthereumTemplateManager {
    fn default() -> Self {
        Self::new().expect("Failed to initialize template manager")
    }
} 