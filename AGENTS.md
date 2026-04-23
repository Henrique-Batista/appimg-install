# Aura-Image: Guidelines para Agentes e Desenvolvedores

Este documento serve como um guia de arquitetura e boas práticas para qualquer agente autônomo de IA ou desenvolvedor humano que for contribuir para o repositório **Aura-Image**.

## 1. Visão Geral da Arquitetura

O Aura-Image é uma ferramenta de gerenciamento de AppImages para Linux escrita em Rust. O projeto é estritamente separado em duas camadas principais:

- `src/core/`: Toda a lógica agnóstica a interfaces. Instalações, deleções, validações, criação de arquivos desktop, etc.
- `src/ui/`: Apenas entrada e saída visual de dados, seja pelo terminal direto (CLI) ou pela Interface de Usuário em Terminal (TUI).
- `src/utils/`: Ferramentas transversais como configuração de logging, hash/checksum helpers, etc.

## 2. Regra de Ouro: O Trait Executor e o Dry-Run

**NUNCA realize operações I/O destrutivas diretamente com `std::fs` ou `tokio::fs` na lógica de instalação.**

Todas as funções do `src/core/` que alteram o sistema (criar pastas, copiar arquivos, setar permissões) **DEVEM** receber uma injeção de dependência do `Trait Executor` (`core::executor::Executor`). 
- Isso garante que a flag `--dry-run` (`DryRunExecutor`) funcione impecavelmente para relatar intenções ao usuário sem modificar o disco.
- Ao adicionar novas interações com o disco, adicione um método abstrato no `Executor` usando `#[async_trait::async_trait]` e o implemente no `RealExecutor` e no `DryRunExecutor`.

## 3. Gestão de Erros

O projeto utiliza o padrão ouro de errors do ecossistema Rust:
- `thiserror`: Use-o extensivamente no `src/core/` para definir Enumerações de erro tipadas para falhas granulares (Ex: falha ao extrair icone, binário não encontrado, erro no header ELF).
- `anyhow`: Use-o na camada `src/ui/` e em `main.rs` para capturar os erros propagados e adicionar o contexto necessário para o usuário final com mensagens ricas. 
- *NUNCA use `.unwrap()` ou `.expect()` em código de produção, a menos que seja matematicamente provado impossível de falhar.*

## 4. Logging Estruturado

A ferramenta **NÃO** usa `println!` para depuração ou saída de status. Todo log deve passar pelo crate `tracing`.
- `tracing::trace!`/`debug!`: Para detalhes granulares de caminhos lidos, outputs parciais.
- `tracing::info!`: Para passos principais completados.
- `tracing::warn!`/`error!`: Para capturas de falha e degradações não fatais.

O `init_logger` redireciona logs para `~/.cache/aura-image/aura-image.log`. Na CLI, os logs aparecem no terminal (`stderr`), mas na TUI eles são silenciados para não corromper a interface gráfica.

## 5. UI: Ratatui e Clap

- A CLI é orientada a *Commands* usando o crate `clap` (derive). Qualquer nova flag ou subcomando deve estar documentado em `ui::cli_args::Cli` com *doc comments* (`///`).
- A interface TUI utiliza `ratatui` e `crossterm`. Ao alterar os gráficos da TUI, lembre-se das restrições de concorrência (`Send`/`Sync`) na propagação de erros customizados para o `anyhow`, sendo recomendado retornar apenas `std::io::Result<()>` da função base de desenho para simplificar e envolver esse resutado em um `anyhow` global se necessário.

## 6. Padronizações de Código

- Async runtime padrão é `tokio`. Use bibliotecas async-first nativas onde possível.
- Formatação padrão estrita via `cargo fmt`.
- Zero tolerância para warnings no Clippy: Qualquer PR deve passar em `cargo clippy`. (Exceções de `#[allow(dead_code)]` só no período de scaffold).

## 7. Atualizações Recentes do Projeto (Histórico)

Qualquer alteração significativa na estrutura deve ser relatada abaixo para fins de contexto de longo prazo:

- **Validação por Magic Bytes (`validator.rs`)**: Foi implementada a leitura assíncrona dos primeiros 11 bytes (via `tokio::io::AsyncReadExt`) para atestar cabecalhos ELF e assinaturas `AI` de AppImages válidos. Erros são lançados via `thiserror`.
- **Scanner de Diretórios (`scanner.rs`)**: Adicionado o módulo capaz de listar assincronamente as extensões `.appimage` em diretórios do sistema e do usuário.
- **Removedor e CLI (`remover.rs` & `main.rs`)**: Implementada a funcionalidade de desinstalação pelo domínio (`remover.rs`) incluindo os atalhos `.desktop`. Agora também unificada e 100% funcional pela Interface de Linha de Comando usando `list` e `remove`.
- **Estado e Foco na TUI (`tui_render.rs`)**: A interface `ratatui` foi transformada de um painel estático em um layout dividido (Menu Esquerdo e Conteúdo Direito). O controle de estado agora gerencia o `Focus` atual (`Menu` vs `List`), ativando bordas reativas e navegação por teclado.
- **Persistência e Desktop Database (`desktop.rs`)**: Melhorada a criação de arquivos `.desktop` (padrão FreeDesktop) e adicionada a atualização automática do banco de dados de aplicações via `update-desktop-database` para reconhecimento imediato no KDE/GNOME.
- **Arquitetura de Logs (`logger.rs`)**: Implementado sistema de log duplo (Arquivo + Terminal) com supressão inteligente para a TUI e suporte a flags `--quiet`/`--verbose`.
- **Suíte de Testes e UX (`tui_render.rs`)**: Implementada cobertura de testes no `core` e ajustes de UX na TUI, como Word Wrap nos logs, detecção de tamanho mínimo de terminal e remoção de passos redundantes de simulação (Dry-Run) antes da execução real.
