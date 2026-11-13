use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Se true, itens removidos vão para a lixeira em vez de serem apagados.
    #[serde(default)]
    pub usar_lixeira: bool,
    /// Caminhos personalizados adicionados pelo usuário.
    #[serde(default)]
    pub caminhos: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            usar_lixeira: false,
            caminhos: Vec::new(),
        }
    }
}

fn caminho_config() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("purgar")
        .join("config.json")
}

pub fn carregar() -> Config {
    let path = caminho_config();
    match fs::read_to_string(&path) {
        Ok(txt) => serde_json::from_str(&txt).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

pub fn salvar(cfg: &Config) -> Result<()> {
    let path = caminho_config();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).context("criar diretório de configuração")?;
    }
    let txt = serde_json::to_string_pretty(cfg)?;
    fs::write(&path, txt).context("gravar configuração")?;
    Ok(())
}

fn dir_dados() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("purgar")
}

pub fn registrar_historico(categoria: &str, bytes: u64, erros: u64) {
    let dir = dir_dados();
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("historico.jsonl");
    let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&path) {
        use std::io::Write;
        let _ = writeln!(
            f,
            "{}",
            serde_json::json!({
                "ts": ts,
                "categoria": categoria,
                "bytes": bytes,
                "erros": erros,
            })
        );
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Registro {
    pub ts: String,
    pub categoria: String,
    pub bytes: u64,
    pub erros: u64,
}

pub fn historico() -> Vec<Registro> {
    let path = dir_dados().join("historico.jsonl");
    let Ok(txt) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    txt.lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}
