# purgar

Limpeza de cache e arquivos temporários para Linux, com TUI premium em Rust.
Inspirado no CCleaner, 100% em português do Brasil.

## Recursos

- **Painel inicial** com análise em tempo real do espaço recuperável por categoria
- **Limpeza básica**: cache de aplicativos, navegadores, lixeira, miniaturas, caches de desenvolvimento (npm, pip, Cargo, Go, Gradle, Maven...), logs de usuário
- **Limpeza avançada**: logs do sistema, journal do systemd, cache de pacotes, temporários antigos em `/tmp` e `/var/tmp`, coredumps
- **Confirmação antes de cada limpeza** com explicação do que será afetado
- **Gerenciador de inicialização** (autostart): ativar/desativar/excluir programas
- **Desinstalador** de programas RPM, Flatpak e Snap com filtro de busca
- **Estatísticas** de espaço liberado historicamente
- **Caminhos personalizados** e opção de mover para a lixeira em vez de apagar
- Elevação via `pkexec` (polkit) apenas quando necessário

## Uso

```bash
purgar              # abre a TUI
purgar scan         # analisa todas as categorias (CLI)
purgar clean <id>   # limpa uma categoria (CLI)
purgar lista        # lista categorias
purgar --help       # ajuda
```

## Compilar

```bash
cargo build --release
# binário em target/release/purgar
```

## Segurança

- Nunca segue symlinks ao medir/remover
- Preserva sockets e diretórios ativos em `/tmp` (`.X11-unix`, `systemd-private-*`, `ssh-*`)
- Bloqueia limpeza de cache de navegadores enquanto houver navegador aberto
- Modo lixeira opcional (configurações) para não apagar definitivamente

## Configuração

`~/.config/purgar/config.json` — preferências e caminhos personalizados.
Histórico de limpezas em `~/.local/share/purgar/historico.jsonl`.
