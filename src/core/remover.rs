use super::executor::Executor;
use anyhow::Result;
use std::path::Path;

pub struct Remover<'a> {
    executor: &'a dyn Executor,
}

impl<'a> Remover<'a> {
    pub fn new(executor: &'a dyn Executor) -> Self {
        Self { executor }
    }

    pub async fn remove(&self, appimage_path: &Path) -> Result<()> {
        tracing::info!("Starting removal of {:?}", appimage_path);
        self.executor.remove_file(appimage_path).await?;
        tracing::info!("Removal complete.");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::executor::{DryRunExecutor, RealExecutor};
    use tempfile::tempdir;

    #[tokio::test]
    async fn dry_run_remove_does_not_delete_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("app.AppImage");
        tokio::fs::write(&file, b"data").await.unwrap();

        let executor = DryRunExecutor;
        let remover = Remover::new(&executor);
        remover.remove(&file).await.unwrap();

        assert!(file.exists(), "DryRunExecutor não deve apagar o arquivo");
    }

    #[tokio::test]
    async fn real_remove_deletes_existing_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("app.AppImage");
        tokio::fs::write(&file, b"data").await.unwrap();

        let executor = RealExecutor;
        let remover = Remover::new(&executor);
        remover.remove(&file).await.unwrap();

        assert!(!file.exists(), "arquivo deve ter sido removido");
    }

    #[tokio::test]
    async fn real_remove_nonexistent_is_ok() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("nao_existe.AppImage");

        let executor = RealExecutor;
        let remover = Remover::new(&executor);
        // remove_file no executor não falha se não existe
        remover.remove(&file).await.unwrap();
    }
}
