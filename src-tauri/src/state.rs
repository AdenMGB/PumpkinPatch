use crate::api::PatchApiError;
use patch_core::AppState;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::OnceCell;

pub struct AppStateHandle {
    cell: OnceCell<Arc<AppState>>,
    plugins_root: OnceCell<PathBuf>,
}

impl AppStateHandle {
    pub fn new() -> Self {
        Self {
            cell: OnceCell::new(),
            plugins_root: OnceCell::new(),
        }
    }

    pub fn set_plugins_root(&self, path: PathBuf) {
        let _ = self.plugins_root.set(path);
    }

    pub fn plugins_root(&self) -> PathBuf {
        self.plugins_root
            .get()
            .cloned()
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("..")
                    .join("plugins")
            })
    }

    pub async fn init(&self) -> Result<(), PatchApiError> {
        if self.cell.get().is_some() {
            return Ok(());
        }
        let state = AppState::init().await?;
        let _ = self.cell.set(Arc::new(state));
        Ok(())
    }

    pub async fn get(&self) -> Result<Arc<AppState>, PatchApiError> {
        self.cell
            .get()
            .cloned()
            .ok_or(PatchApiError::NotInitialized)
    }
}
