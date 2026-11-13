use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::Local;
use rayon::prelude::*;
use walkdir::WalkDir;

/// Tamanho em bytes de um arquivo ou diretório (não segue symlinks).
pub fn tamanho(caminho: &Path) -> u64 {
    let meta = match fs::symlink_metadata(caminho) {
        Ok(m) => m,
        Err(_) => return 0,
    };
    if meta.is_file() {
        return meta.len();
    }
    if meta.is_symlink() {
        return 0;
    }
    if meta.is_dir() {
        return tamanho_dir(caminho);
    }
    0
}

/// Soma o tamanho de todos os arquivos dentro do diretório, em paralelo.
pub fn tamanho_dir(dir: &Path) -> u64 {
    let total = AtomicU64::new(0);
    let entradas: Vec<PathBuf> = match fs::read_dir(dir) {
        Ok(rd) => rd.flatten().map(|e| e.path()).collect(),
        Err(_) => return 0,
    };
    entradas.par_iter().for_each(|p| {
        total.fetch_add(tamanho_unico(p), Ordering::Relaxed);
    });
    total.load(Ordering::Relaxed)
}

fn tamanho_unico(caminho: &Path) -> u64 {
    let meta = match fs::symlink_metadata(caminho) {
        Ok(m) => m,
        Err(_) => return 0,
    };
    if meta.is_file() {
        meta.len()
    } else if meta.is_dir() {
        tamanho_dir_seq(caminho)
    } else {
        0
    }
}

fn tamanho_dir_seq(dir: &Path) -> u64 {
    let mut total = 0u64;
    let walker = WalkDir::new(dir).follow_links(false).into_iter();
    for entry in walker.filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            total += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    total
}

/// Lista os filhos imediatos de um diretório, opcionalmente excluindo nomes.
pub fn filhos(dir: &Path, excluir: &[&str]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let nome = e.file_name();
            let nome = nome.to_string_lossy();
            if excluir.iter().any(|x| *x == nome) {
                continue;
            }
            out.push(e.path());
        }
    }
    out.sort();
    out
}

/// Raiz da lixeira do usuário (freedesktop.org Trash spec).
pub fn lixeira_raiz() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/"))
                .join(".local/share")
        })
        .join("Trash")
}

/// Remove um arquivo ou diretório.
///
/// Se `lixeira` for true, move para a lixeira (freedesktop.org) e retorna os
/// bytes movidos. Nunca apaga permanentemente como fallback silencioso: se não
/// for possível mover/copiar para a lixeira, devolve erro.
pub fn remover(caminho: &Path, lixeira: bool) -> io::Result<u64> {
    remover_em(caminho, lixeira, None)
}

/// Como [`remover`], permitindo indicar uma lixeira alternativa (testes).
pub fn remover_em(caminho: &Path, lixeira: bool, raiz_lixeira: Option<&Path>) -> io::Result<u64> {
    let bytes = tamanho(caminho);
    if lixeira {
        let raiz = raiz_lixeira
            .map(Path::to_path_buf)
            .unwrap_or_else(lixeira_raiz);
        mover_lixeira_em(caminho, &raiz)?;
    } else {
        remover_direto(caminho)?;
    }
    Ok(bytes)
}

fn remover_direto(caminho: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(caminho)?;
    if meta.is_dir() {
        fs::remove_dir_all(caminho)
    } else {
        fs::remove_file(caminho)
    }
}

/// Move `caminho` para a lixeira seguindo a especificação do freedesktop:
/// `.trashinfo` é gravado antes do move, `Path=` é URL-encoded e, se o
/// `rename` falhar (ex.: outro sistema de arquivos), tenta cópia verificada
/// antes de remover o original.
fn mover_lixeira_em(caminho: &Path, raiz: &Path) -> io::Result<()> {
    let files = raiz.join("files");
    let info = raiz.join("info");
    fs::create_dir_all(&files)?;
    fs::create_dir_all(&info)?;

    let nome = caminho
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "item".into());

    let carimbo = Local::now().format("%Y%m%d%H%M%S%3f").to_string();
    let mut contador = 0u32;
    let destino = loop {
        let sufixo = if contador == 0 {
            String::new()
        } else {
            format!("-{contador}")
        };
        let candidato = files.join(format!("{carimbo}-{nome}{sufixo}"));
        if !candidato.exists() {
            break candidato;
        }
        contador += 1;
        if contador > 10_000 {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "não foi possível encontrar um nome livre na lixeira",
            ));
        }
    };

    let nome_destino = destino
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "item".into());
    let info_path = info.join(format!("{nome_destino}.trashinfo"));
    let original = caminho.to_string_lossy();
    let conteudo = format!(
        "[Trash Info]\nPath={}\nDeletionDate={}\n",
        url_encoding(&original),
        Local::now().format("%Y-%m-%dT%H:%M:%S")
    );
    // Grava o .trashinfo antes do move: se o processo cair no meio, o item
    // continua na lixeira e o metadado já existe.
    fs::write(&info_path, conteudo)?;

    match fs::rename(caminho, &destino) {
        Ok(()) => Ok(()),
        Err(erro_rename) => {
            // Provavelmente EXDEV (outro sistema de arquivos) ou permissão.
            // Copia com verificação e só então remove o original.
            match copiar_recursivo(caminho, &destino) {
                Ok(()) => match remover_direto(caminho) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        let _ = remover_direto(&destino);
                        let _ = fs::remove_file(&info_path);
                        Err(e)
                    }
                },
                Err(_) => {
                    let _ = remover_direto(&destino);
                    let _ = fs::remove_file(&info_path);
                    Err(erro_rename)
                }
            }
        }
    }
}

