//! Validação de caminhos: barreira de segurança para caminhos personalizados.
//!
//! O usuário pode adicionar qualquer diretório/arquivo em "Caminhos
//! personalizados" pela TUI ou pelo `config.json`. Sem validação, um caminho
//! como `/`, `~` ou `/home` esvaziaria o sistema inteiro. Este módulo
//! canonicaliza, verifica a raiz e impede alvos perigosos.

use std::fs;
use std::path::{Path, PathBuf};

/// Caminhos que nunca podem ser usados como alvo de limpeza.
pub const CAMINHOS_PROTEGIDOS: &[&str] = &[
    "/",
    "/bin",
    "/boot",
    "/dev",
    "/etc",
    "/home",
    "/lib",
    "/lib64",
    "/media",
    "/mnt",
    "/nix",
    "/opt",
    "/proc",
    "/root",
    "/run",
    "/sbin",
    "/snap",
    "/srv",
    "/sys",
    "/tmp",
    "/usr",
    "/usr/local",
    "/var",
    "/var/lib",
    "/var/log",
    "/var/tmp",
];

/// Expande `~`, `~/...`, `$HOME/...` e `${HOME}/...` em um caminho.
pub fn expandir(caminho: &str) -> PathBuf {
    let caminho = caminho.trim();
    let home = dirs::home_dir();
    if caminho == "~" {
        return home.unwrap_or_else(|| PathBuf::from("/"));
    }
    for prefixo in ["~/", "$HOME/", "${HOME}/"] {
        if let Some(resto) = caminho.strip_prefix(prefixo)
            && let Some(home) = home
        {
            return home.join(resto);
        }
    }
    PathBuf::from(caminho)
}

/// `true` se o caminho canônico é uma raiz protegida.
pub fn caminho_protegido(canonico: &Path) -> bool {
    CAMINHOS_PROTEGIDOS.iter().any(|p| canonico == Path::new(p))
}

/// Valida um caminho configurado pelo usuário para limpeza.
///
/// Devolve o caminho canônico (symlinks resolvidos) em caso de sucesso.
pub fn validar_caminho_custom(caminho: &Path) -> Result<PathBuf, String> {
    if caminho.as_os_str().is_empty() {
        return Err("caminho vazio".into());
    }

    let meta = fs::symlink_metadata(caminho).map_err(|_| {
        format!(
            "'{}' não existe ou não pode ser acessado",
            caminho.display()
        )
    })?;
    if !meta.is_dir() && !meta.is_file() {
        return Err(format!(
            "'{}' não é um arquivo ou diretório comum",
            caminho.display()
        ));
    }

    let canonico = fs::canonicalize(caminho)
        .map_err(|e| format!("não foi possível resolver '{}': {e}", caminho.display()))?;

    if !canonico.is_absolute() {
        return Err(format!("'{}' não é um caminho absoluto", compat(&canonico)));
    }
    if caminho_protegido(&canonico) {
        return Err(format!(
            "'{}' é um diretório do sistema e não pode ser limpo",
            compat(&canonico)
        ));
    }
    if let Some(home) = dirs::home_dir() {
        let home_canon = fs::canonicalize(&home).unwrap_or(home);
        if canonico == home_canon {
            return Err("a própria pasta home não pode ser usada como alvo".into());
        }
        if home_canon.starts_with(&canonico) {
            return Err(format!(
                "'{}' é um diretório acima da sua home",
                compat(&canonico)
            ));
        }
    }
    // Exige profundidade mínima: raiz + 2 componentes (ex.: /tmp/algo).
    if canonico.components().count() < 3 {
        return Err(format!(
            "'{}' é amplo demais; escolha um subdiretório",
            compat(&canonico)
        ));
    }
    Ok(canonico)
}

/// Confere se um diretório-raiz de alvo interno pode ser limpo.
///
/// Usado como defesa em profundidade para alvos fixos (`~/.cache`, etc.):
/// se o diretório for um symlink para uma raiz protegida, a limpeza é pulada.
pub fn dir_raiz_permitido(dir: &Path) -> bool {
    let Ok(canonico) = fs::canonicalize(dir) else {
        // Inexistente: não há nada a limpar.
        return true;
    };
    if caminho_protegido(&canonico) {
        return false;
    }
    if let Some(home) = dirs::home_dir() {
        let home_canon = fs::canonicalize(&home).unwrap_or(home);
        if canonico == home_canon {
            return false;
        }
    }
    true
}

fn compat(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn expande_til_e_home() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(expandir("~"), home);
        assert_eq!(expandir("~/x"), home.join("x"));
        assert_eq!(expandir("$HOME/y"), home.join("y"));
        assert_eq!(expandir("${HOME}/z"), home.join("z"));
        assert_eq!(expandir("/abs"), PathBuf::from("/abs"));
    }

    #[test]
    fn rejeita_raizes_do_sistema() {
        for p in ["/", "/usr", "/etc", "/home", "/tmp", "/var", "/boot"] {
            assert!(
                validar_caminho_custom(Path::new(p)).is_err(),
                "deveria rejeitar {p}"
            );
        }
    }
}
