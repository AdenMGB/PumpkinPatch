use crate::db::Database;
use crate::error::Result;
use crate::java::resolve_java_executable;
use crate::models::AppSettings;
use crate::network::NetworkOrchestrator;
use crate::process::ProcessSupervisor;
use crate::releases::ReleaseService;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub dirs: AppDirs,
    pub db: Database,
    pub releases: ReleaseService,
    pub networks: NetworkOrchestrator,
    pub processes: Arc<RwLock<ProcessSupervisor>>,
    settings: Arc<RwLock<AppSettings>>,
}

#[derive(Debug, Clone)]
pub struct AppDirs {
    pub root: PathBuf,
    pub networks: PathBuf,
    pub instances: PathBuf,
    pub binaries_pumpkin: PathBuf,
    pub binaries_velocity: PathBuf,
    pub plugins_cache: PathBuf,
    pub cache: PathBuf,
    pub database: PathBuf,
}

impl AppDirs {
    pub fn new(root: PathBuf) -> Self {
        Self {
            networks: root.join("networks"),
            instances: root.join("instances"),
            binaries_pumpkin: root.join("binaries").join("pumpkin"),
            binaries_velocity: root.join("binaries").join("velocity"),
            plugins_cache: root.join("plugins").join("cache"),
            cache: root.join("cache"),
            database: root.join("patch-core.db"),
            root,
        }
    }

    pub fn ensure_all(&self) -> std::io::Result<()> {
        for dir in [
            &self.root,
            &self.networks,
            &self.instances,
            &self.binaries_pumpkin,
            &self.binaries_velocity,
            &self.plugins_cache,
            &self.cache,
        ] {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }
}

impl AppState {
    pub async fn init() -> Result<Self> {
        let root = directories::ProjectDirs::from("com", "PumpkinPatch", "Pumpkin Patch")
            .map(|d| d.data_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".").join(".pumpkin-patch-data"));
        let dirs = AppDirs::new(root);
        dirs.ensure_all()?;
        let db = Database::connect(&dirs.database).await?;
        let settings = db.get_settings().await?;
        let settings = ensure_java_in_settings(&db, settings).await?;
        let releases = ReleaseService::new(dirs.clone());
        let networks = NetworkOrchestrator::new(dirs.clone(), releases.clone());
        let processes = Arc::new(RwLock::new(ProcessSupervisor::new()));
        Ok(Self {
            dirs,
            db,
            releases,
            networks,
            processes,
            settings: Arc::new(RwLock::new(settings)),
        })
    }

    pub async fn settings(&self) -> AppSettings {
        self.settings.read().await.clone()
    }

    pub async fn update_settings(&self, settings: AppSettings) -> Result<()> {
        let settings = ensure_java_in_settings(&self.db, settings).await?;
        self.db.save_settings(&settings).await?;
        *self.settings.write().await = settings;
        Ok(())
    }

    pub async fn resolve_java_for_runtime(&self) -> Result<String> {
        let settings = self.settings().await;
        let path = resolve_java_executable(&settings.java_path)?;
        let resolved = path.to_string_lossy().into_owned();
        if resolved != settings.java_path {
            let updated = AppSettings {
                java_path: resolved.clone(),
                ..settings
            };
            self.db.save_settings(&updated).await?;
            *self.settings.write().await = updated;
        }
        Ok(resolved)
    }
}

async fn ensure_java_in_settings(db: &Database, settings: AppSettings) -> Result<AppSettings> {
    if let Ok(path) = resolve_java_executable(&settings.java_path) {
        let java_path = path.to_string_lossy().into_owned();
        if java_path != settings.java_path {
            let updated = AppSettings {
                java_path,
                ..settings
            };
            db.save_settings(&updated).await?;
            return Ok(updated);
        }
    }
    Ok(settings)
}
