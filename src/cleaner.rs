use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant, SystemTime};

use rayon::prelude::*;
use walkdir::WalkDir;

use crate::fsutil;
use crate::model::{Alvo, Categoria, Resumo, ScanResult};
use crate::seguranca;
use crate::util::drenar;

/// Mensagens enviadas pela thread de trabalho para a TUI.
pub enum Msg {
    /// (bytes processados, itens processados)
    Progresso(u64, u64),
    /// Linha de log (saída de comandos, avisos).
    Linha(String),
    /// Fim de uma categoria dentro de uma limpeza em lote (para histórico).
    Historico {
        categoria: &'static str,
        bytes: u64,
        movidos: u64,
        erros: u64,
    },
    /// Operação concluída.
    Fim(Result<Resumo, String>),
}

/// Opções de execução de uma limpeza.
#[derive(Clone)]
pub struct Opcoes {
    pub usar_lixeira: bool,
    pub cancelar: Arc<AtomicBool>,
    /// Total estimado (usado pela barra de progresso da TUI).
    pub total_estimado: u64,
    /// Lixeira alternativa (apenas para testes).
    pub lixeira_raiz: Option<PathBuf>,
}

impl Opcoes {
    pub fn novo(usar_lixeira: bool) -> Self {
        Self {
            usar_lixeira,
            cancelar: Arc::new(AtomicBool::new(false)),
            total_estimado: 0,
            lixeira_raiz: None,
        }
    }

    pub fn com_total(usar_lixeira: bool, total_estimado: u64) -> Self {
        let mut opcoes = Self::novo(usar_lixeira);
        opcoes.total_estimado = total_estimado;
        opcoes
    }

    pub fn cancelado(&self) -> bool {
        self.cancelar.load(Ordering::Relaxed)
    }
}

/// Limpa uma categoria em segundo plano, enviando progresso por `tx`.
pub fn limpar(categoria: &Categoria, opcoes: &Opcoes, tx: Sender<Msg>) {
    let mut resumo = Resumo {
        lixeira: opcoes.usar_lixeira,
        ..Resumo::default()
    };
    let _ = tx.send(Msg::Linha(format!(
        "Iniciando limpeza de '{}'...",
        categoria.nome
    )));
    executar_limpeza(categoria, opcoes, &tx, &mut resumo);
    finalizar(&tx, resumo);
}

/// Limpa várias categorias em sequência (usado por "Limpar tudo").
pub fn limpar_todas(categorias: &[Categoria], opcoes: &Opcoes, tx: Sender<Msg>) {
    let mut resumo = Resumo {
        lixeira: opcoes.usar_lixeira,
        ..Resumo::default()
    };
    for categoria in categorias {
        if opcoes.cancelado() {
            resumo.cancelado = true;
            break;
        }
        if categoria.informativo {
            continue;
        }
        let _ = tx.send(Msg::Linha(format!("=== {} ===", categoria.nome)));
        executar_limpeza(categoria, opcoes, &tx, &mut resumo);
        if resumo.cancelado {
            break;
        }
    }
    finalizar(&tx, resumo);
}

fn finalizar(tx: &Sender<Msg>, mut resumo: Resumo) {
    if resumo.cancelado {
        resumo.linhas.push("Limpeza cancelada pelo usuário.".into());
    }
    let _ = tx.send(Msg::Fim(Ok(resumo)));
}

