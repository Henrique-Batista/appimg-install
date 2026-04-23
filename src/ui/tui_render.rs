use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use crate::core::scanner::{list_installed_appimages, AppImageInfo};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Terminal,
};
use std::io;

pub async fn run_tui() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        tracing::error!("Error running TUI: {:?}", err);
    }

    Ok(())
}

#[derive(PartialEq)]
pub enum Focus {
    Menu,
    List,
    InstallForm,
}

#[derive(Clone, Copy, PartialEq)]
pub enum MenuItem {
    Listar,
    Instalar,
    Remover,
}

impl MenuItem {
    fn as_str(&self) -> &'static str {
        match self {
            MenuItem::Listar => "Listar",
            MenuItem::Instalar => "Instalar",
            MenuItem::Remover => "Remover",
        }
    }
}

pub struct InstallFormState {
    pub appimage_path: String,
    pub target_dir: String,
    pub custom_name: String,
    pub create_desktop: bool,
    pub global: bool,
    pub step: usize,
}

impl Default for InstallFormState {
    fn default() -> Self {
        Self {
            appimage_path: String::new(),
            target_dir: String::new(),
            custom_name: String::new(),
            create_desktop: true, // Criar atalho .desktop por padrão
            global: false,        // Instalação local por padrão
            step: 0,
        }
    }
}

struct App {
    pub items: Vec<AppImageInfo>,
    pub list_state: ListState,
    pub focus: Focus,
    pub menu_items: Vec<MenuItem>,
    pub menu_state: ListState,
    pub install_form: InstallFormState,
    pub has_interacted: bool,
    pub app_logs: Vec<String>,
}

impl App {
    async fn new() -> Result<App> {
        let mut appimages = list_installed_appimages(false).await.unwrap_or_default();
        let mut global_appimages = list_installed_appimages(true).await.unwrap_or_default();
        appimages.append(&mut global_appimages);

        let mut state = ListState::default();
        if !appimages.is_empty() {
            state.select(Some(0));
        }

        let menu_items = vec![MenuItem::Listar, MenuItem::Instalar, MenuItem::Remover];
        let mut menu_state = ListState::default();
        menu_state.select(Some(0));

        Ok(App {
            items: appimages,
            list_state: state,
            focus: Focus::Menu,
            menu_items,
            menu_state,
            install_form: InstallFormState::default(),
            has_interacted: false,
            app_logs: vec!["Aura-Image TUI Iniciado.".to_string()],
        })
    }

