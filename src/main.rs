mod core;
mod ui;
mod utils;

use clap::Parser;
use std::path::PathBuf;
use ui::cli_args::{Cli, Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    // O logger deve ser inicializado APÓS o parse do CLI para sabermos se:
    // 1. Estamos no modo TUI (que exige silêncio no terminal)
    // 2. Estamos no modo Quiet ou Verbose na CLI
    let is_tui = matches!(&cli.command, Some(Commands::Tui) | None);
    utils::logger::init_logger(is_tui, cli.quiet, cli.verbose)?;

    match &cli.command {
        Some(Commands::Install { path, global, dry_run, target_dir, no_desktop }) => {
            tracing::info!("Installing AppImage from {}", path);
            
            let path_buf = PathBuf::from(path);
            
            if let Err(e) = core::validator::validate_appimage(&path_buf).await {
                tracing::error!("Validation failed: {}", e);
                return Ok(());
            }

            let executor: Box<dyn core::executor::Executor> = if *dry_run {
                Box::new(core::executor::DryRunExecutor)
            } else {
                Box::new(core::executor::RealExecutor)
            };

            let installer = core::installer::Installer::new(executor.as_ref());
            let default_target = dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("~/.local/share"))
                .join("appimages");
            
            let final_target = if *global {
                PathBuf::from("/opt/appimages")
            } else {
                target_dir.clone().unwrap_or(default_target)
            };

            if !final_target.exists() {
                if !*dry_run {
                    tokio::fs::create_dir_all(&final_target).await?;
                } else {
                    tracing::info!("[DRY-RUN] Would create directory {:?}", final_target);
                }
            }

            if let Err(e) = installer.install(&path_buf, &final_target).await {
                tracing::error!("Installation failed: {}", e);
                return Ok(());
            }

            if !no_desktop {
                let app_name = path_buf.file_stem().unwrap_or_default().to_string_lossy().to_string();
                let installed_path = final_target.join(path_buf.file_name().unwrap_or_default());
                let desktop_dir = dirs::data_local_dir().unwrap_or_default().join("applications");
                
                if !desktop_dir.exists() && !*dry_run {
                    tokio::fs::create_dir_all(&desktop_dir).await?;
                }

                if let Err(e) = core::desktop::create_desktop_entry(executor.as_ref(), &app_name, &installed_path, &desktop_dir).await {
                    tracing::warn!("Falha ao criar atalho .desktop: {:?}", e);
                } else {
                    tracing::info!("Atalho .desktop criado para '{}'", app_name);
                }
            }
        }
        Some(Commands::Remove { name }) => {
            tracing::info!("Removing AppImage: {}", name);
            let mut local = core::scanner::list_installed_appimages(false).await?;
            let mut global = core::scanner::list_installed_appimages(true).await?;
            local.append(&mut global);
            
            let target = local.into_iter().find(|app| {
                app.name.to_lowercase() == name.to_lowercase() || 
                app.path.file_stem().unwrap_or_default().to_string_lossy().to_lowercase() == name.to_lowercase()
            });

            if let Some(app) = target {
                let executor = core::executor::RealExecutor;
                let remover = core::remover::Remover::new(&executor);
                
                remover.remove(&app.path).await?;
                let desktop_name = app.path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                let desktop_dir = dirs::data_local_dir().unwrap_or_default().join("applications");
                if let Err(e) = core::desktop::remove_desktop_entry(&executor, &desktop_name, &desktop_dir).await {
                    tracing::warn!("Erro ao remover atalho: {:?}", e);
                }
                
                println!("AppImage '{}' removido com sucesso.", name);
            } else {
                eprintln!("AppImage não encontrado: {}", name);
            }
        }
        Some(Commands::List) => {
            tracing::info!("Listing AppImages...");
            let mut local = core::scanner::list_installed_appimages(false).await?;
            let mut global = core::scanner::list_installed_appimages(true).await?;
            local.append(&mut global);
            
            if local.is_empty() {
                println!("Nenhum AppImage instalado.");
            } else {
                println!("{:<30} | {:<10} | Caminho", "Nome", "Tamanho");
                println!("{:-<30}-+-{:-<10}-+-{:-<50}", "", "", "");
                for app in local {
                    println!("{:<30} | {:<7.2} MB | {}", app.name, app.size_mb, app.path.display());
                }
            }
        }
        Some(Commands::Tui) | None => {
            ui::tui_render::run_tui().await?;
        }
    }

    Ok(())
}
