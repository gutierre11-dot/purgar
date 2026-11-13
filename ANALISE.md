# purgar — análise técnica e lacunas

**Versão analisada:** 0.1.0 (`4e0bd74` — "Adiciona README completo e licença MIT para publicação")
**Data:** 2026-11-13
**Escopo:** leitura integral dos 3.754 LOC, compilação, lints, testes exploratórios em PTY, reprodução de comportamento de `find`, medições no sistema real. Nenhum código foi alterado e nenhuma limpeza destrutiva real foi executada (apenas `scan`, leituras e navegação na TUI).

---

## 1. Resumo executivo

O projeto tem uma **base sólida e bem organizada**: módulos com responsabilidades claras (`model`, `categories`, `cleaner`, `fsutil`, `tools`, `config`, `tui`), código limpo, boa cobertura de categorias para Fedora/KDE, TUI funcional e bonita, e várias ideias de segurança corretas (não seguir symlinks durante a varredura, `Clear` no modal, confirmação antes de limpar, modo lixeira opcional).

Mas há **problemas reais** que impedem considerar o projeto "pronto para publicação":

| Indicador | Resultado |
|---|---|
| Compilação (`cargo check --all-targets`) | ✅ sem erros |
| `cargo fmt --check` | ✅ limpo |
| `cargo clippy --all-targets` | ⚠️ **19 warnings** |
| `cargo test` | ⚠️ **0 testes** |
| MSRV real (dependências) | **Rust 1.95** (`sysinfo 0.40.3`), README diz 1.85 |
| Dependências não usadas | `thiserror` (compilada, nunca importada) |
| Binário release | 2,3 MB |
| `purgar scan` completo (esta máquina) | 1,8 s ✅ |
| Categoria "Arquivos temporários antigos" (parte root) | ❌ **sempre falha** (bug no `find`) |
| TUI em terminal sem resposta / erro de setup | ❌ deixa terminal em raw mode + alternate screen |

**Veredito:** bom MVP, com **1 bug que quebra uma funcionalidade inteira**, **5 riscos sérios de perda de dados**, **4 afirmações do README que não condizem com o código** e **lacunas de engenharia/distribuição** (testes, CI, packaging). As correções da Fase 0/1 são pequenas e cabem em ~1–2 dias.

---

## 2. Como a análise foi feita

