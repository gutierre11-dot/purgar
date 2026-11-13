use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const DIAS_TMP_PADRAO: u64 = 7;
pub const DIAS_LOGS_PADRAO: u64 = 30;
pub const HISTORICO_MAX_PADRAO: usize = 500;

/// Preferências do usuário (`~/.config/purgar/config.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// Se true, itens removidos vão para a lixeira em vez de serem apagados.
    #[serde(default)]
    pub usar_lixeira: bool,
    /// Caminhos personalizados adicionados pelo usuário.
    #[serde(default)]
    pub caminhos: Vec<String>,
    /// Idade mínima (dias) para limpar /tmp e /var/tmp.
    #[serde(default = "dias_tmp_padrao")]
    pub dias_tmp: u64,
    /// Idade mínima (dias) para remover logs rotacionados do sistema.
    #[serde(default = "dias_logs_padrao")]
    pub dias_logs: u64,
    /// Máximo de registros mantidos no histórico.
    #[serde(default = "historico_max_padrao")]
    pub historico_max: usize,
    /// Se true, desinstalação de Flatpak também remove os dados do aplicativo.
    #[serde(default)]
    pub apagar_dados_flatpak: bool,
}

fn dias_tmp_padrao() -> u64 {
    DIAS_TMP_PADRAO
}

fn dias_logs_padrao() -> u64 {
    DIAS_LOGS_PADRAO
}

fn historico_max_padrao() -> usize {
    HISTORICO_MAX_PADRAO
}

impl Default for Config {
    fn default() -> Self {
        Self {
            usar_lixeira: false,
            caminhos: Vec::new(),
            dias_tmp: DIAS_TMP_PADRAO,
            dias_logs: DIAS_LOGS_PADRAO,
            historico_max: HISTORICO_MAX_PADRAO,
            apagar_dados_flatpak: false,
        }
    }
}

/// Diretório de configuração: `$XDG_CONFIG_HOME/purgar`.
pub fn dir_config() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("purgar")
}

/// Diretório de dados: `$XDG_DATA_HOME/purgar`.
pub fn dir_dados() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("purgar")
}

/// Caminho do arquivo de configuração.
pub fn caminho_config() -> PathBuf {
    dir_config().join("config.json")
}

/// Caminho do arquivo de histórico.
pub fn caminho_historico() -> PathBuf {
    dir_dados().join("historico.jsonl")
}

/// Carrega a configuração padrão do usuário.
pub fn carregar() -> Config {
    carregar_de(&dir_config())
}

/// Carrega a configuração de um diretório arbitrário (usado em testes).
///
/// Se o arquivo estiver corrompido, ele é preservado como `config.json.invalido`
/// e um aviso é emitido — nunca sobrescreve silenciosamente.
pub fn carregar_de(dir: &Path) -> Config {
    let path = dir.join("config.json");
    let Ok(txt) = fs::read_to_string(&path) else {
        return Config::default();
    };
    match serde_json::from_str::<Config>(&txt) {
        Ok(cfg) => cfg,
        Err(erro) => {
            let backup = path.with_extension("json.invalido");
            let _ = fs::rename(&path, &backup);
            eprintln!(
                "aviso: configuração inválida em {} ({erro}). \
                 Backup salvo em {}; usando padrões.",
                path.display(),
                backup.display()
            );
            Config::default()
        }
    }
}

/// Grava a configuração padrão do usuário de forma atômica.
pub fn salvar(cfg: &Config) -> Result<()> {
    salvar_em(&dir_config(), cfg)
}

/// Grava a configuração em um diretório arbitrário (usado em testes).
pub fn salvar_em(dir: &Path, cfg: &Config) -> Result<()> {
    fs::create_dir_all(dir).context("criar diretório de configuração")?;
    let path = dir.join("config.json");
    let tmp = dir.join(format!("config.json.tmp.{}", std::process::id()));
    let mut txt = serde_json::to_string_pretty(cfg)?;
    txt.push('\n');
    fs::write(&tmp, txt).context("gravar configuração temporária")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600));
    }
    fs::rename(&tmp, &path).context("publicar configuração")?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registro {
    pub ts: String,
    pub categoria: String,
    pub bytes: u64,
    #[serde(default)]
    pub movidos: u64,
    #[serde(default)]
    pub erros: u64,
}

