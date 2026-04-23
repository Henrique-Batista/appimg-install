use tracing_subscriber::{fmt, EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_logger(is_tui: bool, quiet: bool, verbose: bool) -> anyhow::Result<()> {
    let log_dir = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("aura-image");
    
    std::fs::create_dir_all(&log_dir)?;
    let log_path = log_dir.join("aura-image.log");
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    // Camada para arquivo (Sempre ativa, respeita RUST_LOG ou info por padrão)
    let file_layer = fmt::layer()
        .with_target(false)
        .with_thread_ids(false)
        .with_line_number(false)
        .with_ansi(false)
        .with_writer(log_file)
        .with_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")));

    // Camada para console (Apenas se não for TUI e não estiver em modo quiet)
    let console_layer = if !is_tui && !quiet {
        let level = if verbose { "debug" } else { "info" };
        Some(fmt::layer()
            .with_target(false)
            .with_thread_ids(false)
            .with_line_number(false)
            .with_ansi(true)
            .with_writer(std::io::stderr)
            .with_filter(EnvFilter::new(level)))
    } else {
        None
    };

    tracing_subscriber::registry()
        .with(file_layer)
        .with(console_layer)
        .init();

    if is_tui {
        tracing::info!("Logger TUI inicializado. Logs silenciados no terminal.");
    } else {
        tracing::info!("Logger CLI inicializado. Logs visíveis no terminal.");
    }

    Ok(())
}
