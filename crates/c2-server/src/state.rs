use c2_config::Config;
use c2_core::bus::Bus;
use c2_permissions::PermissionGate;
use c2_provider::ProviderRegistry;
use c2_storage::Db;
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
