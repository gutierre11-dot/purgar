use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant, SystemTime};

use rayon::prelude::*;
use walkdir::WalkDir;

use crate::fsutil;
use crate::model::{Alvo, Categoria, Resumo, ScanResult};

/// Mensagens enviadas pela thread de trabalho para a TUI.
pub enum Msg {
    /// (bytes processados, itens processados)
    Progresso(u64, u64),
    /// Linha de log (saída de comandos, avisos).
    Linha(String),
    /// Operação concluída.
    Fim(Result<Resumo, String>),
}

/// Encontra os caminhos que uma categoria removeria (para escaneamento).
fn unidades(alvo: &Alvo) -> Vec<PathBuf> {
    match alvo {
        Alvo::Esvaziar { dir, excluir } => {
            if !dir.is_dir() {
                return Vec::new();
            }
            fsutil::filhos(dir, excluir)
        }
        Alvo::Remover(path) => {
            if path.exists() || path.symlink_metadata().is_ok() {
                vec![path.clone()]
            } else {
                Vec::new()
            }
        }
        Alvo::Nomes { raiz, nomes, prof } => coletar_nomes(raiz, nomes, *prof, 0),
        Alvo::Antigos { dirs, dias, sudo } => {
            if *sudo {
                Vec::new()
            } else {
                coletar_antigos(dirs, *dias)
            }
        }
        Alvo::ComandoUser(_) | Alvo::ComandoRoot(_) => Vec::new(),
    }
}

fn coletar_nomes(raiz: &Path, nomes: &[&str], prof_max: usize, prof: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if prof > prof_max || !raiz.is_dir() {
        return out;
    }
    let rd = match std::fs::read_dir(raiz) {
        Ok(r) => r,
        Err(_) => return out,
    };
    for e in rd.flatten() {
        let p = e.path();
        let nome = e.file_name().to_string_lossy().into_owned();
        if let Ok(ft) = e.file_type() {
            if ft.is_dir() {
                if nomes.iter().any(|n| *n == nome) {
                    out.push(p);
                    continue; // não desce dentro de um match
                }
                if prof < prof_max {
                    out.extend(coletar_nomes(&p, nomes, prof_max, prof + 1));
                }
            }
        }
    }
    out
}

fn coletar_antigos(dirs: &[PathBuf], dias: u64) -> Vec<PathBuf> {
    let limite = SystemTime::now()
        .checked_sub(Duration::from_secs(dias * 86400))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let mut out = Vec::new();
    for dir in dirs {
        if !dir.is_dir() {
            continue;
        }
        for entry in WalkDir::new(dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.depth() == 0 {
                continue;
            }
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            let modificado = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            if modificado < limite {
                out.push(entry.path().to_path_buf());
            }
        }
    }
    out
}

/// Analisa uma categoria e retorna o espaço que seria liberado.
pub fn escanear(categoria: &Categoria) -> ScanResult {
    let mut res = ScanResult::default();
    // unidades por alvo (paralelo entre alvos)
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
            // comandos não têm tamanho mensurável diretamente
            if requer_root {
                res.detalhes
                    .push(format!("{} — requer root", rotulo_alvo(alvo)));
            } else {
                res.detalhes
                    .push(format!("{} — sob demanda", rotulo_alvo(alvo)));
            }
            continue;
        }
        if matches!(alvo, Alvo::Antigos { sudo: true, .. }) {
            res.detalhes
                .push(format!("{} — requer root", rotulo_alvo(alvo)));
            continue;
        }
        let bytes: u64 = paths.par_iter().map(|p| fsutil::tamanho(p)).sum();
        res.bytes += bytes;
        res.itens += paths.len() as u64;
        if bytes > 0 {
            res.detalhes
                .push(format!("{} — {}", rotulo_alvo(alvo), fsutil::humano(bytes)));
        }
    }
    res
}

fn rotulo_alvo(alvo: &Alvo) -> String {
    match alvo {
        Alvo::Esvaziar { dir, .. } => format!("Conteúdo de {}", dir.display()),
        Alvo::Remover(p) => format!("{}", p.display()),
        Alvo::Nomes { raiz, .. } => format!("Caches em {}", raiz.display()),
        Alvo::Antigos { dirs, dias, sudo } => format!(
            "Arquivos > {} dias em {}{}",
            dias,
            dirs.iter()
                .map(|d| d.display().to_string())
                .collect::<Vec<_>>()
                .join(", "),
            if *sudo { " (root)" } else { "" }
        ),
        Alvo::ComandoUser(cmd) => cmd.join(" "),
        Alvo::ComandoRoot(cmd) => format!("[root] {}", cmd.join(" ")),
    }
}

