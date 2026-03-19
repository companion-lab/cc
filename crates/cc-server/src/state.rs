use cc_config::Config;
use cc_core::bus::Bus;
use cc_permissions::PermissionGate;
use cc_provider::ProviderRegistry;
use cc_storage::Db;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: Arc<Db>,
    pub bus: Arc<Bus>,
    pub registry: Arc<ProviderRegistry>,
    pub permissions: Arc<PermissionGate>,
}

impl AppState {
    pub async fn new(config: Config, db: Arc<Db>, bus: Arc<Bus>) -> anyhow::Result<Self> {
        let registry = ProviderRegistry::from_config(&config).await?;
        Ok(Self {
            config,
            db,
            bus,
            registry: Arc::new(registry),
            permissions: Arc::new(PermissionGate::allow_all()),
        })
    }
}
