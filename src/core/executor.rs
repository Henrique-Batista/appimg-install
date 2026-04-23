use anyhow::Result;
use std::path::Path;

#[async_trait::async_trait]
pub trait Executor: Send + Sync {
    async fn copy_file(&self, src: &Path, dst: &Path) -> Result<()>;
    async fn set_permissions(&self, path: &Path, mode: u32) -> Result<()>;
    async fn write_file(&self, path: &Path, content: &str) -> Result<()>;
    async fn remove_file(&self, path: &Path) -> Result<()>;
}

pub struct RealExecutor;

#[async_trait::async_trait]
impl Executor for RealExecutor {
    async fn copy_file(&self, src: &Path, dst: &Path) -> Result<()> {
        tracing::info!("Copying {:?} to {:?}", src, dst);
        tokio::fs::copy(src, dst).await?;
        Ok(())
    }

    async fn set_permissions(&self, path: &Path, mode: u32) -> Result<()> {
        tracing::info!("Setting permissions of {:?} to {:o}", path, mode);
        use std::os::unix::fs::PermissionsExt;
        let mut perms = tokio::fs::metadata(path).await?.permissions();
        perms.set_mode(mode);
        tokio::fs::set_permissions(path, perms).await?;
        Ok(())
    }

    async fn write_file(&self, path: &Path, content: &str) -> Result<()> {
        tracing::info!("Writing to {:?}", path);
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    async fn remove_file(&self, path: &Path) -> Result<()> {
        tracing::info!("Removing file {:?}", path);
        if path.exists() {
            tokio::fs::remove_file(path).await?;
        }
        Ok(())
    }
}

pub struct DryRunExecutor;

#[async_trait::async_trait]
impl Executor for DryRunExecutor {
    async fn copy_file(&self, src: &Path, dst: &Path) -> Result<()> {
        tracing::info!("[DRY-RUN] Would copy {:?} to {:?}", src, dst);
        Ok(())
    }

    async fn set_permissions(&self, path: &Path, mode: u32) -> Result<()> {
        tracing::info!("[DRY-RUN] Would set permissions of {:?} to {:o}", path, mode);
        Ok(())
    }

    async fn write_file(&self, path: &Path, _content: &str) -> Result<()> {
        tracing::info!("[DRY-RUN] Would write file to {:?}", path);
        Ok(())
    }

    async fn remove_file(&self, path: &Path) -> Result<()> {
        tracing::info!("[DRY-RUN] Would remove file {:?}", path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::tempdir;

    // ── DryRunExecutor ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn dry_run_copy_file_does_not_create_dst() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.txt");
        let dst = dir.path().join("dst.txt");
        // src nem precisa existir — DryRun não toca o disco
        let executor = DryRunExecutor;
        executor.copy_file(&src, &dst).await.unwrap();
        assert!(!dst.exists(), "DryRunExecutor não deve criar o destino");
    }

    #[tokio::test]
    async fn dry_run_write_file_does_not_create_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("output.txt");
        let executor = DryRunExecutor;
        executor.write_file(&file, "conteúdo").await.unwrap();
        assert!(!file.exists(), "DryRunExecutor não deve criar o arquivo");
    }

    #[tokio::test]
    async fn dry_run_remove_file_does_not_fail_on_missing() {
        let path = Path::new("/tmp/nao-existe-jamais-aura.txt");
        let executor = DryRunExecutor;
        executor.remove_file(path).await.unwrap();
    }

    #[tokio::test]
    async fn dry_run_set_permissions_does_not_fail() {
        let path = Path::new("/tmp/nao-existe-jamais-aura.txt");
        let executor = DryRunExecutor;
        executor.set_permissions(path, 0o755).await.unwrap();
    }

    // ── RealExecutor ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn real_executor_write_and_remove() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        let executor = RealExecutor;

        executor.write_file(&file, "hello aura").await.unwrap();
        assert!(file.exists());
        let content = tokio::fs::read_to_string(&file).await.unwrap();
        assert_eq!(content, "hello aura");

        executor.remove_file(&file).await.unwrap();
        assert!(!file.exists());
    }

    #[tokio::test]
    async fn real_executor_copy_file_produces_identical_content() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.bin");
        let dst = dir.path().join("dst.bin");
        tokio::fs::write(&src, b"content").await.unwrap();

        let executor = RealExecutor;
        executor.copy_file(&src, &dst).await.unwrap();

        assert!(dst.exists());
        let bytes = tokio::fs::read(&dst).await.unwrap();
        assert_eq!(bytes, b"content");
    }

    #[tokio::test]
    async fn real_executor_set_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let file = dir.path().join("perm_test.sh");
        tokio::fs::write(&file, b"#!/bin/sh").await.unwrap();

        let executor = RealExecutor;
        executor.set_permissions(&file, 0o755).await.unwrap();

        let meta = tokio::fs::metadata(&file).await.unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o755);
    }

    #[tokio::test]
    async fn real_executor_remove_nonexistent_file_is_ok() {
        let executor = RealExecutor;
        let path = Path::new("/tmp/aura_nao_existe_remove_test.txt");
        // remove_file só apaga se existir, deve retornar Ok
        executor.remove_file(path).await.unwrap();
    }
}
