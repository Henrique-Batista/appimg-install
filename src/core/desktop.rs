use super::executor::Executor;
use anyhow::Result;
use std::path::Path;

/// Normaliza o nome do arquivo de atalho: minúsculas, espaços viram hífens.
fn desktop_filename(name: &str) -> String {
    format!("{}.desktop", name.to_lowercase().replace(' ', "-"))
}

pub async fn create_desktop_entry(
    executor: &dyn Executor,
    name: &str,
    exec_path: &Path,
    desktop_dir: &Path,
) -> Result<()> {
    // IMPORTANTE: `Exec` NÃO deve usar aspas ao redor do caminho na spec FreeDesktop.
    // Usamos `{}` (Display) e não `{:?}` (Debug) para evitar que Rust insira as aspas extras.
    let exec_str = exec_path.display().to_string();

    let desktop_content = format!(
        "[Desktop Entry]\n\
        Version=1.0\n\
        Type=Application\n\
        Name={name}\n\
        Exec={exec_str}\n\
        Terminal=false\n\
        Categories=Utility;\n\
        StartupNotify=true\n"
    );

    let desktop_file = desktop_dir.join(desktop_filename(name));
    executor.write_file(&desktop_file, &desktop_content).await?;

    tracing::info!(
        "Desktop entry criado: {:?} (diretório: {:?})",
        desktop_file.file_name().unwrap_or_default(),
        desktop_dir
    );

    // Atualiza o banco de dados de aplicações para que o KDE/GNOME detecte o novo atalho.
    refresh_desktop_database(desktop_dir);

    Ok(())
}

pub async fn remove_desktop_entry(
    executor: &dyn Executor,
    name: &str,
    desktop_dir: &Path,
) -> Result<()> {
    let desktop_file = desktop_dir.join(desktop_filename(name));
    if desktop_file.exists() {
        executor.remove_file(&desktop_file).await?;
        tracing::info!(
            "Desktop entry removido: {:?} (diretório: {:?})",
            desktop_file.file_name().unwrap_or_default(),
            desktop_dir
        );
        refresh_desktop_database(desktop_dir);
    } else {
        tracing::warn!("Desktop entry não encontrado para remover: {:?}", desktop_file);
    }
    Ok(())
}

/// Dispara `update-desktop-database` no diretório dado para que
/// o ambiente desktop (KDE Plasma, GNOME, etc.) reflita as mudanças
/// sem precisar reiniciar a sessão.
fn refresh_desktop_database(desktop_dir: &Path) {
    let dir = desktop_dir.to_path_buf();
    tracing::info!("Atualizando banco de dados de aplicações em {:?}...", dir);
    // Spawna sem bloquear; falhas são apenas avisadas via log.
    match std::process::Command::new("update-desktop-database")
        .arg(&dir)
        .status()
    {
        Ok(status) if status.success() => {
            tracing::info!("update-desktop-database concluído com sucesso.");
        }
        Ok(status) => {
            tracing::warn!("update-desktop-database retornou código {:?}", status.code());
        }
        Err(e) => {
            tracing::warn!("Não foi possível executar update-desktop-database: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::executor::RealExecutor;
    use tempfile::tempdir;

    // ── desktop_filename ──────────────────────────────────────────────────────

    #[test]
    fn filename_lowercase_and_spaces_to_hyphens() {
        assert_eq!(desktop_filename("My App"), "my-app.desktop");
    }

    #[test]
    fn filename_already_lowercase_no_spaces() {
        assert_eq!(desktop_filename("kdenlive"), "kdenlive.desktop");
    }

    #[test]
    fn filename_mixed_case() {
        assert_eq!(desktop_filename("OpenRGB"), "openrgb.desktop");
    }

    // ── create_desktop_entry ──────────────────────────────────────────────────

    #[tokio::test]
    async fn create_entry_produces_correct_file_path() {
        let dir = tempdir().unwrap();
        let exec_path = dir.path().join("myapp.AppImage");
        let executor = RealExecutor;

        create_desktop_entry(&executor, "myapp", &exec_path, dir.path())
            .await
            .unwrap();

        let desktop_file = dir.path().join("myapp.desktop");
        assert!(desktop_file.exists(), "arquivo .desktop deve existir");
    }

    #[tokio::test]
    async fn create_entry_exec_has_no_extra_quotes() {
        let dir = tempdir().unwrap();
        let exec_path = dir.path().join("test-app.AppImage");
        let executor = RealExecutor;

        create_desktop_entry(&executor, "test-app", &exec_path, dir.path())
            .await
            .unwrap();

        let content = tokio::fs::read_to_string(dir.path().join("test-app.desktop"))
            .await
            .unwrap();

        // Exec NÃO deve ter aspas extras em volta do caminho
        let exec_line = content
            .lines()
            .find(|l| l.starts_with("Exec="))
            .expect("linha Exec= deve existir");
        assert!(
            !exec_line.contains('"'),
            "Exec não deve ter aspas duplas: {}",
            exec_line
        );
    }

    #[tokio::test]
    async fn create_entry_contains_required_fields() {
        let dir = tempdir().unwrap();
        let exec_path = dir.path().join("app.AppImage");
        let executor = RealExecutor;

        create_desktop_entry(&executor, "app", &exec_path, dir.path())
            .await
            .unwrap();

        let content = tokio::fs::read_to_string(dir.path().join("app.desktop"))
            .await
            .unwrap();

        assert!(content.contains("[Desktop Entry]"));
        assert!(content.contains("Version=1.0"));
        assert!(content.contains("Type=Application"));
        assert!(content.contains("Name=app"));
        assert!(content.contains("Terminal=false"));
        assert!(content.contains("StartupNotify=true"));
    }

    // ── remove_desktop_entry ──────────────────────────────────────────────────

    #[tokio::test]
    async fn remove_entry_deletes_existing_file() {
        let dir = tempdir().unwrap();
        let desktop_file = dir.path().join("app.desktop");
        tokio::fs::write(&desktop_file, "[Desktop Entry]\n").await.unwrap();

        let executor = RealExecutor;
        remove_desktop_entry(&executor, "app", dir.path())
            .await
            .unwrap();

        assert!(!desktop_file.exists(), ".desktop deve ter sido removido");
    }

    #[tokio::test]
    async fn remove_entry_is_ok_when_file_does_not_exist() {
        let dir = tempdir().unwrap();
        let executor = RealExecutor;
        // Não deve retornar erro mesmo sem o arquivo
        remove_desktop_entry(&executor, "nonexistent-app", dir.path())
            .await
            .unwrap();
    }
}
