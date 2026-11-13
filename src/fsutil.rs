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
    // Coleta entradas de primeiro nível e processa em paralelo
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

/// Versão sequencial usada dentro da recursão (evita explosão de threads).
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

/// Remove um arquivo ou diretório. Se `lixeira` for true, move para
/// ~/.local/share/Trash (files + info). Retorna bytes liberados/movidos.
pub fn remover(caminho: &Path, lixeira: bool) -> io::Result<u64> {
    let bytes = tamanho(caminho);
    if lixeira {
        mover_lixeira(caminho)?;
    } else {
        if caminho.is_dir() && !caminho.is_symlink() {
            fs::remove_dir_all(caminho)?;
        } else {
            fs::remove_file(caminho)?;
        }
    }
    Ok(bytes)
}

/// Move `caminho` para a lixeira seguindo a especificação do freedesktop.
fn mover_lixeira(caminho: &Path) -> io::Result<()> {
    let trash = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/"))
        .join(".local/share/Trash");
    let files = trash.join("files");
    let info = trash.join("info");
    fs::create_dir_all(&files)?;
    fs::create_dir_all(&info)?;

    let nome = caminho
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "item".into());

    // nome único
    let carimbo = Local::now().format("%Y%m%d%H%M%S").to_string();
    let mut destino = files.join(format!("{}-{}", carimbo, nome));
    let mut i = 1;
    while destino.exists() {
        destino = files.join(format!("{}-{}-{}", carimbo, nome, i));
        i += 1;
    }
    // Só é possível mover dentro do mesmo sistema de arquivos;
    // caso contrário, remove diretamente.
    match fs::rename(caminho, &destino) {
        Ok(()) => {
            let nome_destino = destino.file_name().unwrap().to_string_lossy();
            let info_final = info.join(format!("{}.trashinfo", nome_destino));
            let original = caminho.to_string_lossy();
            let conteudo = format!(
                "[Trash Info]\nPath={}\nDeletionDate={}\n",
                original,
                Local::now().format("%Y-%m-%dT%H:%M:%S")
            );
            let _ = fs::write(&info_final, conteudo);
            Ok(())
        }
        Err(_) => {
            if caminho.is_dir() && !caminho.is_symlink() {
                fs::remove_dir_all(caminho)
            } else {
                fs::remove_file(caminho)
            }
        }
    }
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