/// Executa um comando capturando stdout/stderr linha a linha.
fn executar_comando(cmd: &[String], root: bool, tx: &Sender<Msg>) -> Result<(), String> {
    let mut full: Vec<String> = if root {
        let mut v = vec!["pkexec".to_string()];
        v.extend(cmd.iter().cloned());
        v
    } else {
        cmd.to_vec()
    };
    // pkexec precisa de DISPLAY/Wayland para a caixa de diálogo do polkit
    if root {
        if std::env::var("DISPLAY").is_err() && std::env::var("WAYLAND_DISPLAY").is_err() {
            return Err("Elevação requer sessão gráfica (DISPLAY/WAYLAND_DISPLAY).".into());
        }
    }
    let programa = full.remove(0);
    let _ = &mut full;
    let mut child = Command::new(&programa)
        .args(&full)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Falha ao executar '{}': {}", programa, e))?;

    // drena stdout+stderr em threads para não bloquear
    use std::io::{BufRead, BufReader, Read};
    fn drenar<S: Read + Send + 'static>(stream: Option<S>, tx: &Sender<Msg>) {
        if let Some(s) = stream {
            let tx2 = tx.clone();
            std::thread::spawn(move || {
                for linha in BufReader::new(s).lines().map_while(Result::ok) {
                    let _ = tx2.send(Msg::Linha(linha));
                }
            });
        }
    }
    drenar(child.stdout.take(), tx);
    drenar(child.stderr.take(), tx);
    let status = child
        .wait()
        .map_err(|e| format!("Erro aguardando comando: {}", e))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("Comando terminou com status {}", status))
    }
}