fn executar_limpeza(categoria: &Categoria, opcoes: &Opcoes, tx: &Sender<Msg>, resumo: &mut Resumo) {
    let inicio = Instant::now();

    // Acumula o resultado desta categoria sobre o que já foi processado
    // anteriormente (quando chamada por limpar_todas). Caso contrário
    // (limpar único) o resumo já começa em zero.
    let acumulado_anterior = (resumo.bytes, resumo.movidos, resumo.itens, resumo.erros);
    *resumo = Resumo {
        lixeira: opcoes.usar_lixeira,
        ..Resumo::default()
    };

    for alvo in &categoria.alvos {
        if opcoes.cancelado() {
            resumo.cancelado = true;
            break;
        }
        match alvo {
            Alvo::ComandoUser(cmd) => {
                executar_comando_medindo(cmd, false, opcoes, tx, resumo);
            }
            Alvo::ComandoRoot(cmd) => {
                executar_comando_medindo(cmd, true, opcoes, tx, resumo);
            }
            Alvo::Antigos { dirs, dias, sudo } => {
                if *sudo {
                    let _ = tx.send(Msg::Linha(format!(
                        "Removendo temporários antigos (root) em {}...",
                        lista_dirs(dirs)
                    )));
                    let antes = espaco_livre();
                    match executar_find_sudo(dirs, *dias, tx) {
                        Ok(()) => {
                            let delta = delta_livre(antes, espaco_livre());
                            if delta > 0 {
                                resumo.bytes += delta;
                            }
                        }
                        Err(erro) => {
                            resumo.erros += 1;
                            let _ = tx.send(Msg::Linha(format!("  erro: {erro}")));
                        }
                    }
                } else {
                    limpar_antigos_usuario(dirs, *dias, opcoes, tx, resumo);
                }
            }
            Alvo::Medir(_) => {
                // Categoria informativa: nada a remover.
            }
            _ => {
                let paths = unidades(alvo);
                if paths.is_empty() {
                    continue;
                }
                let _ = tx.send(Msg::Linha(format!("Processando {}...", rotulo_alvo(alvo))));
                remover_itens(&paths, opcoes, tx, resumo);
            }
        }
    }

    // Emite um histórico por categoria para que o consumidor (limpar_todas,
    // a TUI e os testes) possa registrar cada etapa separadamente.
    let _ = tx.send(Msg::Historico {
        categoria: categoria.id,
        bytes: resumo.bytes,
        movidos: resumo.movidos,
        erros: resumo.erros,
    });

    // Restaura o acumulado anterior e soma o resultado desta categoria.
    let (bytes_ant, movidos_ant, itens_ant, erros_ant) = acumulado_anterior;
    resumo.bytes += bytes_ant;
    resumo.movidos += movidos_ant;
    resumo.itens += itens_ant;
    resumo.erros += erros_ant;

    if !resumo.cancelado {
        let duracao = inicio.elapsed();
        resumo.linhas.push(format!(
            "Concluído em {:.1}s — liberados {}, movidos para a lixeira {}, \
             {} item(ns), {} erro(s).",
            duracao.as_secs_f64(),
            fsutil::humano(resumo.bytes),
            fsutil::humano(resumo.movidos),
            resumo.itens,
            resumo.erros
        ));
    }
}