```bash
cargo check --all-targets          # ✅ ok
cargo clippy --all-targets         # 19 warnings
cargo test                         # 0 testes
cargo fmt --check                  # ✅ ok
./target/release/purgar scan       # 1,8 s; tabela desalinhada; TOTAL inclui downloads
# reprodução do find do código (sem pkexec) em diretório temporário:
find DIR -xdev -mindepth 1 '(' -path '*/systemd-private-*' ')' -prune -o -mtime +6 -delete
# → find: erro: -delete ativa -depth e -prune não faz nada (exit 1, nada é apagado)

# TUI dirigida por PTY (Python pty + winsize + resposta a \x1b[6n):
#   Home, Detalhe, modal de confirmação, Estatísticas, Configurações,
#   Desinstalador: renderizam, `q` sai e restaura o terminal (exit 0).
#   Com `\x1b[6n` não respondido: "Error: The cursor position could not be read
#   within a normal duration", exit 1, SEM restaurar alternate screen.
#   Sem TTY: "Error: No such device or address (os error 6)".
```

Medições auxiliares: `cargo metadata` (MSRV), soma de `~/.cache` (overlap entre categorias), `grep` de uso de dependências.

---

## 3. Achados priorizados

Legenda: 🔴 crítico (perda de dados / funcionalidade quebrada) · 🟠 alto (bug ou risco relevante) · 🟡 médio (qualidade/UX) · 🔵 baixo (polish) · 📦 produto/distribuição.

### 3.1 Tabela resumo

| ID | Sev. | Área | Achado | Evidência |
|---|---|---|---|---|
| C1 | 🔴 | temp root | `find` com `-prune` + `-delete` **sempre falha** → a parte root de `/tmp` e `/var/tmp` nunca limpa | `src/cleaner.rs:382-448` |
| C2 | 🔴 | lixeira | Se `rename` para a lixeira falha (outro FS/permissão), `remover` **apaga permanentemente em silêncio** — contradiz "nada é apagado definitivamente" | `src/fsutil.rs:139-145` |
| C3 | 🔴 | /tmp | limpeza como usuário remove diretórios antigos com `remove_dir_all`, **apagando arquivos recentes dentro deles**; e não respeita as exclusões documentadas (`.X11-unix`, `systemd-private-*`, `ssh-*`) | `src/cleaner.rs:316-379`, `src/fsutil.rs:87-99` |
| C4 | 🔴 | caminhos custom | Nenhuma validação: aceitar `/`, `~`, `/home`, `/usr` em "Caminhos personalizados" e limpar **esvazia o sistema inteiro** | `src/categories.rs:372-396`, `src/tui/mod.rs:434-452` |
| C5 | 🔴 | TUI | Se a thread de trabalho morre sem `Msg::Fim` (panic/desconexão), a TUI **trava para sempre** na tela "Operação em andamento" (Esc não faz nada) | `src/tui/mod.rs:329-371` |
| C6 | 🔴 | TUI | `&item.exec[..57]` pode causar **panic** por corte em byte não-UTF-8 (item de autostart com acento) | `src/tui/ui.rs:705-709` |
| A1 | 🟠 | TUI | `Ctrl+C` anunciado no rodapé **não sai**; só `Ctrl+Q` funciona | `src/tui/ui.rs:307`, `src/tui/mod.rs:505` |
| A2 | 🟠 | TUI | Falha em `Terminal::new`/erro de setup deixa terminal em raw mode + alternate screen (sem guarda `Drop`) | `src/tui/mod.rs:379-418` |
| A3 | 🟠 | navegadores | `~/.cache/{google-chrome,chromium,brave,vivaldi,microsoft-edge,opera,zen,librewolf,floorp}` são **excluídos** do cache genérico mas **nenhuma categoria os limpa** | `src/categories.rs:21-55` vs `130-184` |
| A4 | 🟠 | lixeira | "Lixeira" com `usar_lixeira=true` **re-move itens para a própria lixeira** (nunca libera espaço) e o histórico registra como "liberado" | `src/cleaner.rs:280-283`, `src/config.rs:57-75` |
| A5 | 🟠 | histórico | Todas as categorias de comando root (journal/dnf/coredumps) registram **0 B liberados**, mesmo liberando GB; resultado da limpeza mostra "0 B" | `src/cleaner.rs:235-272, 303-313` |
| A6 | 🟠 | CLI | `purgar clean <id>` **não confirma, não tem `--dry-run` e não bloqueia navegador aberto** (README afirma o contrário) | `src/main.rs:78-134` |
| A7 | 🟠 | UX | Home mostra **"Nada a limpar — sistema já está otimizado"** para categorias que só funcionam como root (`logs_sistema`, `journal_sistema`, `cache_sistema`, `coredumps`) | `src/tui/ui.rs:239-258` |
| A8 | 🟠 | escopo | `Alvo::Nomes` em `~/.var/app` (profundidade 8) apaga **qualquer pasta chamada `cache`/`Cache`** de qualquer Flatpak, não só navegadores | `src/categories.rs:178-182` |
| A9 | 🟠 | desinstalador | Sem proteção para pacotes críticos (`snapd`, `core`, `kernel*`, `dnf`, `systemd`, `flatpak`); `flatpak uninstall --delete-data` apaga dados do app de forma irreversível | `src/tools.rs:149-281` |
| A10 | 🟠 | autostart | `Hidden=` é **acrescentado no fim do arquivo**, possivelmente fora do grupo `[Desktop Entry]` → toggle pode não fazer efeito; ignora `X-GNOME-Autostart-enabled` | `src/tools.rs:92-113` |
| M1 | 🟡 | totais | `purgar scan` `TOTAL` inclui 8,7 GB de **Downloads (informativo)** e **conta duas vezes** `~/.cache/flatpak`/`dnf` | `src/main.rs:54-76`, `src/categories.rs:117-129, 325-334` |
| M2 | 🟡 | CLI | Tabela do `scan` desalinha: coluna fixa de 22 chars estoura com "Nome (id)" | `src/main.rs:62-75` (saída real: `Cache de aplicativos (cache_apps)     819,9 MB`) |
| M3 | 🟡 | CLI | Sem `--version`/`-V` (exit 2), sem `--json`, sem `--dry-run`, sem `--yes`, sem `--quiet` | `src/main.rs:31-39` |
| M4 | 🟡 | CLI | Sem TTY, `purgar` imprime `Error: No such device or address (os error 6)` | reproduzido |
| M5 | 🟡 | TUI | Scroll da Home/Desinstalador/Autostart calcula offset por índice de item, mas a lista renderiza linhas extras (cabeçalhos de seção + linhas em branco) → seleção pode sair da área visível | `src/tui/ui.rs:203-206, 683-685, 802-805` |
| M6 | 🟡 | TUI | Listagem de pacotes (`rpm -qa`, `flatpak list`, `snap list`) e `processo_rodando` rodam **síncronos na thread da UI** → travadas perceptíveis | `src/tui/mod.rs:229-232, 540-545`, `src/tools.rs:149-201` |
| M7 | 🟡 | TUI | Sem cancelamento de limpeza/scan (Esc na tela de Progresso é no-op); `iniciar_scan_tudo` acumula threads de scan sem cancelar as antigas | `src/tui/mod.rs:201-212, 482-486` |
| M8 | 🟡 | lixeira | `.trashinfo` fora da spec: `Path=` sem URL-encoding, info escrito depois do `rename`, sem `directorysizes`/suporte a lixeira de outros mounts | `src/fsutil.rs:101-147` |
| M9 | 🟡 | categorias | Suporte a distro incompleto: `cache_sistema` só DNF/YUM, `dnf_cache` sempre aparece e falha em Debian; sem apt/pacman/zypper | `src/categories.rs:280-334` |
| M10 | 🟡 | categorias | Categorias ausentes relevantes: cache apt, Docker/Podman, revisões antigas do Snap, `flatpak uninstall --unused`, VS Code/JetBrains, shader cache do Steam, `systemd-tmpfiles` | `src/categories.rs` |
| M11 | 🟡 | config | `carregar` **reseta silenciosamente** config inválida e a próxima gravação sobrescreve o arquivo; `salvar` não é atômico e não usa lock (apesar de `fs2` disponível) | `src/config.rs:33-49` |
| M12 | 🟡 | histórico | `historico.jsonl` cresce sem limite e é lido inteiro a cada operação; Estatísticas mostram **ids crus** (`cache_dev`) e registros de 0 B | `src/config.rs:85-93`, `src/tui/ui.rs:909-943` |
| M13 | 🟡 | UI | `wrap_text` mede bytes, não largura de display → quebra errada com acentos/emoji; cores RGB fixas sem fallback/no-color | `src/tui/ui.rs:36-52, 12-23` |
| M14 | 🟡 | arquitetura | `Categoria`/`Alvo` não derivam `Clone`; o mesmo `match` de clonagem manual aparece **2×** (+ uso em série) | `src/main.rs:143-177`, `src/tui/mod.rs:822-864` |
| M15 | 🟡 | arquitetura | `escanear` mede o tamanho e `remover` **mede de novo** antes de apagar → I/O dobrado em caches grandes | `src/fsutil.rs:87-99`, `src/cleaner.rs:280-283` |
| M16 | 🟡 | UX | Sem `?`/ajuda na TUI, sem busca de categorias, sem seleção múltipla/"Limpar tudo", sem cancelar | `src/tui/` |
| M17 | 🟡 | progresso | Tela de progresso não tem barra/percentual (total é conhecido pelo scan e não é usado) | `src/tui/ui.rs:471-547` |
| B1 | 🔵 | deps | `thiserror` não usado; `fs2 0.4.3` (última versão há anos, sem manutenção) — sucessor `fs4`; `fs2` também não é usado para lock | `Cargo.toml` |
| B2 | 🔵 | lints | 19 warnings do clippy: `collapsible_if` ×10, `unnecessary_sort_by` ×2, `redundant_closure` ×2, `derivable_impls`, `explicit_counter_loop`, `vec_init_then_push`, `single_match`, `print_literal` | `cargo clippy` |
| B3 | 🔵 | dead code | Restos: `let _ = &mut full`, `let _ = inicio`, bloco vazio de detecção de scan (`done`), `if id == "downloads" { }` | `src/cleaner.rs:186-187, 303`, `src/tui/mod.rs:299-319` |
| B4 | 🔵 | UI | `desenhar()` recebe `&mut App` sem mutar; `ESTADOS`/helpers duplicados | `src/tui/ui.rs:54`, `src/tui/mod.rs:644-677` |
| B5 | 🔵 | i18n | Textos pt-BR hardcoded; sem `LANG`/i18n (ok para o escopo declarado, mas não escala) | todo o código |
| B6 | 🔵 | docs | `logs_sistema` diz ".gz e numéricos", o comando também remove `*.old`; `coredumps` mascara erro com `2>/dev/null; true` (sempre "concluído") | `src/categories.rs:257-267, 336-347` |
| B7 | 🔵 | mídia | README sem screenshots/GIF da TUI (projeto visual) | `README.md` |
| B8 | 🔵 | permissões | Sem single-instance lock; duas TUIs podem limpar/gravar config concorrentemente | `src/tui/mod.rs` |
| D1 | 📦 | MSRV | README diz "Rust 1.85+"; dependências exigem **1.95** (`sysinfo`); badge diz "1.99" | `README.md`, `cargo metadata` |
| D2 | 📦 | packaging | Sem `.spec` (alvo Fedora!), deb/AUR, man page, completions, política polkit, `cargo install` documentado | repo |
| D3 | 📦 | CI/release | Sem GitHub Actions (fmt/clippy/test/build), sem binários release, sem `CHANGELOG`, sem screenshot | repo |
| D4 | 📦 | metadata | `Cargo.toml` sem `description`, `license`, `repository`, `keywords`, `categories`, `rust-version`, `[profile.release]` (lto/strip) | `Cargo.toml` |
| D5 | 📦 | README | 4 afirmações divergentes do código (confirmação no CLI, sockets em /tmp, "nunca segue symlinks", bloqueio de navegador no CLI) | ver §5 |
| D6 | 📦 | polkit | `pkexec` sem `.policy` própria → diálogo genérico e sem mensagem em pt-BR | `src/cleaner.rs:172-185` |

---

## 4. Detalhamento dos críticos (com correção sugerida)

### C1 — A categoria `tmp_antigos` (parte root) nunca funciona

`executar_find_sudo` monta:

```text
find /var/tmp /tmp -xdev -mindepth 1 ( -path '*/.X11-unix' -o ... -path '*/ssh-*' ) -prune -o -mtime +7 -delete
```

O GNU find recusa a combinação: `-delete` ativa `-depth` implicitamente, e **`-prune` não tem efeito com `-depth`**. Resultado reproduzido:

```
find: A ação -delete automaticamente ativa -depth, mas -prune faz nada quando -depth tiver efeito.
exit 1 — nenhum arquivo apagado
```

Impacto: a parte privilegiada de `/tmp` e `/var/tmp` **nunca limpa**; `purgar clean tmp_antigos` sempre termina com `1 erro` (exit 1 no CLI) e a TUI mostra erro. Em implementações de `find` diferentes de GNU, a combinação pode ser ainda pior (apagar o que era para preservar).

**Correção** (sem `-prune`; exclusões como predicados negados, `-delete` por último):

```rust
let mut args: Vec<String> = vec!["find".into()];
for d in dirs { args.push(d.display().to_string()); }
args.extend(["-xdev", "-mindepth", "1", "-type", "f", "-mtime"].map(String::from));
args.push(format!("+{}", dias));
for pat in [
    "*/systemd-private-*", "*/ssh-*", "*/.X11-unix/*", "*/.ICE-unix/*",
    "*/.XIM-unix/*", "*/.font-unix/*", "*/.Test-unix/*", "*/.dbus/*",
] {
    args.push("!".into());
    args.push("-path".into());
    args.push(pat.into());
}
args.push("-delete".into());
```

Além disso: medir antes (com o mesmo `find ... -print0` ou estimativa) para o resumo não mostrar 0 B; e não repetir `/tmp` nos dois alvos (usuário e root) na mesma categoria.

### C2 — Modo lixeira pode apagar permanentemente

```rust
match fs::rename(caminho, &destino) {
    Ok(()) => { /* grava .trashinfo */ Ok(()) }
    Err(_) => {                       // <- cross-device, permissão, etc.
        if caminho.is_dir() { fs::remove_dir_all(caminho) } else { fs::remove_file(caminho) }
    }
}
```

Se o item está em outro mount (ex.: cache em partição separada) o `rename` falha com `EXDEV` e o arquivo é **apagado de verdade** — exatamente o que o modo lixeira promete evitar. O `.trashinfo` também só é escrito *depois* do move (se o processo morrer no meio, fica item órfão na lixeira), o `Path=` não é URL-encoded como a spec exige e o `DeletionDate` não tem timezone.

**Correção:** em falha de `rename`, retornar `io::Error` (limpeza falha e usuário é avisado) ou fazer fallback `copy + remove` com verificação; escrever o `.trashinfo` antes do move e removê-lo em caso de erro; usar `urlencoding` para `Path`; opcionalmente delegar a `gio trash`.

### C3 — Limpeza de `/tmp` como usuário pode apagar dados recentes e ignora exclusões

`limpar_antigos_usuario` coleta **qualquer entrada** (arquivo *ou diretório*) com `mtime` antigo e chama `fsutil::remover`, que para diretório faz `remove_dir_all`. Cenário real: um diretório criado há 10 dias cujo arquivo interno foi **modificado ontem** — o diretório tem mtime antigo (modificar conteúdo não altera mtime do diretório), então ele entra na lista e o arquivo recente é apagado junto.

Além disso, a versão como usuário **não aplica** as exclusões documentadas no README: filtra só por UID (`src/cleaner.rs:350`). Sockets `ssh-*` e `.X11-unix` pertencentes ao usuário podem ser removidos se antigos.

**Correção:** (a) remover somente arquivos/symlinks (`entry.file_type().is_file() || is_symlink()`), nunca diretórios; (b) aplicar os mesmos `-path` de exclusão da versão root; (c) ordenar e apagar, e para diretórios vazios antigos usar `fs::remove_dir` (não recursivo); (d) considerar `atime` quando disponível.

### C4 — Caminhos personalizados sem validação = risco de destruição total

`Config.caminhos` aceita qualquer string; a TUI adiciona o que for digitado (`Enter`) e `cleaner` transforma em `Esvaziar { dir }` se for diretório. Um usuário que adicionar `~`, `/` ou `/home` e clicar em "Limpar agora" **esvazia a home ou o sistema de arquivos**. Não há denylist, canonicalização, checagem de profundidade, de symlink, nem confirmação diferenciada.

**Correção mínima:**

```rust
fn caminho_seguro(p: &Path) -> bool {
    let Ok(canon) = p.canonicalize() else { return false };
    let home = dirs::home_dir();
    let proibidos = ["/", "/usr", "/etc", "/home", "/var", "/boot", "/dev", "/proc", "/sys", "/run", "/bin", "/lib"];
    if proibidos.iter().any(|d| canon.as_path() == Path::new(d)) { return false; }
    if home.as_deref() == Some(canon.as_path()) { return false; }
    canon.components().count() >= 3 // exige profundidade mínima
}
```

Validar no `cleaner` (não só na UI), mostrar erro no modal, exigir confirmação digitada ("LIMPAR") para a categoria custom e nunca permitir `Esvaziar` em raiz de sistema. O mesmo vale para `Alvo::Remover` (hoje só a flag `informativo` protege o Downloads).

### C5 — TUI trava se a thread de limpeza morrer

Em `drenar_jobs`:

```rust
loop {
    match rx.try_recv() {
        Ok(Msg::Progresso(..)) => ...,
        Ok(Msg::Linha(..)) => ...,
        Ok(Msg::Fim(r)) => { fim = Some(r); break; }
        Err(_) => break,   // <-- Empty OU Disconnected
    }
}
```

Se a thread der panic (OU um `?` futuro) sem enviar `Msg::Fim`, o canal fica desconectado e `job_rx` continua `Some` → a tela `Progresso` fica presa para sempre ("Aguarde a conclusão..."). Esc é no-op e não há timeout.

**Correção:** distinguir `TryRecvError::Empty` de `Disconnected` e, no segundo caso, preencher `resultado = Some(Err("thread de limpeza terminou inesperadamente".into()))`, limpar `job_rx` e ir para `Tela::Resultado`.

### C6 — Panic por slicing de UTF-8

```rust
let exec = if item.exec.len() > 60 {
    format!("{}...", &item.exec[..57])   // panic se byte 57 não for boundary
```

Hoje nenhum `.desktop` desta máquina tem `Exec` assim, mas qualquer autostart do usuário com caminho acentuado (ex.: `~/Área de Trabalho/...`) pode estourar `byte index 57 is not a char boundary`. Correção:

```rust
let exec: String = if item.exec.chars().count() > 60 {
    format!("{}...", item.exec.chars().take(57).collect::<String>())
} else { item.exec.clone() };
```

---

## 5. Altos — o que mais dói no uso real

1. **`Ctrl+C` não sai (A1)**: rodapé promete "Ctrl+C sair", mas só `Ctrl+Q` é tratado (`src/tui/mod.rs:505`). Em raw mode o SIGINT é desligado; adicionar `Char('c') if CONTROL => sair` (ou ignorar Ctrl+C apenas na tela de progresso, com aviso).
2. **Terminal sujo em erro (A2)**: `run()` faz `enable_raw_mode` → `EnterAlternateScreen` → `Terminal::new(...)?`; qualquer falha depois do alternate screen retorna sem `LeaveAlternateScreen`/`disable_raw_mode` (reproduzido em PTY). Criar um guard com `Drop` e instalar panic hook que restaura o terminal.
3. **Caches de navegadores Chromium nunca limpos (A3)**: `CACHE_EXCLUIR` exclui `google-chrome`, `chromium`, `brave`, `vivaldi`, `microsoft-edge`, `opera`, `zen`, `librewolf`, `floorp`, `thorium` de `cache_apps`, mas `cache_navegadores` só trata `~/.cache/mozilla` + `~/.config/*` + `~/.var/app`. Ou seja, `~/.cache/google-chrome` etc. **ficam fora de qualquer categoria**, e o scan mostra 0. Adicionar `Alvo::Esvaziar { dir: p("~/.cache/<navegador>") }` para cada um (ou remover da lista de exclusão e deixar o genérico cuidar).
4. **Estatísticas mentem (A4/A5/M12)**: (a) modo lixeira registra bytes "liberados" que só foram movidos; (b) comandos root registram 0 B; (c) IDs crus na tela; (d) operações de 0 B entram no histórico. Corrigir: separar `liberado` × `movido`, medir `free_space` antes/depois para categorias de comando (ou parsear saída de `journalctl --vacuum-time`), mapear id→nome e ignorar registros de 0 B.
5. **Root-only aparece como "sistema otimizado" (A7)**: a Home usa `tam == 0` como sinônimo de nada a limpar. Para `logs_sistema`/`journal_sistema`/`cache_sistema`/`coredumps`/`tmp_antigos`, `ScanResult.requer_root` já existe — usar: "requer administrador (tamanho não medido)". O mesmo vale para a tabela do `scan` no CLI.
6. **`~/.var/app` overreach (A8)**: profundidade 8 e nomes `cache`/`Cache` apagam caches de *qualquer* Flatpak. Restringir a IDs de navegadores conhecidos (`org.mozilla.firefox`, `com.google.Chrome`, `com.brave.Browser`, `io.github.zen_browser.zen`, …) ou renomear/documentar a categoria.
7. **Desinstalador perigoso (A9)**: listar todos os RPMs e permitir remover `kernel`, `systemd`, `dnf`, `glibc`... e `snap remove snapd/core` quebra o sistema. Adicionar denylist + aviso vermelho; tornar `--delete-data` do Flatpak opcional; detectar gerenciador (dnf vs apt vs pacman) e esconder origens indisponíveis.
8. **Autostart (A10)**: `definir_hidden` insere/substitui `Hidden=` onde quer que a primeira ocorrência esteja, mas se não existir **acrescenta no fim do arquivo**, possivelmente fora do grupo `[Desktop Entry]` (`.desktop` com `[Desktop Action ...]` no fim). Fazer parse por grupos (editar a seção `[Desktop Entry]` especificamente) e respeitar `X-GNOME-Autostart-enabled` (GNOME/KDE atuais).

---

## 6. README × código (divergências a corrigir)

| README afirma | Código faz |
|---|---|
| "Confirmação antes de cada limpeza" | CLI `clean` limpa **sem qualquer confirmação/flags** |
| "Bloqueia limpeza ... enquanto houver navegador aberto" | Só a **TUI** bloqueia (`src/tui/mod.rs:591`); CLI não |
| "Nunca segue symlinks ao medir ou remover" | `Esvaziar`/`Nomes` **seguem** symlink do diretório-raiz (`dir.is_dir()`, `read_dir`); só a varredura interna não segue |
| "Preserva sockets e diretórios ativos em /tmp (.X11-unix, systemd-private-*, ssh-*)" | Exclusões existem **só** no caminho root (que está quebrado — C1); o caminho como usuário filtra só por UID |
| "Modo lixeira opcional (nada é apagado definitivamente)" | Em falha de `rename`, apaga permanentemente (C2) |
| "Estatísticas de espaço liberado historicamente" | Inclui itens movidos para a lixeira e registra 0 B para comandos root |
| "Rust 1.85+" | Dependências exigem **1.95** (`sysinfo 0.40.3`) |

Também vale documentar que as categorias avançadas exigem `pkexec`/polkit e que o CLI é destrutivo por natureza.

---

## 7. Lacunas de produto (paridade com CCleaner / uso diário)

1. **"Limpar tudo" / seleção múltipla**: hoje só uma categoria por vez (diferente do "Analisar + Limpar" do CCleaner).
2. **`--dry-run`/preview** no CLI e **modo simulação** na TUI (há scan, mas a limpeza é sempre real).
3. **Cancelamento** de limpeza e de scan em andamento (com token `AtomicBool` checado entre itens).
4. **Progresso real**: barra/percentual usando o total do scan; ETA; throughput.
5. **Ajuda na TUI** (`?`/F1) e rodapé contextual (hoje o rodapé do Detalhe não mostra `Esc` na Home etc.).
6. **Filtro/busca de categorias** na Home (são 18 linhas hoje; crescerá).
7. **Categorias novas** (ver M10) e **detecção de distro/gerenciador** para esconder o que não existe (`dnf` em Debian).
8. **Configuração mais rica**: dias de retenção de `/tmp` e logs, exclusões, categorias habilitadas, limite do histórico, tema/cores, `no-color`.
9. **Snapshots/undo**: com lixeira ativa, um "Esvaziar lixeira" poderia virar "restaurar".
10. **Integração desktop**: notificação ao fim, `.desktop` do app, ícone, autostart opcional do próprio purgar (irônico).

---

## 8. Qualidade de engenharia

- **Testes: zero.** Prioridades: `fsutil::humano`, `wrap_text`, `parse_desktop`/`definir_hidden`, `coletar_nomes`, matching de `processo_rodando`, validação de caminhos, e testes de integração com `tempfile` para `remover`, lixeira, `/tmp` antigos e categoria custom (incluindo tentativa de path proibido).
- **CI**: workflow com `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, build release; `cargo audit`/`cargo deny`.
- **MSRV**: adicionar `rust-version = "1.95"` **ou** pinar `sysinfo` mais antigo se quiser manter 1.85. Atualizar README/badge.
- **Dependências**: remover `thiserror`; trocar `fs2` por `fs4` (ou `rustix::fs::statvfs`); avaliar `clap` (CLI crescerá), `tempfile` (dev), `unicode-width` (wrap correto). O `Cargo.lock` já está versionado (bom para binário).
- **Lints**: 19 warnings do clippy — aplicar `cargo clippy --fix` (16 automáticos) e configurar `[lints]` no `Cargo.toml`.
- **Duplicação**: derivar `Clone` em `Alvo`/`Categoria` e apagar as 3 cópias de clonagem; unificar `App::new` × `recarregar_categorias`; `desenhar(&App)`.
- **Dead code**: `let _ = &mut full`, `let _ = inicio`, bloco `done` vazio, `if id == "downloads" {}` (B3).
- **Arquitetura**: separar `ui` (puro) de `App` (estado) já está quase lá; introduzir `Job`/`Scan` tipados com `Result` e cancelamento; mover validação de caminho para `cleaner`/`model`.
- **Erros**: o CLI devolve `anyhow::Error` cru (`Error: No such device...`); mapear para mensagens em pt-BR + `--help` de fallback.
- **Single-instance**: `fs2`/`fs4` lock em `~/.local/share/purgar/lock` para evitar duas TUIs limpando ao mesmo tempo.
- **Performance**: medir uma vez e reutilizar (M15); scan incremental/cancelável; `sysinfo` com refresh mínimo e cache; `processo_rodando` só com nomes.
- **Segurança**: `#![forbid(unsafe_code)]` (não há `unsafe`), validação de entrada, revisão de todos os `remove_dir_all`.

---

## 9. Distribuição e publicação

- **Packaging**: `.spec` RPM/COPR (o público-alvo declarado é Fedora), `.deb`, `PKGBUILD`/AUR, Flatpak (com portal), AppImage; `cargo install --path .` documentado; binários pré-compilados no GitHub Releases.
- **`purgar.policy`** para pkexec (mensagem localizada "Limpar cache do sistema") + instalação em `/usr/share/polkit-1/actions`.
- **Man page** (`purgar.1`) e `completions/` (bash/zsh/fish) para `purgar`, `scan`, `clean`, `lista`.
- **Release**: GitHub Actions com matriz de build (x86_64/aarch64), checksums, `CHANGELOG.md`, tag semântica.
- **README**: corrigir MSRV e as 4 afirmações (§6), adicionar **screenshots/GIF**, tabela de categorias com o que cada uma toca, seção "segurança/limitações" (root, lixeira, CLI destrutivo), requisitos (`pkexec`, `systemd`, `dnf`), e instruções de desinstalação.
- **`Cargo.toml`**: `description`, `license`, `repository`, `homepage`, `keywords`, `categories`, `rust-version`, `[profile.release] lto = "fat", strip = true, codegen-units = 1` (binário 2,3 MB → ~1,4 MB).

---

## 10. Roadmap sugerido

### Fase 0 — Quick wins (½–1 dia)
- [ ] C6 (slicing UTF-8), A1 (Ctrl+C), B3 (dead code), B2 (`clippy --fix` + `[lints]`)
- [ ] D4 metadata + `rust-version` + `[profile.release]`; remover `thiserror`
- [ ] M2 (tabela do scan), M1 (excluir informativos/deduplicar do TOTAL), M3 (`--version`)
- [ ] D5 corrigir README (MSRV + 4 afirmações)

### Fase 1 — Segurança e correção (1–2 dias)
- [ ] C1 (`find` sem `-prune`; medir root), C3 (`/tmp` só arquivos + exclusões), C2 (lixeira sem fallback destrutivo)
- [ ] C4 (validação de caminhos + confirmação forte)
- [ ] C5 (tela de progresso não trava), A2 (guard de terminal + panic hook), M4 (sem TTY: mensagem amigável)
- [ ] A3 (caches Chromium), A4/A5/M12 (histórico fiel)

### Fase 2 — Robustez e UX (3–5 dias)
- [ ] A6 (`--dry-run`/`--yes`/bloqueio de navegador no CLI), M6 (listagens assíncronas), M7/M16 (cancelamento)
- [ ] A7 (root-only na Home/scan), A8 (escopo `~/.var/app`), A9 (proteção de pacotes críticos), A10 (autostart por grupo)
- [ ] M5 (scroll), M13 (wrap unicode/no-color), M11 (config atômica + backup), M14/M15 (refactor/performance), M17 (gauge)
- [ ] Testes unitários + integração; CI; `#![forbid(unsafe_code)]`

### Fase 3 — Produto e distribuição (2–3 dias)
- [ ] "Limpar tudo", busca, ajuda `?`, M10 (novas categorias + detecção de distro), M9
- [ ] M8 (lixeira spec-compliant), M16 (single-instance), polkit `.policy`
- [ ] Packaging (.spec/deb/AUR/Flatpak), man page, completions, release CI, screenshots, CHANGELOG

**Estimativa total:** ~7–11 dias de trabalho focado para chegar a uma v1.0 publicável com confiança.

---

## 11. Pontos fortes (não perder no refactor)

- Estrutura de módulos enxuta e legível (10 arquivos, 3.754 LOC, `cargo fmt` limpo).
- Boa separação `Categoria`/`Alvo` — fácil adicionar categorias e testar.
- Segurança pensada: `filter_map` com `follow_links(false)`, `symlink_metadata` ao medir, confirmação antes de limpar, `Clear` no modal, bloqueio de navegadores na TUI.
- TUI de qualidade: paleta consistente, spinner, painel de detalhes, progresso com log, histórico e estatísticas — visualmente acima da média de utilitários CLI.
- Scan rápido (1,8 s) graças a rayon + walkdir; progresso incremental por categoria.
- CLI funcional (`scan`/`clean`/`lista`) e código de saída coerente (0/1/2).
- `Cargo.lock` versionado; binário pequeno; MIT; README bem escrito (só precisa ser corrigido onde diverge).

---

## 12. Anexo — reproduções

```bash
# 1) find quebrado (C1)
T=$(mktemp -d); mkdir -p "$T/systemd-private-x" "$T/normal"
touch -d '10 days ago' "$T/systemd-private-x/old" "$T/normal/old"
find "$T" -xdev -mindepth 1 '(' -path '*/systemd-private-*' ')' -prune -o -mtime +6 -delete
# -> find: ... -delete automaticamente ativa -depth ... -prune faz nada  (exit 1)

# 2) scan: total inclui downloads e tabela desalinhada (M1/M2)
./target/release/purgar scan

# 3) --version ausente (M3)
./target/release/purgar --version; echo $?   # 2

# 4) sem TTY (M4)
./target/release/purgar </dev/null; echo $?  # 1 + mensagem crua

# 5) TUI em PTY com \x1b[6n sem resposta (A2)
#    -> "Error: The cursor position could not be read..." e alt screen não restaurado

# 6) MSRV real (D1)
cargo metadata --format-version 1 | grep -o '"rust_version":"[^"]*"' | sort -u
# -> sysinfo 0.40.3: 1.95
```
