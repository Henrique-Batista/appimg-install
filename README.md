# Aura-Image 🌌

O **Aura-Image** é um gerenciador de AppImages moderno e performático para Linux, escrito em Rust. Ele oferece tanto uma interface visual rica em terminal (TUI) quanto comandos diretos via linha de comando (CLI) para instalar, remover e catalogar suas aplicações.

## ✨ Funcionalidades

- **Suporte Global**: Instale aplicações para todos os usuários do sistema em `/opt/appimages` com a flag `--global`.
- **Elevação de Privilégios Inteligente**: Solicita permissões de root automaticamente via `sudo` (CLI) ou `pkexec` (TUI) apenas quando necessário.
- **Segurança Reforçada**: Módulo de segurança dedicado que valida inputs e caminhos para prevenir ataques de execução de código arbitrário e manipulação de arquivos sensíveis.
- **Interface TUI Intuitiva**: Navegue por suas aplicações e identifique facilmente apps globais com a marcação visual `[Global]`.
- **CLI Poderosa**: Gerencie aplicações programaticamente ou via terminal direto com comandos como `list`, `install` e `remove`.
- **Segurança e Validação**: Verifica a integridade dos AppImages (Magic Bytes) antes de qualquer operação.
- **Integração com Desktop**: Cria automaticamente arquivos `.desktop` compatíveis com o padrão FreeDesktop e atualiza o banco de dados do sistema para reconhecimento imediato no KDE, GNOME e outros ambientes.
- **Modo Dry-Run**: Visualize o que será feito no sistema sem realizar alterações reais (disponível via CLI).
- **Logs Inteligentes**: Sistema de log duplo que salva detalhes em arquivo (`~/.cache/aura-image/aura-image.log`) e exibe apenas o necessário no terminal, evitando poluição visual na TUI.

## 🚀 Instalação

### Pré-requisitos
- **Rust** (Cargo) instalado em seu sistema Linux.

### Compilando do código fonte
1. Clone o repositório:
   ```bash
   git clone https://github.com/Henrique-Batista/appimg-install.git
   cd appimg-install
   ```
2. Compile o projeto:
   ```bash
   cargo build --release
   ```
3. O binário estará disponível em `target/release/aura-image`.

## 🛠️ Como Usar

### Interface TUI
Para abrir a interface interativa, basta executar sem argumentos:
```bash
./aura-image
```

### Linha de Comando (CLI)
- **Listar aplicações**:
  ```bash
  aura-image list
  ```
- **Instalar um AppImage (Local)**:
  ```bash
  aura-image install /caminho/para/seu.AppImage
  ```
- **Instalar um AppImage (Global)**:
  ```bash
  aura-image install --global /caminho/para/seu.AppImage
  ```
- **Remover uma aplicação**:
  ```bash
  aura-image remove nome-do-app
  ```
- **Ver ajuda**:
  ```bash
  aura-image --help
  ```

## 🏗️ Arquitetura

O projeto segue uma separação estrita de responsabilidades:
- `src/core/`: Lógica de domínio agnóstica à interface (Instalação, Remoção, Validação, Scanner).
- `src/ui/`: Implementações visuais (TUI com Ratatui e CLI com Clap).
- `src/utils/`: Ferramentas utilitárias (Logger, Helpers).

## 🧪 Testes

O Aura-Image conta com uma suíte de testes unitários para garantir a estabilidade do núcleo. Para rodar os testes:
```bash
cargo test
```

## 📝 Licença

Este projeto está sob a licença MIT.
