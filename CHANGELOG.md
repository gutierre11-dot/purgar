# Changelog

Todas as mudanças notáveis deste projeto são documentadas neste arquivo.

O formato é baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/)
e este projeto adere ao [Versionamento Semântico](https://semver.org/lang/pt-BR/).

## [Não lançado]

## [0.2.0] - 2026-11-13

Primeira versão publicável do purgar.

### Added

- TUI completa em ratatui, em português do Brasil, com tela inicial de análise,
  detalhamento por categoria, progresso, resultado, estatísticas de limpeza e
  configurações.
- Gerenciador de inicialização para ativar, desativar e excluir programas
  iniciados com a sessão.
- Desinstalador de programas RPM, Flatpak e Snap, com proteção de pacotes
  essenciais e opção de remover dados de aplicativos Flatpak.
- CLI com os subcomandos `scan`, `clean`, `lista` e `config`.
- Flags de CLI: `--json`, `--detalhes`, `--categoria ID`, `--todos-basicos`,
  `--sim`/`--yes`, `--dry-run`, `--lixeira`, `--sem-lixeira`, `--version`/`-V`
  e `--help`/`-h`.
- Categorias novas: `cache_navegadores` completa (Firefox, Chrome, Chromium,
  Brave, Edge, Vivaldi, Opera, Zen, LibreWolf e Floorp, incluindo `~/.cache`,
  perfis e Flatpak), `apt_cache`, `flatpak_cache`, `flatpak_unused`,
  `docker_cache`, `vscode_cache`, `jetbrains_cache` e `custom`.
- Detecção do gerenciador de pacotes (dnf, apt, pacman e zypper) e de
  ferramentas opcionais, com categorias condicionais.
- Análise de categorias que exigem administrador com indicação explícita de
  `requer_admin` na tabela e no JSON do `scan`.
- Configuração ampliada com as chaves `usar_lixeira`, `caminhos`, `dias_tmp`,
  `dias_logs`, `historico_max` e `apagar_dados_flatpak`.
- Histórico de limpezas em `~/.local/share/purgar/historico.jsonl`, em JSON
  Lines, com os campos `ts`, `categoria`, `bytes`, `movidos` e `erros`.
- Man page `purgar.1` e completions de bash, zsh e fish.
- Empacotamento para Fedora/RHEL (`packaging/purgar.spec`) e Arch Linux
  (`packaging/PKGBUILD`).
- CI no GitHub Actions com formatação, clippy, testes e build de release.
- Workflow de release para tags `v*`, com tarballs para `x86_64` e `aarch64` e
  checksums SHA-256.
- Testes unitários e de integração para as rotinas de filesystem, configuração,
  validação de caminhos e lixeira.
- Documentação: README reescrito, `SECURITY.md`, `CONTRIBUTING.md` e este
  changelog.

### Changed

- Projeto atualizado para a edição 2024 do Rust, com MSRV 1.95 (exigida por
  `sysinfo 0.40.3`).
- `purgar` sem argumentos abre a TUI; em ambiente sem TTY, orienta a usar
  `purgar scan` ou `purgar --help` em vez de falhar.
- Códigos de saída do CLI padronizados: `0` sucesso, `1` falhas na limpeza,
  `2` uso inválido ou operação recusada e `130` cancelamento.
- Categorias somente de análise, como `downloads`, ficam fora do total do
  `scan` e não podem ser limpas.
- O histórico separa o espaço efetivamente liberado do espaço movido para a
  lixeira e respeita o limite de `historico_max` registros.
- Reaproveitamento da medição feita no `scan` durante a limpeza, reduzindo I/O.
- Tela de progresso com cancelamento por `Esc`.
- Comandos externos de listagem do desinstalador passaram a rodar fora da
  thread da interface.

### Fixed

- A TUI restaura o terminal mesmo em erro ou panic, incluindo falhas ao
  inicializar o terminal.
- `Ctrl+C` encerra a TUI corretamente.
- Corrigido panic por corte de string UTF-8 ao truncar entradas de autostart.
- A tela de progresso não fica presa quando a thread de trabalho termina de
  forma inesperada.
- Corrigida a limpeza administrativa de `/tmp` e `/var/tmp` que falhava por
  combinar `find -prune` com `-delete`.
- A limpeza de `/tmp` como usuário remove apenas arquivos e symlinks antigos,
  nunca diretórios com conteúdo, e respeita as exclusões documentadas.
- A categoria `lixeira` não move novamente itens que já estão na lixeira.
- Caches de navegadores baseados em Chromium deixaram de ficar órfãos e passaram
  a ser cobertos por `cache_navegadores`.
- Categorias que exigem administrador deixam de aparecer como "nada a limpar".
- O autostart edita `Hidden=` dentro do grupo `[Desktop Entry]`.
- Corrigida a paginação e a rolagem das listas da TUI.
- O desinstalador reconhece o gerenciador de pacotes disponível e protege
  pacotes essenciais.

### Security

- Validação de caminhos personalizados, com rejeição de `/`, `/usr`, `/etc`,
  `/home`, `/root`, `/var`, `/boot`, `/dev`, `/proc`, `/sys`, `/run`, `/bin`,
  `/lib*`, `/opt`, `/srv`, `/media`, `/mnt`, `/tmp`, `/var/tmp`, da própria home
  e de caminhos rasos demais. O caminho precisa existir.
- Modo lixeira alinhado à freedesktop.org Trash spec, com `Path=` em
  URL-encoding e `.trashinfo` gravado antes do move; quando o move não é
  possível, usa cópia seguida de remoção verificada e nunca apaga
  silenciosamente.
- Caminhos personalizados passam por validação também no momento da limpeza,
  não apenas na interface.
- Limpeza de `/tmp` como usuário preserva `.X11-unix`, `.ICE-unix`, `.XIM-unix`,
  `.font-unix`, `.Test-unix`, `.dbus`, `systemd-private-*` e `ssh-*`.
- Bloqueio da limpeza de cache de navegadores enquanto houver navegador aberto,
  tanto na TUI quanto no CLI.
- CLI exige confirmação interativa ou `--sim` explícito em ambiente não
  interativo.
- Ações administrativas usam `pkexec`/polkit e falham com mensagem clara quando
  não há sessão gráfica para autenticação.
- Varredura interna nunca segue symlinks; diretórios-raiz simbólicos são
  resolvidos para validação.

[0.2.0]: https://github.com/gutierre11-dot/purgar/releases/tag/v0.2.0
[Não lançado]: https://github.com/gutierre11-dot/purgar/commits/main
