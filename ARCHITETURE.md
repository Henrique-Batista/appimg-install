# Aura-Image: Arquitetura e Design do Sistema

Este documento descreve a arquitetura base do **Aura-Image**, um gerenciador de AppImages focado em segurança, previsibilidade (dry-run) e usabilidade (TUI e CLI).

## 1. Visão Macro

O sistema é construído sobre o ecossistema assíncrono do Rust (`tokio`), e sua responsabilidade é atuar como uma ponte fácil entre arquivos `*.AppImage` soltos no disco e a integração completa com os ambientes desktop (FreeDesktop.org).

Ele está estritamente particionado em camadas lógicas para impedir que o código visual (UI) se misture com a lógica de sistema (Core).

## 2. Camada `src/core/` (Lógica de Domínio)

Nenhum arquivo desta camada tem permissão de desenhar telas ou capturar inputs do terminal diretamente.

### `executor.rs`
Abstração fundamental do sistema baseada no padrão de injeção de dependência via Traits.
Contém a interface `Executor` (`async-trait`), implementada por:
- `RealExecutor`: Realiza cópias assíncronas de fato via `tokio::fs`.
- `DryRunExecutor`: Simplesmente emite logs informativos (`tracing`) fingindo as operações.
Todo método destrutivo em outros módulos recebe `&dyn Executor`.

### `validator.rs`
Responsável por validar a integridade de um AppImage antes da instalação.
Atualmente, garante a assinatura do arquivo através da leitura de seus "Magic Bytes" primários sem carregar o binário inteiro em memória (leitura dos primeiros 11 bytes para encontrar `0x7F 'E' 'L' 'F'` e `A I \x01/\x02`).

### `scanner.rs`
Varre diretórios locais (`~/.local/share/appimages/`) e globais (`/opt/appimages/`) assincronamente para catalogar os pacotes já instalados.

### `installer.rs`, `remover.rs` & `desktop.rs`
Scripts abstratos de instalação (cópia e permissões), remoção de arquivos, e geração/remoção de metadados (`.desktop`), sempre usando estritamente o `Executor` para realizar suas ações em disco.

### `AppImageInfo`
Estrutura central de dados que rastreia metadados das aplicações, incluindo se a instalação é local ou global (via flag `global`), permitindo que a UI aplique decorações visuais e o sistema decida os caminhos de remoção.

## 3. Camada `src/ui/` (Interface e Interação)

### `cli_args.rs`
Construído com `clap` (feature `derive`), encapsula todos os parâmetros e documentações para os subcomandos de linha de comando direta: `install`, `remove`, `list` e as flags globais.

### `tui_render.rs`
A interface rica em terminal. Construída com `ratatui` e gerida por um loop de eventos `crossterm`.
A TUI utiliza o conceito de **Estados Focais** (enum `Focus`) para decidir se as setas do teclado controlam o *Menu Lateral* (com abas lógicas) ou a *Lista de Conteúdo* (trazendo os dados do scanner). O padrão de projeto aplicado aqui é o Loop de Redesenho (Draw Loop), mantendo uma taxa de atualização reativa (100ms/poll).

## 4. Gestão de Erros, Logs e Segurança (`src/utils/`)

### `logger.rs`
O sistema utiliza o crate `tracing` para logs estruturados com comportamento inteligente:
- **Sempre**: Grava logs detalhados em `~/.cache/aura-image/aura-image.log`.
- **CLI**: Exibe logs no terminal (stderr) por padrão, respeitando as flags `--quiet` e `--verbose`.
- **TUI**: Silencia a saída no terminal para evitar corrupção visual dos gráficos do Ratatui.

### `elevation.rs`
Gerencia a elevação de privilégios necessária para operações globais.
- Utiliza `sudo` para comandos via CLI.
- Utiliza `pkexec` (Polkit) para a TUI, garantindo integração com prompts de senha gráficos.
- Assegura que o processo pai (TUI) continue rodando enquanto o comando elevado é executado em um sub-processo silencioso.

### `security.rs`
Módulo de segurança crítico que atua como firewall de sanidade para operações elevadas.
- Valida nomes de aplicativos contra caracteres de escape de caminho (`../`).
- Restringe caminhos de instalação globais apenas a diretórios autorizados (`/opt/appimages`, `/usr/share/applications`).
- Previne leitura de arquivos sensíveis do sistema durante a validação de origem.

## 5. Testes Automatizados

O projeto mantém uma suíte de testes unitários e de integração nos módulos do `src/core/`. Os testes cobrem:
- Validação de Magic Bytes (casos de sucesso e falha).
- Operações de I/O via `RealExecutor` e `DryRunExecutor` usando diretórios temporários.
- Lógica de geração e normalização de nomes de arquivos `.desktop`.
- Escaneamento de diretórios e cálculo de metadados.