    pub fn next(&mut self) {
        if self.items.is_empty() { return; }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.items.is_empty() { return; }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn menu_next(&mut self) {
        let i = match self.menu_state.selected() {
            Some(i) => {
                if i >= self.menu_items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.menu_state.select(Some(i));
    }

    pub fn menu_previous(&mut self) {
        let i = match self.menu_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.menu_items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.menu_state.select(Some(i));
    }

    pub async fn refresh_list(&mut self) {
        let mut appimages = list_installed_appimages(false).await.unwrap_or_default();
        let mut global_appimages = list_installed_appimages(true).await.unwrap_or_default();
        appimages.append(&mut global_appimages);
        self.items = appimages;
        if self.items.is_empty() {
            self.list_state.select(None);
        } else if self.list_state.selected().is_none() {
            self.list_state.select(Some(0));
        }
    }

    pub async fn execute_install(&mut self) -> Result<()> {
        let path = std::path::PathBuf::from(self.install_form.appimage_path.trim());
        if !path.is_file() {
            self.app_logs.push(format!("Erro: O caminho fornecido não aponta para um arquivo válido: {:?}", path));
            anyhow::bail!("O caminho fornecido não existe ou não é um arquivo.");
        }
        self.app_logs.push(format!("Iniciando instalação de {:?}", path));

        // 1. Validation
        self.app_logs.push("Validando integridade (Magic Bytes)...".to_string());
        if let Err(e) = crate::core::validator::validate_appimage(&path).await {
            self.app_logs.push(format!("Falha na validação: {}", e));
            anyhow::bail!("AppImage corrompido ou inválido.");
        }
        self.app_logs.push("AppImage válido!".to_string());

        let target_dir = if self.install_form.global {
            if !self.is_root() {
                self.app_logs.push("A instalação global requer privilégios de root. Solicitando autenticação gráfica...".to_string());
                
                // Construct CLI arguments for elevated installation
                let mut args = vec![
                    "install".to_string(),
                    self.install_form.appimage_path.clone(),
                    "--global".to_string(),
                ];
                
                if !self.install_form.create_desktop {
                    args.push("--no-desktop".to_string());
                }
                
                if !self.install_form.target_dir.trim().is_empty() {
                    args.push("--target-dir".to_string());
                    args.push(self.install_form.target_dir.trim().to_string());
                }
                
                // Perform elevated operation without restarting TUI
                match crate::utils::elevation::run_elevated_with_pkexec(&args) {
                    Ok(_) => {
                        self.app_logs.push("Operação elevada concluída com sucesso.".to_string());
                        self.refresh_list().await;
                        self.focus = Focus::List;
                        self.install_form = InstallFormState::default();
                        return Ok(());
                    }
                    Err(e) => {
                        self.app_logs.push(format!("Falha na operação elevada: {}", e));
                        anyhow::bail!("Falha na autenticação ou execução root.");
                    }
                }
            }
            std::path::PathBuf::from("/opt/appimages")
        } else if self.install_form.target_dir.trim().is_empty() {
            dirs::data_local_dir().unwrap_or_default().join("appimages")
        } else {
            std::path::PathBuf::from(self.install_form.target_dir.trim())
        };

        let name = if self.install_form.custom_name.trim().is_empty() {
            path.file_stem().unwrap_or_default().to_string_lossy().to_string()
        } else {
            self.install_form.custom_name.trim().to_string()
        };
        
        let file_name = match path.file_name() {
            Some(name) => name,
            None => {
                self.app_logs.push("Erro: Não foi possível determinar o nome do arquivo.".to_string());
                anyhow::bail!("Nome de arquivo inválido.");
            }
        };
        let installed_path = target_dir.join(file_name);
        let desktop_dir = if self.install_form.global {
            std::path::PathBuf::from("/usr/share/applications")
        } else {
            dirs::data_local_dir().unwrap_or_default().join("applications")
        };

        // 2. Real Execution Phase
        self.app_logs.push("Iniciando cópias e permissões...".to_string());
        if !target_dir.exists() {
            tokio::fs::create_dir_all(&target_dir).await?;
        }

        let real_executor = crate::core::executor::RealExecutor;
        let installer = crate::core::installer::Installer::new(&real_executor);
        installer.install(&path, &target_dir).await?;

        if self.install_form.create_desktop {
            if !desktop_dir.exists() {
                tokio::fs::create_dir_all(&desktop_dir).await?;
            }
            crate::core::desktop::create_desktop_entry(&real_executor, &name, &installed_path, &desktop_dir).await?;
            self.app_logs.push(format!("Atalho .desktop criado para '{}'.", name));
        } else {
            self.app_logs.push("Criação de atalho .desktop ignorada (desmarcado).".to_string());
        }

        self.app_logs.push("Instalação concluída com sucesso.".to_string());
        self.refresh_list().await;
        self.focus = Focus::List;
        self.install_form = InstallFormState::default();

        Ok(())
    }

    pub async fn execute_remove(&mut self, info: &AppImageInfo) -> Result<()> {
        self.app_logs.push(format!("Iniciando remoção de {:?}", info.path));

        let desktop_name = info.path.file_stem().unwrap_or_default().to_string_lossy().to_string();
        
        let is_global = info.path.starts_with("/opt");
        if is_global && !self.is_root() {
            self.app_logs.push("A remoção de um AppImage global requer privilégios de root. Solicitando autenticação gráfica...".to_string());
            
            let args = vec![
                "remove".to_string(),
                info.name.clone(),
            ];

            match crate::utils::elevation::run_elevated_with_pkexec(&args) {
                Ok(_) => {
                    self.app_logs.push("Remoção elevada concluída com sucesso.".to_string());
                    self.refresh_list().await;
                    return Ok(());
                }
                Err(e) => {
                    self.app_logs.push(format!("Falha na remoção elevada: {}", e));
                    anyhow::bail!("Falha na autenticação ou execução root.");
                }
            }
        }

        let local_desktop_dir = dirs::data_local_dir().unwrap_or_default().join("applications");
        let global_desktop_dir = std::path::PathBuf::from("/usr/share/applications");

        // Real Execution
        self.app_logs.push("Executando remoção...".to_string());
        let real_executor = crate::core::executor::RealExecutor;
        let remover = crate::core::remover::Remover::new(&real_executor);
        remover.remove(&info.path).await?;

        // Try to remove corresponding .desktop file
        self.app_logs.push("Limpando atalho .desktop se existir...".to_string());
        
        // Always try local
        if let Err(e) = crate::core::desktop::remove_desktop_entry(&real_executor, &desktop_name, &local_desktop_dir).await {
            tracing::debug!("Erro ao remover atalho local: {:?}", e);
        }

        // Try global if it was global
        if is_global {
            if let Err(e) = crate::core::desktop::remove_desktop_entry(&real_executor, &desktop_name, &global_desktop_dir).await {
                tracing::warn!("Erro ao remover atalho global: {:?}", e);
            }
        }

        self.refresh_list().await;
        Ok(())
    }

    fn is_root(&self) -> bool {
        crate::utils::elevation::is_root()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_execute_remove_global_requires_root() {
        let mut app = App::new().await.unwrap();
        let info = AppImageInfo {
            name: "test.appimage".to_string(),
            path: PathBuf::from("/opt/appimages/test.appimage"),
            size_mb: 1.0,
            is_global: true,
        };

        // Certifique-se de que NÃO somos root (fake)
        unsafe { std::env::remove_var("AURA_FAKE_ROOT"); }
        
        let result = app.execute_remove(&info).await;
        // Agora o resultado é sucesso porque Mock de pkexec não falha por padrão
        if let Err(ref e) = result {
            println!("Error removing: {}", e);
        }
        assert!(result.is_ok());
        assert!(app.app_logs.iter().any(|l| l.contains("Solicitando autenticação gráfica")));
        assert!(app.app_logs.iter().any(|l| l.contains("Remoção elevada concluída com sucesso")));
    }

    #[tokio::test]
    async fn test_execute_install_global_requires_root() {
        let mut app = App::new().await.unwrap();
        app.install_form.global = true;
        app.install_form.appimage_path = "/tmp/fake.appimage".to_string();
        
        // Criar um arquivo fake para passar na verificação de is_file()
        let fake_path = PathBuf::from("/tmp/fake.appimage");
        let mut header = vec![0u8; 11];
        header[0..4].copy_from_slice(&[0x7f, 0x45, 0x4c, 0x46]);
        header[8..11].copy_from_slice(&[0x41, 0x49, 0x02]);
        tokio::fs::write(&fake_path, header).await.unwrap();

        // Certifique-se de que NÃO somos root (fake)
        unsafe { std::env::remove_var("AURA_FAKE_ROOT"); }

        let result = app.execute_install().await;
        // Agora o resultado é sucesso porque Mock de pkexec não falha por padrão
        if let Err(ref e) = result {
            println!("Error: {}", e);
        }
        assert!(result.is_ok());
        assert!(app.app_logs.iter().any(|l| l.contains("Solicitando autenticação gráfica")));
        assert!(app.app_logs.iter().any(|l| l.contains("Operação elevada concluída com sucesso")));
        
        tokio::fs::remove_file(&fake_path).await.unwrap();
    }

    #[tokio::test]
    async fn test_execute_install_global_pkexec_failure() {
        let mut app = App::new().await.unwrap();
        app.install_form.global = true;
        app.install_form.appimage_path = "/tmp/fake_fail.appimage".to_string();
        
        let fake_path = PathBuf::from("/tmp/fake_fail.appimage");
        let mut header = vec![0u8; 11];
        header[0..4].copy_from_slice(&[0x7f, 0x45, 0x4c, 0x46]);
        header[8..11].copy_from_slice(&[0x41, 0x49, 0x02]);
        tokio::fs::write(&fake_path, header).await.unwrap();

        unsafe { 
            std::env::remove_var("AURA_FAKE_ROOT"); 
            std::env::set_var("AURA_TEST_PKEXEC_FAIL", "1");
        }

        let result = app.execute_install().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Falha na autenticação"));
        
        unsafe { std::env::remove_var("AURA_TEST_PKEXEC_FAIL"); }
        tokio::fs::remove_file(&fake_path).await.unwrap();
    }
}
async fn run_app(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
    let mut app = App::new().await?;

    loop {
        terminal.draw(|f| {
            let area = f.area();

            // --- Guarda de tamanho mínimo ---
            // Se o terminal for menor que 80x24, renderiza uma tela de aviso
            // em vez da UI normal, evitando que widgets extrapolem os limites.
            const MIN_WIDTH: u16 = 80;
            const MIN_HEIGHT: u16 = 24;
            if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
                let warning = Paragraph::new(format!(
                    "Terminal muito pequeno!\n\n\
                    Tamanho mínimo: {}×{} colunas×linhas\n\
                    Tamanho atual:  {}×{}\n\n\
                    Redimensione a janela do terminal para continuar.",
                    MIN_WIDTH, MIN_HEIGHT, area.width, area.height
                ))
                .block(Block::default().borders(Borders::ALL).title(" Aura-Image "))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: false });
                f.render_widget(warning, area);
                return;
            }

            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
                .split(f.area());

            let menu_block = Block::default()
                .title("Menu")
                .borders(Borders::ALL)
                .border_style(if app.focus == Focus::Menu { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::DarkGray) });
            
            let menu_items_list: Vec<ListItem> = app.menu_items.iter().map(|i| {
                ListItem::new(i.as_str()).style(Style::default().fg(Color::White))
            }).collect();

            let menu_list = List::new(menu_items_list)
                .block(menu_block)
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ");

            f.render_stateful_widget(menu_list, main_chunks[0], &mut app.menu_state);

            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
                .split(main_chunks[1]);

            let active_menu = app.menu_items[app.menu_state.selected().unwrap_or(0)];

            if active_menu == MenuItem::Instalar {
                let form_block = Block::default()
                    .title("Instalar AppImage")
                    .borders(Borders::ALL)
                    .border_style(if app.focus == Focus::InstallForm { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::DarkGray) });

                let mut text = vec![];
                
                text.push(ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled("1. Caminho do AppImage:", Style::default()),
                ]));
                text.push(ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled(format!("> {}{}", app.install_form.appimage_path, if app.focus == Focus::InstallForm && app.install_form.step == 0 { "█" } else { "" }), if app.install_form.step == 0 && app.focus == Focus::InstallForm { Style::default().fg(Color::Yellow) } else { Style::default() })
                ]));
                text.push(ratatui::text::Line::from(""));

                text.push(ratatui::text::Line::from("2. Diretório de Destino (Vazio = ~/.local/share/appimages/):"));
                text.push(ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled(format!("> {}{}", app.install_form.target_dir, if app.focus == Focus::InstallForm && app.install_form.step == 1 { "█" } else { "" }), if app.install_form.step == 1 && app.focus == Focus::InstallForm { Style::default().fg(Color::Yellow) } else { Style::default() })
                ]));
                text.push(ratatui::text::Line::from(""));

                text.push(ratatui::text::Line::from("3. Nome do Atalho Customizado (Vazio = Nome do Arquivo):"));
                text.push(ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled(format!("> {}{}", app.install_form.custom_name, if app.focus == Focus::InstallForm && app.install_form.step == 2 { "█" } else { "" }), if app.install_form.step == 2 && app.focus == Focus::InstallForm { Style::default().fg(Color::Yellow) } else { Style::default() })
                ]));
                text.push(ratatui::text::Line::from(""));

                text.push(ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled(format!("[{}] Criar arquivo .desktop de atalho", if app.install_form.create_desktop { "X" } else { " " }), if app.install_form.step == 3 && app.focus == Focus::InstallForm { Style::default().fg(Color::Yellow) } else { Style::default() })
                ]));
                text.push(ratatui::text::Line::from(""));

                text.push(ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled(format!("[{}] Instalação Global (/opt, requer root)", if app.install_form.global { "X" } else { " " }), if app.install_form.step == 4 && app.focus == Focus::InstallForm { Style::default().fg(Color::Yellow) } else { Style::default() })
                ]));
                text.push(ratatui::text::Line::from(""));
                text.push(ratatui::text::Line::from(""));

                text.push(ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled("   [ EXECUTAR INSTALAÇÃO ]   ", if app.install_form.step == 5 && app.focus == Focus::InstallForm { Style::default().bg(Color::Yellow).fg(Color::Black) } else { Style::default().bg(Color::DarkGray) })
                ]));

                let form_p = Paragraph::new(text).block(form_block).wrap(ratatui::widgets::Wrap { trim: true });
                f.render_widget(form_p, right_chunks[0]);
            } else {
                let list_block = Block::default()
                    .title("AppImages Instalados")
                    .borders(Borders::ALL)
                    .border_style(if app.focus == Focus::List { 
                        if active_menu == MenuItem::Remover { Style::default().fg(Color::Red) } else { Style::default().fg(Color::Green) }
                    } else { Style::default().fg(Color::DarkGray) });

                let items: Vec<ListItem> = if app.items.is_empty() {
                    vec![ListItem::new("Nenhum AppImage instalado foi encontrado no sistema.\nColoque algum AppImage em ~/.local/share/appimages/ para testar.").style(Style::default().fg(Color::DarkGray))]
                } else {
                    app.items.iter().map(|i| {
                        let global_tag = if i.is_global { "[Global] " } else { "" };
                        let content = format!("{}{} ({:.2} MB)\n  Path: {}", global_tag, i.name, i.size_mb, i.path.display());
                        let mut style = Style::default().fg(Color::White);
                        if i.is_global {
                            style = style.fg(Color::Cyan);
                        }
                        ListItem::new(content).style(style)
                    }).collect()
                };

                let list = List::new(items)
                    .block(list_block)
                    .highlight_style(if app.focus == Focus::List {
                        if active_menu == MenuItem::Remover {
                            Style::default().bg(Color::Red).fg(Color::Black).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                        }
                    } else {
                        Style::default()
                    })
                    .highlight_symbol(if app.focus == Focus::List { ">> " } else { "   " });

                f.render_stateful_widget(list, right_chunks[0], &mut app.list_state);
            }

            let logs_block = Block::default().title(" Logs ").borders(Borders::ALL);

            // Calcula quantas linhas cabem no painel de logs (descontando as bordas).
            // Isso garante que nunca exibimos mais conteúdo do que o espaço disponível.
            let log_inner_height = right_chunks[1].height.saturating_sub(2) as usize;
            // Cada mensagem pode ter múltiplas linhas após o wrap; usamos uma margem
            // conservadora de 1 linha por mensagem para o cálculo inicial.
            let mut max_log_lines = log_inner_height.max(1);
            
            // Subtrair espaço do tutorial se ele estiver sendo exibido
            if !app.has_interacted {
                max_log_lines = max_log_lines.saturating_sub(2);
            }

            let display_logs = if app.app_logs.len() > max_log_lines {
                app.app_logs[app.app_logs.len() - max_log_lines..].to_vec()
            } else {
                app.app_logs.clone()
            };

            let logs_text = if !app.has_interacted {
                format!(
                    "TUTORIAL: ↑/↓ Tab=navegar  ←/→=trocar painel  Enter=confirmar  q=sair\n\n{}",
                    display_logs.join("\n")
                )
            } else {
                display_logs.join("\n")
            };

            // `Wrap { trim: false }` quebra linhas longas dentro dos limites da caixa
            // sem cortar espaços iniciais, mantendo o alinhamento das mensagens.
            let logs_p = Paragraph::new(logs_text)
                .block(logs_block)
                .wrap(Wrap { trim: false });
            f.render_widget(logs_p, right_chunks[1]);
        })?;

        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = event::read()? {
                app.has_interacted = true;
                match key.code {
                    KeyCode::Char(c) => {
                        if app.focus == Focus::InstallForm {
                            if c == ' ' {
                                if app.install_form.step == 3 {
                                    app.install_form.create_desktop = !app.install_form.create_desktop;
                                } else if app.install_form.step == 4 {
                                    app.install_form.global = !app.install_form.global;
                                }
                            } else if app.install_form.step < 3 {
                                match app.install_form.step {
                                    0 => app.install_form.appimage_path.push(c),
                                    1 => app.install_form.target_dir.push(c),
                                    2 => app.install_form.custom_name.push(c),
                                    _ => {}
                                }
                            }
                        } else if c == 'q' {
                            return Ok(());
                        }
                    }
                    KeyCode::Backspace => {
                        if app.focus == Focus::InstallForm {
                            match app.install_form.step {
                                0 => { app.install_form.appimage_path.pop(); }
                                1 => { app.install_form.target_dir.pop(); }
                                2 => { app.install_form.custom_name.pop(); }
                                _ => {}
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if app.focus == Focus::InstallForm {
                            if app.install_form.step == 5 {
                                if let Err(e) = app.execute_install().await {
                                    app.app_logs.push(format!("Falha na instalação: {:?}", e));
                                    tracing::error!("Falha na instalação: {:?}", e);
                                } else {
                                    app.menu_state.select(Some(0));
                                    app.focus = Focus::List;
                                }
                            } else if app.install_form.step == 3 {
                                app.install_form.create_desktop = !app.install_form.create_desktop;
                            } else if app.install_form.step == 4 {
                                app.install_form.global = !app.install_form.global;
                            } else {
                                app.install_form.step += 1;
                            }
                        } else if app.focus == Focus::List {
                            let active_menu = app.menu_items[app.menu_state.selected().unwrap_or(0)];
                            if active_menu == MenuItem::Remover
                                && let Some(selected_idx) = app.list_state.selected()
                                    && selected_idx < app.items.len() {
                                        let appimage_info = app.items[selected_idx].clone();
                                        if let Err(e) = app.execute_remove(&appimage_info).await {
                                            app.app_logs.push(format!("Falha na remoção: {:?}", e));
                                            tracing::error!("Falha na remoção: {:?}", e);
                                        } else {
                                            app.app_logs.push(format!("{} removido com sucesso.", appimage_info.name));
                                        }
                                    }
                        }
                    }
                    KeyCode::Right => {
                        if app.focus == Focus::Menu {
                            let active_menu = app.menu_items[app.menu_state.selected().unwrap_or(0)];
                            match active_menu {
                                MenuItem::Instalar => app.focus = Focus::InstallForm,
                                _ => app.focus = Focus::List,
                            }
                        }
                    }
                    KeyCode::Left => {
                        if app.focus == Focus::List || app.focus == Focus::InstallForm { app.focus = Focus::Menu; }
                    }
                    KeyCode::Down | KeyCode::Tab => {
                        match app.focus {
                            Focus::Menu => app.menu_next(),
                            Focus::List => app.next(),
                            Focus::InstallForm => app.install_form.step = (app.install_form.step + 1) % 6,
                        }
                    }
                    KeyCode::Up | KeyCode::BackTab => {
                        match app.focus {
                            Focus::Menu => app.menu_previous(),
                            Focus::List => app.previous(),
                            Focus::InstallForm => {
                                if app.install_form.step == 0 {
                                    app.install_form.step = 5;
                                } else {
                                    app.install_form.step -= 1;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
    }
}