/// Limpa uma categoria em segundo plano, enviando progresso por `tx`.
pub fn limpar(categoria: &Categoria, usar_lixeira: bool, tx: Sender<Msg>) {
    let inicio = Instant::now();
    let mut resumo = Resumo {
        bytes: 0,
        itens: 0,
        erros: 0,
        lixeira: usar_lixeira,
        linhas: Vec::new(),
    };
    let _ = tx.send(Msg::Linha(format!(
        "Iniciando limpeza de '{}'...",
        categoria.nome
    )));

    for alvo in &categoria.alvos {
        match alvo {
            Alvo::ComandoUser(cmd) | Alvo::ComandoRoot(cmd) => {
                let root = matches!(alvo, Alvo::ComandoRoot(_));
                let _ = tx.send(Msg::Linha(format!(
                    "Executando: {}",
                    if root { "[root] " } else { "" }.to_string() + &cmd.join(" ")
                )));
                match executar_comando(cmd, root, &tx) {
                    Ok(()) => {
                        let _ = tx.send(Msg::Linha("  concluído".into()));
                    }
                    Err(e) => {
                        resumo.erros += 1;
                        let _ = tx.send(Msg::Linha(format!("  erro: {}", e)));
                    }
                }
            }
            Alvo::Antigos { dirs, dias, sudo } => {
                if *sudo {
                    let _ = tx.send(Msg::Linha(format!(
                        "Removendo temporários antigos (root) em {}...",
                        dirs.iter()
                            .map(|d| d.display().to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )));
                    match executar_find_sudo(dirs, *dias, &tx) {
                        Ok(()) => {}
                        Err(e) => {
                            resumo.erros += 1;
                            let _ = tx.send(Msg::Linha(format!("  erro: {}", e)));
                        }
                    }
                } else {
                    limpar_antigos_usuario(dirs, *dias, usar_lixeira, &tx, &mut resumo);
                }
            }
            _ => {
                let paths = unidades(alvo);
                if paths.is_empty() {
                    continue;
                }
                let _ = tx.send(Msg::Linha(format!("Processando {}...", rotulo_alvo(alvo))));
                // paralelo entre itens do alvo
                let resultados: Vec<std::io::Result<u64>> = paths
                    .par_iter()
                    .map(|p| fsutil::remover(p, usar_lixeira))
                    .collect();
                for r in resultados {
                    match r {
                        Ok(bytes) => {
                            resumo.bytes += bytes;
                            resumo.itens += 1;
                            let _ = tx.send(Msg::Progresso(resumo.bytes, resumo.itens));
                        }
                        Err(e) => {
                            resumo.erros += 1;
                            if resumo.erros <= 10 {
                                let _ = tx.send(Msg::Linha(format!("  falha: {}", e)));
                            }
                        }
                    }
                }
            }
        }
    }

    let _ = inicio;
    let dur = inicio.elapsed();
    resumo.linhas.push(format!(
        "Concluído em {:.1}s — {} em {} item(ns), {} erro(s).",
        dur.as_secs_f64(),
        fsutil::humano(resumo.bytes),
        resumo.itens,
        resumo.erros
    ));
    let _ = tx.send(Msg::Fim(Ok(resumo)));
}

/// Versão Rust da limpeza de arquivos antigos (como usuário).
fn limpar_antigos_usuario(
    dirs: &[PathBuf],
    dias: u64,
    lixeira: bool,
    tx: &Sender<Msg>,
    resumo: &mut Resumo,
) {
    use std::os::unix::fs::MetadataExt;
    let meu_uid = std::fs::metadata("/proc/self")
        .map(|m| m.uid())
        .unwrap_or(u32::MAX);
    let limite = SystemTime::now()
        .checked_sub(Duration::from_secs(dias * 86400))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    // coleta primeiro para evitar modificar durante a caminhada
    let mut alvos = Vec::new();
    for dir in dirs {
        if !dir.is_dir() {
            continue;
        }
        for entry in WalkDir::new(dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.depth() == 0 {
                continue;
            }
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            // em /tmp (sticky), só remove o que é do usuário
            if dir.starts_with("/tmp") && meta.uid() != meu_uid {
                continue;
            }
            if meta.modified().unwrap_or(SystemTime::UNIX_EPOCH) < limite {
                alvos.push(entry.path().to_path_buf());
            }
        }
    }
    // remove filhos antes dos pais (ordem reversa de profundidade)
    alvos.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    alvos.dedup();
    let _ = tx.send(Msg::Linha(format!(
        "Removendo {} item(ns) antigo(s) em {}...",
        alvos.len(),
        dirs.iter()
            .map(|d| d.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )));
    for p in alvos {
        match fsutil::remover(&p, lixeira) {
            Ok(bytes) => {
                resumo.bytes += bytes;
                resumo.itens += 1;
                let _ = tx.send(Msg::Progresso(resumo.bytes, resumo.itens));
            }
            Err(_) => resumo.erros += 1,
        }
    }
}

/// Limpa arquivos antigos via pkexec + find (sem lixeira: arquivos velhos).
fn executar_find_sudo(dirs: &[PathBuf], dias: u64, tx: &Sender<Msg>) -> Result<(), String> {
    let mut args: Vec<String> = vec!["find".into()];
    for d in dirs {
        args.push(d.display().to_string());
    }
    args.extend(["-xdev".into(), "-mindepth".into(), "1".into(), "(".into()]);
    for (i, exc) in [
        ".X11-unix",
        ".ICE-unix",
        ".XIM-unix",
        ".font-unix",
        ".Test-unix",
        ".dbus",
    ]
    .iter()
    .enumerate()
    {
        if i > 0 {
            args.push("-o".into());
        }
        args.push("-path".into());
        args.push(format!("*/{}", exc));
    }
    // systemd-private-* e ssh-*
    args.push("-o".into());
    args.push("-path".into());
    args.push("*/systemd-private-*".into());
    args.push("-o".into());
    args.push("-path".into());
    args.push("*/ssh-*".into());
    args.extend([
        ")".into(),
        "-prune".into(),
        "-o".into(),
        "-mtime".into(),
        format!("+{}", dias),
        "-delete".into(),
    ]);
    let cmd: Vec<String> = vec!["pkexec".into()].into_iter().chain(args).collect();
    let _ = tx.send(Msg::Linha(format!("Executando: {}", cmd.join(" "))));
    let mut child = Command::new("pkexec")
        .args(&cmd[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("pkexec não disponível: {}", e))?;
    use std::io::{BufRead, BufReader, Read};
    fn drenar<S: Read + Send + 'static>(stream: Option<S>, tx: &Sender<Msg>) {
        if let Some(s) = stream {
            let tx2 = tx.clone();
            std::thread::spawn(move || {
                for linha in BufReader::new(s).lines().map_while(Result::ok) {
                    let _ = tx2.send(Msg::Linha(linha));
                }
            });
        }
    }
    drenar(child.stdout.take(), tx);
    drenar(child.stderr.take(), tx);
    let status = child.wait().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("find terminou com status {}", status))
    }
}

/// Verifica se um processo está em execução (comparação por nome).
pub fn processo_rodando(nomes: &[&str]) -> Vec<String> {
    use sysinfo::{ProcessRefreshKind, RefreshKind, System};
    let sys = match System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::everything()),
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let mut achados = Vec::new();
    for proc in sys.processes().values() {
        let nome_proc = proc.name().to_string_lossy();
        let nome = nome_proc.to_lowercase();
        // crashpad é um handler de crashes sempre residente — não indica navegador aberto
        // (o nome pode vir truncado pelo comm do Linux, ex.: "chrome_crashpa")
        if nome.contains("crash") {
            continue;
        }
        for alvo in nomes {
            let alvo_l = alvo.to_lowercase();
            // corresponde nome exato ou prefixo seguido de separador
            // (ex.: "firefox" casa "firefox", mas não "firefox-developer-edition")
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

/// Espaço em disco de `/` (total, livre).
pub fn espaco_disco() -> Option<(u64, u64)> {
    Some((fs2::total_space("/").ok()?, fs2::free_space("/").ok()?))
}