/// Registra uma operação de limpeza no histórico padrão.
pub fn registrar_historico(categoria: &str, bytes: u64, movidos: u64, erros: u64, max: usize) {
    registrar_historico_em(&dir_dados(), categoria, bytes, movidos, erros, max);
}

/// Registra uma operação em um diretório de dados arbitrário (usado em testes).
pub fn registrar_historico_em(
    dir: &Path,
    categoria: &str,
    bytes: u64,
    movidos: u64,
    erros: u64,
    max: usize,
) {
    if fs::create_dir_all(dir).is_err() {
        return;
    }
    let path = dir.join("historico.jsonl");
    let ts = chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let linha = serde_json::json!({
        "ts": ts,
        "categoria": categoria,
        "bytes": bytes,
        "movidos": movidos,
        "erros": erros,
    });
    use std::io::Write;
    let gravado = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| writeln!(f, "{linha}"))
        .is_ok();
    if gravado {
        rotacionar_historico(&path, max);
    }
}

fn rotacionar_historico(path: &Path, max: usize) {
    if max == 0 {
        return;
    }
    let Ok(txt) = fs::read_to_string(path) else {
        return;
    };
    let linhas: Vec<&str> = txt.lines().filter(|l| !l.trim().is_empty()).collect();
    if linhas.len() <= max {
        return;
    }
    let inicio = linhas.len() - max;
    let mut novo = linhas[inicio..].join("\n");
    novo.push('\n');
    let tmp = path.with_extension(format!("jsonl.tmp.{}", std::process::id()));
    if fs::write(&tmp, novo).is_ok() {
        let _ = fs::rename(&tmp, path);
    }
}

/// Lê o histórico padrão do usuário.
pub fn historico() -> Vec<Registro> {
    historico_em(&dir_dados())
}

/// Lê o histórico de um diretório de dados arbitrário (usado em testes).
pub fn historico_em(dir: &Path) -> Vec<Registro> {
    let path = dir.join("historico.jsonl");
    let Ok(txt) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    txt.lines()
        .filter_map(|l| serde_json::from_str::<Registro>(l).ok())
        .collect()
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp(nome: &str) -> PathBuf {
        let carimbo = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("purgar-config-{nome}-{carimbo}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn roundtrip_de_configuracao() {
        let dir = temp("roundtrip");
        let mut cfg = Config {
            usar_lixeira: true,
            dias_tmp: 3,
            dias_logs: 10,
            historico_max: 5,
            apagar_dados_flatpak: true,
            ..Config::default()
        };
        cfg.caminhos.push("/tmp/x".into());
        salvar_em(&dir, &cfg).unwrap();
        assert_eq!(carregar_de(&dir), cfg);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_corrompida_e_preservada() {
        let dir = temp("corrompida");
        fs::write(dir.join("config.json"), "{ isto não é json").unwrap();
        let cfg = carregar_de(&dir);
        assert_eq!(cfg, Config::default());
        assert!(dir.join("config.json.invalido").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_ausente_usa_padroes() {
        let dir = temp("ausente");
        assert_eq!(carregar_de(&dir), Config::default());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn historico_registra_e_rotaciona() {
        let dir = temp("historico");
        for i in 0..10 {
            registrar_historico_em(&dir, "cache_dev", i, 0, 0, 3);
        }
        let registros = historico_em(&dir);
        assert_eq!(registros.len(), 3);
        assert_eq!(registros.last().unwrap().bytes, 9);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn historico_aceita_registros_antigos_sem_movidos() {
        let dir = temp("compat");
        fs::write(
            dir.join("historico.jsonl"),
            "{\"ts\":\"2026-01-01 00:00:00\",\"categoria\":\"lixeira\",\"bytes\":10,\"erros\":0}\n",
        )
        .unwrap();
        let registros = historico_em(&dir);
        assert_eq!(registros.len(), 1);
        assert_eq!(registros[0].movidos, 0);
        let _ = fs::remove_dir_all(&dir);
    }
}