fn lista_dirs(dirs: &[PathBuf]) -> String {
    dirs.iter()
        .map(|d| d.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn remover_itens(paths: &[PathBuf], opcoes: &Opcoes, tx: &Sender<Msg>, resumo: &mut Resumo) {
    const LOTE: usize = 256;
    let mut falhas_mostradas = 0u32;
    for lote in paths.chunks(LOTE) {
        if opcoes.cancelado() {
            resumo.cancelado = true;
            break;
        }
        let resultados: Vec<std::io::Result<u64>> = lote
            .par_iter()
            .map(|p| fsutil::remover_em(p, opcoes.usar_lixeira, opcoes.lixeira_raiz.as_deref()))
            .collect();
        for resultado in resultados {
            match resultado {
                Ok(bytes) => {
                    if opcoes.usar_lixeira {
                        resumo.movidos += bytes;
                    } else {
                        resumo.bytes += bytes;
                    }
                    resumo.itens += 1;
                    let _ = tx.send(Msg::Progresso(resumo.bytes + resumo.movidos, resumo.itens));
                }
                Err(erro) => {
                    resumo.erros += 1;
                    if falhas_mostradas < 10 {
                        falhas_mostradas += 1;
                        let _ = tx.send(Msg::Linha(format!("  falha: {erro}")));
                    }
                }
            }
        }
    }
}

/// Encontra os caminhos que um alvo removeria (para escaneamento e prévia).
pub fn unidades(alvo: &Alvo) -> Vec<PathBuf> {
    match alvo {
        Alvo::Esvaziar { dir, excluir } => {
            if !dir.is_dir() || !seguranca::dir_raiz_permitido(dir) {
                return Vec::new();
            }
            fsutil::filhos(dir, excluir)
        }
        Alvo::Remover(path) => {
            if path.symlink_metadata().is_ok() {
                vec![path.clone()]
            } else {
                Vec::new()
            }
        }
        Alvo::Nomes { raiz, nomes, prof } => coletar_nomes(raiz, nomes, *prof, 0),
        Alvo::Antigos { dirs, dias, sudo } => {
            if *sudo {
                coletar_antigos(dirs, *dias, None)
            } else {
                coletar_antigos(dirs, *dias, Some(uid_atual()))
            }
        }
        Alvo::ComandoUser(_) | Alvo::ComandoRoot(_) => Vec::new(),
        Alvo::Medir(path) => {
            if path.symlink_metadata().is_ok() {
                vec![path.clone()]
            } else {
                Vec::new()
            }
        }
    }
}

fn coletar_nomes(raiz: &Path, nomes: &[&str], prof_max: usize, prof: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if prof > prof_max || !raiz.is_dir() || !seguranca::dir_raiz_permitido(raiz) {
        return out;
    }
    let rd = match std::fs::read_dir(raiz) {
        Ok(r) => r,
        Err(_) => return out,
    };
    for e in rd.flatten() {
        let p = e.path();
        let nome = e.file_name().to_string_lossy().into_owned();
        if let Ok(ft) = e.file_type()
            && ft.is_dir()
        {
            if nomes.iter().any(|n| *n == nome) {
                out.push(p);
                continue; // não desce dentro de um match
            }
            if prof < prof_max {
                out.extend(coletar_nomes(&p, nomes, prof_max, prof + 1));
            }
        }
    }
    out
}

fn uid_atual() -> u32 {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata("/proc/self")
        .map(|m| m.uid())
        .unwrap_or(u32::MAX)
}

fn caminho_do_uid_e_sticky(dir: &Path) -> bool {
    dir == Path::new("/tmp") || dir == Path::new("/var/tmp")
}

/// Coleta arquivos/symlinks antigos, respeitando exclusões e (opcionalmente)
/// o UID do usuário em diretórios sticky.
fn coletar_antigos(dirs: &[PathBuf], dias: u64, uid: Option<u32>) -> Vec<PathBuf> {
    use std::os::unix::fs::MetadataExt;
    let limite = SystemTime::now()
        .checked_sub(Duration::from_secs(dias.saturating_mul(86_400)))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let mut out = Vec::new();
    for dir in dirs {
        if !dir.is_dir() {
            continue;
        }
        let filtrar_uid = caminho_do_uid_e_sticky(dir);
        for entry in WalkDir::new(dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.depth() == 0 {
                continue;
            }
            if caminho_tem_componente_excluido(entry.path()) {
                continue;
            }
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if filtrar_uid
                && let Some(uid) = uid
                && meta.uid() != uid
            {
                continue;
            }
            let tipo = entry.file_type();
            if !tipo.is_file() && !tipo.is_symlink() {
                continue;
            }
            if meta.modified().unwrap_or(SystemTime::UNIX_EPOCH) < limite {
                out.push(entry.path().to_path_buf());
            }
        }
    }
    out
}

const NOMES_EXCLUIDOS: &[&str] = &[
    ".X11-unix",
    ".ICE-unix",
    ".XIM-unix",
    ".font-unix",
    ".Test-unix",
    ".dbus",
];

fn nome_excluido(nome: &str) -> bool {
    NOMES_EXCLUIDOS.contains(&nome)
        || nome.starts_with("systemd-private-")
        || nome.starts_with("ssh-")
        || nome.starts_with("pulse-")
}

fn caminho_tem_componente_excluido(caminho: &Path) -> bool {
    caminho.components().any(|c| {
        let nome = c.as_os_str().to_string_lossy();
        nome_excluido(&nome)
    })
}

/// Versão como usuário: remove apenas arquivos/symlinks antigos do próprio
/// usuário; nunca apaga diretórios com conteúdo. Depois tenta remover
/// diretórios vazios antigos com `remove_dir` (não recursivo).
fn limpar_antigos_usuario(
    dirs: &[PathBuf],
    dias: u64,
    opcoes: &Opcoes,
    tx: &Sender<Msg>,
    resumo: &mut Resumo,
) {
    use std::os::unix::fs::MetadataExt;
    let limite = SystemTime::now()
        .checked_sub(Duration::from_secs(dias.saturating_mul(86_400)))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let meu_uid = uid_atual();
    let mut arquivos = Vec::new();
    let mut diretorios = Vec::new();

    for dir in dirs {
        if !dir.is_dir() {
            continue;
        }
        let filtrar_uid = caminho_do_uid_e_sticky(dir);
        for entry in WalkDir::new(dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.depth() == 0 {
                continue;
            }
            if caminho_tem_componente_excluido(entry.path()) {
                continue;
            }
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if filtrar_uid && meta.uid() != meu_uid {
                continue;
            }
            if meta.modified().unwrap_or(SystemTime::UNIX_EPOCH) >= limite {
                continue;
            }
            let tipo = entry.file_type();
            if tipo.is_file() || tipo.is_symlink() {
                arquivos.push(entry.path().to_path_buf());
            } else if tipo.is_dir() {
                diretorios.push(entry.path().to_path_buf());
            }
        }
    }

    arquivos.sort();
    arquivos.dedup();
    let _ = tx.send(Msg::Linha(format!(
        "Removendo {} arquivo(s) antigo(s) em {}...",
        arquivos.len(),
        lista_dirs(dirs)
    )));
    remover_itens(&arquivos, opcoes, tx, resumo);

    // Diretórios: só remove se estiverem vazios (remove_dir não é recursivo).
    diretorios.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    diretorios.dedup();
    for dir in diretorios {
        if opcoes.cancelado() {
            resumo.cancelado = true;
            break;
        }
        let vazio = std::fs::read_dir(&dir)
            .map(|mut rd| rd.next().is_none())
            .unwrap_or(false);
        if vazio {
            let _ = std::fs::remove_dir(&dir);
        }
    }
}

/// Analisa uma categoria e retorna o espaço que seria liberado.
pub fn escanear(categoria: &Categoria) -> ScanResult {
    let mut res = ScanResult::default();
    let resultados: Vec<(usize, Vec<PathBuf>)> = categoria
        .alvos
        .par_iter()
        .enumerate()
        .map(|(i, a)| (i, unidades(a)))
        .collect();

    for (i, paths) in resultados {
        let alvo = &categoria.alvos[i];
        let requer_root = matches!(
            alvo,
            Alvo::ComandoRoot(_) | Alvo::Antigos { sudo: true, .. }
        );
        res.requer_root |= requer_root;

        if matches!(alvo, Alvo::ComandoUser(_) | Alvo::ComandoRoot(_)) {
            if requer_root {
                res.detalhes
                    .push(format!("{} — requer administrador", rotulo_alvo(alvo)));
            } else {
                res.detalhes
                    .push(format!("{} — sob demanda", rotulo_alvo(alvo)));
            }
            continue;
        }

        let bytes: u64 = paths.par_iter().map(|p| fsutil::tamanho(p)).sum();
        res.bytes += bytes;
        res.itens += paths.len() as u64;
        let sufixo = if requer_root {
            " (requer administrador)"
        } else {
            ""
        };
        if bytes > 0 || requer_root {
            res.detalhes.push(format!(
                "{} — {}{}",
                rotulo_alvo(alvo),
                fsutil::humano(bytes),
                sufixo
            ));
        }
    }
    res
}

/// Prévia textual dos alvos de uma categoria (usada no `clean --dry-run`).
pub fn detalhar_alvos(categoria: &Categoria) -> Vec<String> {
    categoria.alvos.iter().map(rotulo_alvo).collect()
}

/// Caminhos que seriam removidos por uma categoria (prévia).
pub fn paths_para_remocao(categoria: &Categoria) -> Vec<PathBuf> {
    categoria.alvos.iter().flat_map(unidades).collect()
}

/// Rótulo legível de um alvo.
pub fn rotulo_alvo(alvo: &Alvo) -> String {
    match alvo {
        Alvo::Esvaziar { dir, .. } => format!("Conteúdo de {}", dir.display()),
        Alvo::Remover(p) => format!("{}", p.display()),
        Alvo::Nomes { raiz, .. } => format!("Caches em {}", raiz.display()),
        Alvo::Antigos { dirs, dias, sudo } => format!(
            "Arquivos > {} dias em {}{}",
            dias,
            lista_dirs(dirs),
            if *sudo { " (root)" } else { "" }
        ),
        Alvo::ComandoUser(cmd) => cmd.join(" "),
        Alvo::ComandoRoot(cmd) => format!("[root] {}", cmd.join(" ")),
        Alvo::Medir(p) => format!("Medição (não remove) — {}", p.display()),
    }
}

/// Executa um comando capturando stdout/stderr linha a linha.
fn executar_comando(cmd: &[String], root: bool, tx: &Sender<Msg>) -> Result<(), String> {
    if root && std::env::var("DISPLAY").is_err() && std::env::var("WAYLAND_DISPLAY").is_err() {
        return Err("elevação requer sessão gráfica (DISPLAY/WAYLAND_DISPLAY)".into());
    }
    let (programa, args): (String, Vec<String>) = if root {
        let mut v = vec!["pkexec".to_string()];
        v.extend(cmd.iter().cloned());
        let programa = v.remove(0);
        (programa, v)
    } else {
        let mut v = cmd.to_vec();
        if v.is_empty() {
            return Err("comando vazio".into());
        }
        let programa = v.remove(0);
        (programa, v)
    };

    let mut child = Command::new(&programa)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("falha ao executar '{programa}': {e}"))?;

    drenar(child.stdout.take(), tx);
    drenar(child.stderr.take(), tx);
    let status = child
        .wait()
        .map_err(|e| format!("erro aguardando comando: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("comando terminou com status {status}"))
    }
}

fn log_comando(cmd: &[String], root: bool) -> String {
    let prefixo = if root { "[root] " } else { "" };
    format!("{prefixo}{}", cmd.join(" "))
}

fn executar_comando_medindo(
    cmd: &[String],
    root: bool,
    opcoes: &Opcoes,
    tx: &Sender<Msg>,
    resumo: &mut Resumo,
) {
    if opcoes.cancelado() {
        resumo.cancelado = true;
        return;
    }
    let _ = tx.send(Msg::Linha(format!(
        "Executando: {}",
        log_comando(cmd, root)
    )));
    let antes = espaco_livre();
    match executar_comando(cmd, root, tx) {
        Ok(()) => {
            let delta = delta_livre(antes, espaco_livre());
            if delta > 0 {
                resumo.bytes += delta;
            }
            let _ = tx.send(Msg::Linha(format!(
                "  concluído ({})",
                fsutil::humano(delta)
            )));
        }
        Err(erro) => {
            resumo.erros += 1;
            let _ = tx.send(Msg::Linha(format!("  erro: {erro}")));
        }
    }
}

fn espaco_livre() -> Option<u64> {
    fsutil::espaco_disco(&crate::categories::home()).map(|(_, livre)| livre)
}

fn delta_livre(antes: Option<u64>, depois: Option<u64>) -> u64 {
    match (antes, depois) {
        (Some(a), Some(d)) if d > a => d - a,
        _ => 0,
    }
}

/// Monta os argumentos do `find` usado na limpeza privilegiada de temporários.
///
/// Importante: não usamos `-prune` junto de `-delete` (GNU find recusa a
/// combinação porque `-delete` ativa `-depth`). As exclusões são predicados
/// negados e `-delete` fica por último.
pub fn args_find_antigos(dirs: &[PathBuf], dias: u64) -> Vec<String> {
    let mut args: Vec<String> = vec!["find".into()];
    for dir in dirs {
        args.push(dir.display().to_string());
    }
    args.extend(["-xdev", "-mindepth", "1", "-type", "f", "-mtime"].map(String::from));
    args.push(format!("+{dias}"));
    for nome in NOMES_EXCLUIDOS {
        for padrao in [format!("*/{nome}"), format!("*/{nome}/*")] {
            args.push("!".into());
            args.push("-path".into());
            args.push(padrao);
        }
    }
    for prefixo in ["systemd-private-", "ssh-", "pulse-"] {
        args.push("!".into());
        args.push("-path".into());
        args.push(format!("*/{prefixo}*"));
    }
    args.push("-delete".into());
    args
}

/// Limpa arquivos antigos via pkexec + find (arquivos velhos, sem lixeira).
fn executar_find_sudo(dirs: &[PathBuf], dias: u64, tx: &Sender<Msg>) -> Result<(), String> {
    if std::env::var("DISPLAY").is_err() && std::env::var("WAYLAND_DISPLAY").is_err() {
        return Err("elevação requer sessão gráfica (DISPLAY/WAYLAND_DISPLAY)".into());
    }
    let args = args_find_antigos(dirs, dias);
    let programa = args.first().cloned().unwrap_or_else(|| "find".into());
    let argumentos: Vec<String> = args.into_iter().skip(1).collect();
    let _ = tx.send(Msg::Linha(format!(
        "Executando: pkexec {}",
        std::iter::once(programa.clone())
            .chain(argumentos.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ")
    )));
    let mut child = Command::new("pkexec")
        .arg(&programa)
        .args(&argumentos)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("pkexec não disponível: {e}"))?;
    drenar(child.stdout.take(), tx);
    drenar(child.stderr.take(), tx);
    let status = child.wait().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("find terminou com status {status}"))
    }
}

