# purgar

[![Rust](https://img.shields.io/badge/Rust-1.99%20Edition%202024-orange?logo=rust)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-Linux-blue)](#)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

Limpador de cache e arquivos temporários para Linux com **TUI premium em Rust** — inspirado no CCleaner, 100% em português do Brasil.

## Recursos

- **Painel inicial** com análise em tempo real do espaço recuperável por categoria
- **Limpeza básica**: cache de aplicativos, navegadores, lixeira, miniaturas, caches de desenvolvimento (npm, pip, Cargo, Go, Gradle, Maven, Bun, Deno), logs de usuário
- **Limpeza avançada**: logs do sistema, journal do systemd, cache de pacotes DNF, temporários antigos em `/tmp` e `/var/tmp`, coredumps
- **Confirmação antes de cada limpeza** com explicação resumida do que será afetado
- **Gerenciador de inicialização** (autostart): ativar/desativar/excluir programas
- **Desinstalador** de programas RPM, Flatpak e Snap com filtro de busca
- **Estatísticas** de espaço liberado historicamente
- **Caminhos personalizados** e modo lixeira opcional (nada é apagado definitivamente)
- Elevação via `pkexec` (polkit) apenas quando necessário

## Instalação

### A partir do código-fonte

```bash
git clone https://github.com/gutierre11-dot/purgar.git
cd purgar
cargo build --release
# binário em target/release/purgar
```

Recomendado copiar para o PATH:

```bash
mkdir -p ~/.local/bin
cp target/release/purgar ~/.local/bin/
```

### Dependências

- Rust 1.85+ (compilação)
- `pkexec` (polkit) — apenas para categorias que exigem root
- Fedora/RHEL, Debian/Ubuntu e derivados (caminhos podem variar em outras distros)

## Uso

```bash
purgar              # abre a TUI
purgar scan         # analisa todas as categorias (modo CLI)
purgar clean <id>   # limpa uma categoria (modo CLI)
purgar lista        # lista categorias disponíveis
purgar --help       # ajuda
```

## Segurança

- Nunca segue symlinks ao medir ou remover arquivos
- Preserva sockets e diretórios ativos em `/tmp` (`.X11-unix`, `systemd-private-*`, `ssh-*`)
- Bloqueia limpeza de cache de navegadores enquanto houver navegador aberto
- Modo lixeira opcional nas configurações

## Configuração

`~/.config/purgar/config.json` — preferências e caminhos personalizados.
Histórico de limpezas em `~/.local/share/purgar/historico.jsonl`.

## Stack

Rust • [ratatui](https://github.com/ratatui/ratatui) • [crossterm](https://github.com/crossterm-rs/crossterm) • walkdir • rayon • serde
