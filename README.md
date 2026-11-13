# purgar

[![CI](https://github.com/gutierre11-dot/purgar/actions/workflows/ci.yml/badge.svg)](https://github.com/gutierre11-dot/purgar/actions/workflows/ci.yml)
[![Licença: MIT](https://img.shields.io/badge/licen%C3%A7a-MIT-green.svg)](LICENSE)
[![Plataforma: Linux](https://img.shields.io/badge/plataforma-Linux-blue.svg)](https://www.kernel.org)
[![Rust 1.95+](https://img.shields.io/badge/rust-1.95%2B-orange.svg)](https://www.rust-lang.org)

**purgar** é um limpador de cache e arquivos temporários para Linux, escrito em
Rust, com interface de terminal (TUI) e interface de linha de comando (CLI).
Foi feito para o dia a dia de quem usa Fedora/RHEL, Debian/Ubuntu e Arch, com
detecção do gerenciador de pacotes disponível, e é totalmente em português do
Brasil.

A ferramenta encontra cache de aplicativos e navegadores, lixeira, miniaturas,
caches de desenvolvimento, logs antigos, temporários, coredumps, caches de
contêineres e muito mais. Você analisa primeiro, escolhe o que limpar e só então
confirma — nada é apagado sem a sua decisão.

## Recursos

- **TUI completa** em [ratatui](https://github.com/ratatui/ratatui): tela
  inicial com análise por categoria, detalhamento, progresso, resultado,
  estatísticas, configurações, gerenciador de inicialização e desinstalador de
  programas.
- **CLI scriptável**: `scan`, `clean`, `lista` e `config`, com `--json`,
  `--detalhes`, `--dry-run`, `--todos-basicos` e códigos de saída estáveis.
- **20 categorias** de limpeza, incluindo categorias condicionais que só
  aparecem quando a ferramenta correspondente existe.
- **Detecção de gerenciador de pacotes**: dnf, apt, pacman e zypper.
- **Modo lixeira** opcional, seguindo a especificação
  [freedesktop.org Trash](https://specifications.freedesktop.org/trash-spec/latest/).
- **Confirmação obrigatória** no CLI e na TUI; `--dry-run` simula a limpeza.
- **Ações administrativas** via `pkexec`/polkit, apenas quando necessário.
- **Validação de caminhos personalizados** com bloqueio de diretórios de
  sistema e de caminhos rasos demais.
- **Proteção de pacotes essenciais** no desinstalador (RPM, Flatpak e Snap).
- **Histórico** de limpezas em JSON Lines e estatísticas de espaço liberado.
- **Terminal sempre restaurado**, inclusive em erro ou panic.

## Preview

Captura ilustrativa da tela inicial da TUI:

```text
+ purgar ---------------------------------------------------------- v0.2.0 +
| Análise concluída: 1,4 GB recuperáveis, 13 categorias                    |
|                                                                          |
|   [x] Cache de aplicativos        cache_apps        812,4 MB             |
|   [x] Cache de navegadores        cache_navegadores 431,9 MB             |
|   [x] Lixeira                     lixeira            98,1 MB             |
|   [x] Miniaturas                  miniaturas         12,6 MB             |
|   [x] Caches de desenvolvimento   cache_dev          84,3 MB             |
|   [x] Logs do usuário             logs_usuario        3,2 MB             |
|   [ ] Downloads (somente análise) downloads          8,7 GB              |
|   [ ] Logs do sistema             logs_sistema     requer admin          |
|   [ ] Journal do sistema          journal_sistema  requer admin          |
|                                                                          |
|   Selecionado: 812,4 MB                                                  |
|   Espaço marcar · Enter detalhes · Esc voltar · Ctrl+C sair              |
+--------------------------------------------------------------------------+
```

Além da tela inicial, a TUI oferece:

- **Gerenciador de inicialização** — ativar, desativar ou excluir programas
  iniciados com a sessão.
- **Desinstalador de programas** — RPM, Flatpak e Snap, com proteção de pacotes
  essenciais.
- **Estatísticas de limpeza** — espaço liberado ao longo do histórico.
- **Configurações** — modo lixeira, retenção de temporários e logs, limite do
  histórico, caminhos personalizados e remoção de dados de Flatpak.

## Requisitos

- **Linux** (Fedora/RHEL, Debian/Ubuntu, Arch e derivados).
- **Rust 1.95 ou superior** para compilar a partir do código-fonte (MSRV
  definida por `sysinfo 0.40.3`).
- **`pkexec`** (polkit) para as categorias que exigem administrador.
- **systemd** (`journalctl`) para as categorias de journal.
- Ferramentas externas opcionais: `dnf`, `apt`, `pacman`, `zypper`, `docker`,
  `flatpak`, `snap`, `code`, IDEs JetBrains. As categorias que dependem delas só
  aparecem quando a ferramenta está disponível.

## Instalação

### A partir do código-fonte

```bash
git clone https://github.com/gutierre11-dot/purgar.git
cd purgar
cargo build --release
# binário em target/release/purgar
```

Para instalar no `PATH` de usuário:

```bash
cargo install --path .
# binário em ~/.cargo/bin/purgar
```

### Fedora/RHEL (RPM)

O repositório inclui um spec em `packaging/purgar.spec`:

```bash
sudo dnf install rpmdevtools rust-packaging
spectool -g -R packaging/purgar.spec
rpmbuild -ba packaging/purgar.spec
sudo dnf install ~/rpmbuild/RPMS/*/purgar-*.rpm
```

O pacote instala o binário, a man page, as completions de bash/zsh/fish, a
licença e o changelog.

### Arch Linux (PKGBUILD)

O repositório inclui um `packaging/PKGBUILD`:

```bash
git clone https://github.com/gutierre11-dot/purgar.git
cd purgar/packaging
makepkg -si
```

### Binários pré-compilados

Cada release publica tarballs para `x86_64-unknown-linux-gnu` e
`aarch64-unknown-linux-gnu`, com checksums SHA-256, na página de
[releases](https://github.com/gutierre11-dot/purgar/releases). Extraia o
arquivo e copie `purgar` para um diretório do seu `PATH`.

### Man page e completions avulsas

Quem instala manualmente pode copiar os arquivos auxiliares:

```bash
sudo install -Dm644 packaging/purgar.1 /usr/local/share/man/man1/purgar.1
sudo install -Dm644 completions/purgar.bash /usr/local/share/bash-completion/completions/purgar
sudo install -Dm644 completions/purgar.zsh /usr/local/share/zsh/site-functions/_purgar
sudo install -Dm644 completions/purgar.fish /usr/local/share/fish/vendor_completions.d/purgar.fish
```

## Uso (CLI)

```text
purgar
purgar scan [--json] [--detalhes] [--categoria ID]
purgar clean <ID>... | --todos-basicos [--sim|--yes] [--dry-run] [--json] [--lixeira|--sem-lixeira]
purgar lista [--json]
purgar config [--json]
purgar --version | -V
purgar --help | -h
```

Sem argumentos, `purgar` abre a TUI e exige um terminal interativo. Em ambiente
não interativo (sem TTY), o programa orienta a usar `purgar scan` ou
`purgar --help`, em vez de falhar.

### `purgar scan`

Analisa as categorias e mostra uma tabela com categoria, id, tamanho, número de
itens e marcação das categorias que exigem administrador.

```bash
purgar scan
purgar scan --categoria cache_apps
purgar scan --detalhes --categoria cache_navegadores
purgar scan --json
```

- `--detalhes` lista o detalhamento por alvo de cada categoria.
- `--categoria ID` filtra por uma categoria.
- `--json` emite um array de objetos com os campos `id`, `nome`, `nivel`,
  `bytes`, `itens`, `requer_admin` e `detalhes`.

Exemplo de saída:

```text
Categoria                     ID                     Tamanho    Itens  Admin
Cache de aplicativos          cache_apps             812,4 MB   15234
Cache de navegadores          cache_navegadores      431,9 MB    8120
Lixeira                       lixeira                 98,1 MB      42
Logs do sistema               logs_sistema                  -       -  sim
```

### `purgar clean`

Limpa uma ou mais categorias informadas por id, ou todas as categorias de nível
básico com `--todos-basicos`.

- Em TTY, pede confirmação interativa (`s/N`).
- Em ambiente não interativo, exige `--sim` (ou `--yes`).
- `--dry-run` apenas analisa e lista o que seria removido, sem apagar nada.
- `--json` emite o resultado em JSON.
- `--lixeira` e `--sem-lixeira` sobrescrevem `usar_lixeira` apenas para aquela
  execução.
- Categorias somente de análise, como `downloads`, não são limpas.

```bash
purgar clean lixeira
purgar clean cache_apps cache_navegadores
purgar clean cache_dev --dry-run
purgar clean --todos-basicos --sim
purgar clean flatpak_cache --sem-lixeira --json
```

### `purgar lista`

Lista todas as categorias disponíveis, com id, nome, nível e se exigem
administrador.

```bash
purgar lista
purgar lista --json
```

### `purgar config`

Mostra os caminhos dos arquivos de configuração e de histórico, os valores
atuais e quantos registros há no histórico.

```bash
purgar config
purgar config --json
```

### Ajuda e versão

```bash
purgar --help
purgar --version
```

### Códigos de saída

| Código | Significado |
|---|---|
| `0` | Sucesso. |
| `1` | Falhas ou erros durante a limpeza. |
| `2` | Uso inválido, argumentos incorretos ou operação recusada. |
| `130` | Limpeza não confirmada ou cancelada pelo usuário. |

## Categorias

Os ids abaixo são os valores aceitos por `purgar clean`, `--categoria` e pela
TUI. Categorias avançadas com a marcação **admin** usam `pkexec`/polkit.

| ID | Nível | O que remove | Admin |
|---|---|---|---|
| `cache_apps` | Básico | Esvazia `~/.cache`, exceto navegadores, caches de desenvolvimento, miniaturas e Flatpak/DNF. | Não |
| `cache_navegadores` | Básico | Cache de Firefox, Chrome, Chromium, Brave, Edge, Vivaldi, Opera, Zen, LibreWolf e Floorp, incluindo `~/.cache`, perfis e Flatpak. | Não |
| `lixeira` | Básico | Lixeira do usuário. | Não |
| `miniaturas` | Básico | Cache de miniaturas. | Não |
| `cache_dev` | Básico | Caches de npm, pip, Yarn, Composer, Cargo, Go, Gradle, Maven, Deno e Bun. | Não |
| `logs_usuario` | Básico | Journal do usuário (`journalctl --user`), mantendo 7 dias. | Não |
| `downloads` | Básico | Pasta Downloads — **somente análise**, nada é removido. | Não |
| `logs_sistema` | Avançado | Logs com mais de 30 dias em `/var/log`. | Sim |
| `journal_sistema` | Avançado | Journal do sistema, mantendo 7 dias. | Sim |
| `cache_sistema` | Avançado | Cache do gerenciador de pacotes detectado: dnf, apt, pacman ou zypper. | Sim |
| `tmp_antigos` | Avançado | Arquivos antigos em `/tmp` e `/var/tmp` com mais de 7 dias, preservando sockets e diretórios ativos. | Sim |
| `dnf_cache` | Avançado | Cache do DNF. | Sim |
| `apt_cache` | Avançado | Cache do APT (aparece em Debian/Ubuntu). | Sim |
| `flatpak_cache` | Avançado | Cache do Flatpak. | Não |
| `flatpak_unused` | Avançado | Runtimes do Flatpak não utilizados. | Não |
| `coredumps` | Avançado | Coredumps do sistema. | Sim |
| `docker_cache` | Avançado | Cache do Docker (`docker system prune -f`). | Sim |
| `vscode_cache` | Avançado | Cache do VS Code. | Não |
| `jetbrains_cache` | Avançado | Caches das IDEs JetBrains. | Não |
| `custom` | Avançado | Caminhos personalizados, somente quando configurados. | Não |

Categorias condicionais (`dnf_cache`, `apt_cache`, `flatpak_cache`,
`flatpak_unused`, `coredumps`, `docker_cache`, `vscode_cache`,
`jetbrains_cache` e `custom`) só aparecem quando a ferramenta correspondente
está instalada, o diretório existe ou há caminhos personalizados configurados.
`apt_cache` e `dnf_cache` dependem da distribuição.

## Configuração

### Arquivos

| Arquivo | Conteúdo |
|---|---|
| `~/.config/purgar/config.json` | Configuração do usuário. |
| `~/.local/share/purgar/historico.jsonl` | Histórico de limpezas em JSON Lines. |

Os caminhos padrão seguem XDG: `$XDG_CONFIG_HOME/purgar/config.json` e
`$XDG_DATA_HOME/purgar/historico.jsonl` quando essas variáveis estão definidas.
Se o arquivo de configuração não existir, os valores padrão são usados.

### Chaves de `config.json`

| Chave | Tipo | Padrão | Descrição |
|---|---|---|---|
| `usar_lixeira` | booleano | `false` | Move os itens para a lixeira em vez de removê-los. Pode ser sobrescrito por `--lixeira`/`--sem-lixeira`. |
| `caminhos` | array de strings | `[]` | Caminhos personalizados da categoria `custom`. São validados antes de qualquer limpeza. |
| `dias_tmp` | inteiro (`u64`) | `7` | Idade mínima, em dias, dos arquivos de `/tmp` e `/var/tmp`. |
| `dias_logs` | inteiro (`u64`) | `30` | Idade mínima, em dias, dos logs do sistema em `/var/log`. |
| `historico_max` | inteiro (`usize`) | `500` | Número máximo de registros mantidos no histórico. |
| `apagar_dados_flatpak` | booleano | `false` | Permite que o desinstalador do Flatpak remova também os dados do aplicativo. |

Exemplo:

```json
{
  "usar_lixeira": true,
  "caminhos": ["/home/usuario/projetos/tmp-build"],
  "dias_tmp": 7,
  "dias_logs": 30,
  "historico_max": 500,
  "apagar_dados_flatpak": false
}
```

### Histórico

Cada linha do histórico é um objeto JSON com os campos `ts`, `categoria`,
`bytes`, `movidos` e `erros`:

```json
{"ts":"2026-11-13T12:00:00-03:00","categoria":"cache_apps","bytes":123456789,"movidos":42,"erros":0}
```

O histórico alimenta a tela de Estatísticas de limpeza da TUI e é limitado por
`historico_max`.

## Segurança e limitações

O purgar é uma ferramenta destrutiva por natureza: ele apaga cache e arquivos
temporários. As garantias implementadas são:

- **Symlinks**: a varredura interna de diretórios nunca segue symlinks.
  Diretórios-raiz simbólicos são resolvidos para validação — ou seja, a
  categoria só é limpa depois que o caminho real é validado.
- **Caminhos personalizados**: são validados e rejeitam `/`, `/usr`, `/etc`,
  `/home`, `/root`, `/var`, `/boot`, `/dev`, `/proc`, `/sys`, `/run`, `/bin`,
  `/lib*`, `/opt`, `/srv`, `/media`, `/mnt`, `/tmp`, `/var/tmp`, a própria home
  e caminhos rasos demais. O caminho precisa existir.
- **Modo lixeira**: segue a
  [freedesktop.org Trash spec](https://specifications.freedesktop.org/trash-spec/latest/),
  com `Path=` em URL-encoding e o arquivo `.trashinfo` criado antes do move. Se
  não for possível mover (por exemplo, por estar em outro sistema de arquivos),
  tenta cópia seguida de remoção verificada; nunca apaga silenciosamente.
- **`/tmp` como usuário**: remove apenas arquivos e symlinks antigos do próprio
  usuário; nunca remove diretórios com conteúdo. Preserva `.X11-unix`,
  `.ICE-unix`, `.XIM-unix`, `.font-unix`, `.Test-unix`, `.dbus`,
  `systemd-private-*` e `ssh-*`.
- **Navegadores**: a limpeza de cache de navegadores é bloqueada enquanto houver
  navegador aberto, tanto na TUI quanto no CLI.
- **Confirmação**: o CLI exige confirmação interativa (ou `--sim`); na TUI, cada
  limpeza passa por um modal de confirmação.
- **Administrador**: ações administrativas usam `pkexec`/polkit. Sem sessão
  gráfica, a operação falha com uma mensagem clara em vez de assumir privilégios
  por conta própria.
- **Terminal**: a TUI restaura o terminal mesmo em caso de erro ou panic. A tela
  de progresso pode ser cancelada com `Esc` e `Ctrl+C` sai do programa.
- **Desinstalador**: pacotes essenciais são protegidos em RPM, Flatpak e Snap.

Limitações:

- A limpeza é definitiva quando o modo lixeira está desativado.
- Categorias administrativas dependem de um agente polkit em sessão gráfica.
- `downloads` é somente informativa.
- Categorias condicionais dependem das ferramentas instaladas e do gerenciador
  de pacotes da distribuição.
- O purgar não remove pacotes do sistema fora do desinstalador e não altera
  configurações do sistema operacional.

## Desenvolvimento

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release
```

O CI do GitHub Actions executa formatação, clippy, testes e build de release em
cada push e pull request. As tags `v*` disparam o workflow de release, que
publica tarballs para `x86_64` e `aarch64`.

Contribuições são bem-vindas. Leia [CONTRIBUTING.md](CONTRIBUTING.md),
[SECURITY.md](SECURITY.md) e o [CHANGELOG.md](CHANGELOG.md).

## Estrutura de arquivos

```text
purgar/
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── lib.rs           # biblioteca e módulos públicos
│   ├── main.rs          # CLI e ponto de entrada
│   ├── categories.rs    # definição das categorias e alvos
│   ├── cleaner.rs       # análise e limpeza
│   ├── fsutil.rs        # utilitários de filesystem e lixeira
│   ├── seguranca.rs     # validação de caminhos e barreiras de segurança
│   ├── config.rs        # configuração e histórico
│   ├── model.rs         # tipos e mensagens
│   ├── tools.rs         # autostart e desinstalador
│   └── tui/
│       ├── mod.rs       # estado e eventos da TUI
│       └── ui.rs        # desenho das telas
├── packaging/
│   ├── purgar.spec      # spec RPM (Fedora/RHEL)
│   ├── PKGBUILD         # receita Arch Linux
│   └── purgar.1         # man page
├── completions/
│   ├── purgar.bash
│   ├── purgar.zsh
│   └── purgar.fish
└── .github/workflows/
    ├── ci.yml
    └── release.yml
```

## Licença

Distribuído sob a licença MIT. Veja [LICENSE](LICENSE).