/// Verifica se um processo está em execução (comparação por nome).
pub fn processo_rodando(nomes: &[&str]) -> Vec<String> {
    use sysinfo::{ProcessRefreshKind, RefreshKind, System};
    let sys = match System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing()),
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let mut achados = Vec::new();
    for proc in sys.processes().values() {
        let nome_proc = proc.name().to_string_lossy();
        let nome = nome_proc.to_lowercase();
        // crashpad é um handler de crashes sempre residente — não indica
        // navegador aberto (o nome pode vir truncado pelo comm do Linux).
        if nome.contains("crash") {
            continue;
        }
        for alvo in nomes {
            let alvo_l = alvo.to_lowercase();
            let casa = nome == alvo_l
                || nome
                    .strip_prefix(&alvo_l)
                    .map(|resto| resto.starts_with(|c: char| !c.is_alphanumeric()))
                    .unwrap_or(false);
            if casa {
                achados.push(nome_proc.to_string());
                break;
            }
        }
    }
    achados.sort();
    achados.dedup();
    achados
}

/// Espaço em disco do sistema de arquivos que contém a home (total, livre).
pub fn espaco_disco() -> Option<(u64, u64)> {
    fsutil::espaco_disco(&crate::categories::home())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn args_find_sem_prune_e_com_exclusoes() {
        let args = args_find_antigos(&[PathBuf::from("/tmp")], 7);
        assert!(!args.iter().any(|a| a == "-prune"));
        assert_eq!(args.last().unwrap(), "-delete");
        assert!(args.contains(&"-type".to_string()));
        assert!(args.contains(&"-xdev".to_string()));
        assert!(args.iter().any(|a| a == "*/.X11-unix/*"));
        assert!(args.iter().any(|a| a == "*/systemd-private-*"));
        assert!(args.contains(&"+7".to_string()));
    }

    #[test]
    fn nomes_excluidos_do_tmp() {
        assert!(nome_excluido(".X11-unix"));
        assert!(nome_excluido("systemd-private-abc"));
        assert!(nome_excluido("ssh-agent"));
        assert!(nome_excluido("pulse-xyz"));
        assert!(!nome_excluido("arquivo.tmp"));
        assert!(caminho_tem_componente_excluido(Path::new(
            "/tmp/.X11-unix/X0"
        )));
        assert!(!caminho_tem_componente_excluido(Path::new(
            "/tmp/cache/file"
        )));
    }
}
