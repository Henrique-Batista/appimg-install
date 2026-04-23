use super::executor::Executor;
use anyhow::Result;
use std::path::Path;

pub struct Installer<'a> {
    executor: &'a dyn Executor,
}

impl<'a> Installer<'a> {
    pub fn new(executor: &'a dyn Executor) -> Self {
        Self { executor }
    }

    pub async fn install(&self, appimage_path: &Path, target_dir: &Path) -> Result<()> {
        let file_name = appimage_path.file_name().unwrap_or_default();
        let target_path = target_dir.join(file_name);

        tracing::info!("Starting installation to {:?}", target_path);

        self.executor.copy_file(appimage_path, &target_path).await?;
        self.executor.set_permissions(&target_path, 0o755).await?;

        tracing::info!("Installation complete.");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::executor::{DryRunExecutor, RealExecutor};
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    #[tokio::test]
    async fn dry_run_install_does_not_copy() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("app.AppImage");
        tokio::fs::write(&src, b"fake").await.unwrap();

        let executor = DryRunExecutor;
        let installer = Installer::new(&executor);
        installer.install(&src, dir.path()).await.unwrap();

        // O arquivo original existe, mas nada deve ter sido copiado sob outro nome
        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        // Apenas o src original deve existir no diretório
        assert_eq!(entries.len(), 1);
    }

    #[tokio::test]
    async fn real_install_copies_file_and_sets_permissions() {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();
        let src = src_dir.path().join("myapp.AppImage");
        tokio::fs::write(&src, b"\x7fELFfakedata").await.unwrap();

        let executor = RealExecutor;
        let installer = Installer::new(&executor);
        installer.install(&src, dst_dir.path()).await.unwrap();

        let installed = dst_dir.path().join("myapp.AppImage");
        assert!(installed.exists(), "arquivo deve ter sido copiado");

        let meta = std::fs::metadata(&installed).unwrap();
        assert_eq!(
            meta.permissions().mode() & 0o777,
            0o755,
            "permissões devem ser 0o755"
        );
    }

    #[tokio::test]
    async fn real_install_preserves_file_content() {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();
        let content = b"unique-content-123";
        let src = src_dir.path().join("data.AppImage");
        tokio::fs::write(&src, content).await.unwrap();

        let executor = RealExecutor;
        let installer = Installer::new(&executor);
        installer.install(&src, dst_dir.path()).await.unwrap();

        let bytes = tokio::fs::read(dst_dir.path().join("data.AppImage"))
            .await
            .unwrap();
        assert_eq!(bytes, content);
    }
}
