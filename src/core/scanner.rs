use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppImageInfo {
    pub name: String,
    pub path: PathBuf,
    pub size_mb: f64,
}

pub async fn list_installed_appimages(global: bool) -> Result<Vec<AppImageInfo>> {
    let target_dir = if global {
        PathBuf::from("/opt/appimages")
    } else {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("~/.local/share"))
            .join("appimages")
    };

    let mut appimages = Vec::new();

    if !target_dir.exists() || !target_dir.is_dir() {
        return Ok(appimages);
    }

    let mut entries = tokio::fs::read_dir(target_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
            // Assumir que arquivos no diretório são AppImages, ou verificar a extensão
            if file_name.ends_with(".appimage") || file_name.contains("appimage") {
                let metadata = entry.metadata().await?;
                let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
                
                appimages.push(AppImageInfo {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path,
                    size_mb,
                });
            }
        }
    }

    Ok(appimages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Versão testável do scanner que aceita um diretório arbitrário,
    /// sem depender de variáveis de ambiente do sistema.
    async fn scan_dir(dir: &std::path::Path) -> Vec<AppImageInfo> {
        if !dir.exists() || !dir.is_dir() {
            return vec![];
        }
        let mut appimages = Vec::new();
        let mut entries = tokio::fs::read_dir(dir).await.unwrap();
        while let Some(entry) = entries.next_entry().await.unwrap() {
            let path = entry.path();
            if path.is_file() {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase();
                if name.ends_with(".appimage") || name.contains("appimage") {
                    let meta = entry.metadata().await.unwrap();
                    let size_mb = meta.len() as f64 / (1024.0 * 1024.0);
                    appimages.push(AppImageInfo {
                        name: entry.file_name().to_string_lossy().to_string(),
                        path,
                        size_mb,
                    });
                }
            }
        }
        appimages
    }

    #[tokio::test]
    async fn empty_directory_returns_empty_list() {
        let dir = tempdir().unwrap();
        let results = scan_dir(dir.path()).await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn nonexistent_directory_returns_empty_list() {
        let path = std::path::Path::new("/tmp/aura_nao_existe_scanner_test_dir");
        let results = scan_dir(path).await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn detects_appimage_by_extension() {
        let dir = tempdir().unwrap();
        tokio::fs::write(dir.path().join("app.AppImage"), b"data")
            .await
            .unwrap();
        let results = scan_dir(dir.path()).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "app.AppImage");
    }

    #[tokio::test]
    async fn detects_appimage_case_insensitive() {
        let dir = tempdir().unwrap();
        tokio::fs::write(dir.path().join("app.APPIMAGE"), b"data")
            .await
            .unwrap();
        let results = scan_dir(dir.path()).await;
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn ignores_non_appimage_files() {
        let dir = tempdir().unwrap();
        tokio::fs::write(dir.path().join("readme.txt"), b"text")
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("script.sh"), b"#!/bin/sh")
            .await
            .unwrap();
        let results = scan_dir(dir.path()).await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn detects_multiple_appimages() {
        let dir = tempdir().unwrap();
        tokio::fs::write(dir.path().join("app1.AppImage"), b"a")
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("app2.appimage"), b"b")
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("other.txt"), b"c")
            .await
            .unwrap();
        let results = scan_dir(dir.path()).await;
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn size_mb_is_calculated_correctly() {
        let dir = tempdir().unwrap();
        // Cria arquivo de exatamente 1 MiB
        let one_mb = vec![0u8; 1024 * 1024];
        tokio::fs::write(dir.path().join("big.AppImage"), &one_mb)
            .await
            .unwrap();
        let results = scan_dir(dir.path()).await;
        assert_eq!(results.len(), 1);
        assert!((results[0].size_mb - 1.0).abs() < 0.001);
    }
}
