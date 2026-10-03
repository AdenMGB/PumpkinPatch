use crate::api::PatchApiError;
use patch_core::AppState;
use std::sync::Arc;
use tokio::sync::OnceCell;

pub struct AppStateHandle {
    cell: OnceCell<Arc<AppState>>,
}

impl AppStateHandle {
    pub fn new() -> Self {
        Self {
            cell: OnceCell::new(),
        }
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