fn copiar_recursivo(origem: &Path, destino: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(origem)?;
    if meta.file_type().is_symlink() {
        let alvo = fs::read_link(origem)?;
        std::os::unix::fs::symlink(alvo, destino)
    } else if meta.is_dir() {
        fs::create_dir_all(destino)?;
        for entrada in fs::read_dir(origem)? {
            let entrada = entrada?;
            copiar_recursivo(&entrada.path(), &destino.join(entrada.file_name()))?;
        }
        fs::set_permissions(destino, meta.permissions())?;
        Ok(())
    } else if meta.is_file() {
        fs::copy(origem, destino)?;
        fs::set_permissions(destino, meta.permissions())?;
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!(
                "tipo de arquivo especial não suportado na lixeira: {}",
                origem.display()
            ),
        ))
    }
}

/// Percent-encoding mínimo exigido pela especificação da lixeira.
pub fn url_encoding(texto: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(texto.len());
    for byte in texto.bytes() {
        let c = byte as char;
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~' | '/') {
            out.push(c);
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// Espaço total e livre (bytes) do sistema de arquivos que contém `caminho`.
pub fn espaco_disco(caminho: &Path) -> Option<(u64, u64)> {
    Some((
        fs2::total_space(caminho).ok()?,
        fs2::free_space(caminho).ok()?,
    ))
}

/// Formata bytes em formato legível pt-BR (ex.: "1,5 GB").
pub fn humano(bytes: u64) -> String {
    const UNIDADES: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut valor = bytes as f64;
    let mut idx = 0;
    while valor >= 1024.0 && idx < UNIDADES.len() - 1 {
        valor /= 1024.0;
        idx += 1;
    }
    if idx == 0 {
        format!("{} {}", bytes, UNIDADES[idx])
    } else {
        format!("{:.1} {}", valor, UNIDADES[idx]).replace('.', ",")
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn modo(caminho: &Path) -> u32 {
        fs::metadata(caminho)
            .map(|m| m.permissions().mode())
            .unwrap_or(0)
    }
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(nome: &str) -> PathBuf {
        let carimbo = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("purgar-{nome}-{carimbo}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn humano_formata_unidades() {
        assert_eq!(humano(0), "0 B");
        assert_eq!(humano(512), "512 B");
        assert_eq!(humano(1024), "1,0 KB");
        assert_eq!(humano(1536), "1,5 KB");
        assert_eq!(humano(1024 * 1024 * 1024), "1,0 GB");
    }

    #[test]
    fn remove_arquivo_e_diretorio() {
        let base = temp_dir("remover");
        let arquivo = base.join("a.txt");
        fs::write(&arquivo, vec![b'x'; 100]).unwrap();
        assert_eq!(remover(&arquivo, false).unwrap(), 100);
        assert!(!arquivo.exists());

        let sub = base.join("sub");
        fs::create_dir_all(sub.join("nested")).unwrap();
        fs::write(sub.join("nested/f.bin"), vec![b'y'; 50]).unwrap();
        let removidos = remover(&sub, false).unwrap();
        assert_eq!(removidos, 50);
        assert!(!sub.exists());
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn move_para_lixeira_com_trashinfo() {
        let base = temp_dir("lixeira");
        let trash = base.join("Trash");
        let arquivo = base.join("Área de Trabalho.txt");
        fs::write(&arquivo, b"conteudo").unwrap();
        remover_em(&arquivo, true, Some(&trash)).unwrap();
        assert!(!arquivo.exists());

        let files = fs::read_dir(trash.join("files")).unwrap().count();
        let info = fs::read_dir(trash.join("info")).unwrap().count();
        assert_eq!(files, 1);
        assert_eq!(info, 1);
        let info_path = fs::read_dir(trash.join("info"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let conteudo = fs::read_to_string(info_path).unwrap();
        assert!(conteudo.starts_with("[Trash Info]"));
        assert!(conteudo.contains("DeletionDate="));
        assert!(conteudo.contains("%20") || conteudo.contains("Trash/"));
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn url_encoding_escapa_espacos_e_acentos() {
        assert_eq!(url_encoding("/a b/ç"), "/a%20b/%C3%A7");
        assert_eq!(url_encoding("/ok/file-name_1.txt"), "/ok/file-name_1.txt");
    }

    #[test]
    fn nao_cria_arquivos_especiais_em_modo_lixeira() {
        // Regressão: falha de rename não deve apagar o original.
        let base = temp_dir("lixeira-falha");
        let arquivo = base.join("foo");
        fs::write(&arquivo, b"x").unwrap();
        // Destino impossível: lixeira sob um arquivo comum.
        let invalida = base.join("invalida");
        fs::write(&invalida, b"nao-e-dir").unwrap();
        assert!(remover_em(&arquivo, true, Some(&invalida)).is_err());
        assert!(arquivo.exists(), "original não pode ser apagado");
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn modo_preserva_permissoes_na_copia_da_lixeira() {
        let base = temp_dir("permissoes");
        let trash = base.join("Trash");
        let arquivo = base.join("exec.sh");
        fs::write(&arquivo, b"#!/bin/sh\n").unwrap();
        fs::set_permissions(&arquivo, fs::Permissions::from_mode(0o755)).unwrap();
        remover_em(&arquivo, true, Some(&trash)).unwrap();
        let item = fs::read_dir(trash.join("files"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        if item.exists() {
            assert_eq!(modo(&item) & 0o777, 0o755);
        }
        let _ = fs::remove_dir_all(&base);
    }
}
