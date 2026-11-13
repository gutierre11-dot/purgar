use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;

use crate::cleaner::Msg;
use crate::config::Registro;

// ===================== AUTOSTART =====================

pub struct AutostartItem {
    pub nome: String,
    pub caminho: PathBuf,
    pub usuario: bool,
    pub habilitado: bool,
    pub exec: String,
}

fn parse_desktop(conteudo: &str) -> (String, bool, String) {
    let mut nome = String::new();
    let mut habilitado = true;
    let mut exec = String::new();
    for linha in conteudo.lines() {
        let l = linha.trim();
        if let Some(v) = l.strip_prefix("Name=") {
            if nome.is_empty() {
                nome = v.to_string();
            }
        } else if let Some(v) = l.strip_prefix("Hidden=") {
            habilitado = v.trim() != "true";
        } else if let Some(v) = l.strip_prefix("Exec=") {
            if exec.is_empty() {
                exec = v.to_string();
            }
        }
    }
    (nome, habilitado, exec)
}

pub fn listar_autostart() -> Vec<AutostartItem> {
    let mut itens = Vec::new();
    let usuario_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("autostart");
    let sistema_dir = PathBuf::from("/etc/xdg/autostart");

    for (dir, usuario) in [(usuario_dir, true), (sistema_dir, false)] {
        if let Ok(rd) = fs::read_dir(&dir) {
            for e in rd.flatten() {
                let caminho = e.path();
                if caminho.extension().and_then(|x| x.to_str()) != Some("desktop") {
                    continue;
                }
                if let Ok(txt) = fs::read_to_string(&caminho) {
                    let (nome, habilitado, exec) = parse_desktop(&txt);
                    itens.push(AutostartItem {
                        nome: if nome.is_empty() {
                            caminho
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into_owned()
                        } else {
                            nome
                        },
                        caminho,
                        usuario,
                        habilitado,
                        exec,
                    });
                }
            }
        }
    }
    itens.sort_by(|a, b| a.nome.to_lowercase().cmp(&b.nome.to_lowercase()));
    itens
}

/// Alterna Hidden=true/false num arquivo .desktop do usuário.
pub fn alternar_autostart(caminho: &PathBuf) -> Result<(), String> {
    let txt = fs::read_to_string(caminho).map_err(|e| e.to_string())?;
    let (_, habilitado, _) = parse_desktop(&txt);
    let novo = if habilitado {
        definir_hidden(&txt, true)
    } else {
        definir_hidden(&txt, false)
    };
    fs::write(caminho, novo).map_err(|e| e.to_string())
}

fn definir_hidden(txt: &str, ocultar: bool) -> String {
    let valor = if ocultar { "true" } else { "false" };
    let mut achou = false;
    let mut out = String::new();
    for linha in txt.lines() {
        if linha.trim_start().starts_with("Hidden=") {
            out.push_str(&format!("Hidden={}\n", valor));
            achou = true;
        } else {
            out.push_str(linha);
            out.push('\n');
        }
    }
    if !achou {
        out.push_str(&format!("Hidden={}\n", valor));
    }
    out
}

pub fn excluir_autostart(caminho: &PathBuf) -> Result<(), String> {
    fs::remove_file(caminho).map_err(|e| e.to_string())
}

// ===================== DESINSTALADOR =====================

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Origem {
    Rpm,
    Flatpak,
    Snap,
}

impl Origem {
    pub fn rotulo(self) -> &'static str {
        match self {
            Origem::Rpm => "RPM",
            Origem::Flatpak => "Flatpak",
            Origem::Snap => "Snap",
        }
    }
}

#[derive(Clone)]
pub struct Pacote {
    pub nome: String,
    pub detalhe: String,
    pub origem: Origem,
}

fn sh(caminho: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(caminho).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

pub fn listar_pacotes() -> Vec<Pacote> {
    let mut pkgs = Vec::new();

    // RPM: nome<TAB>versão
    if let Some(txt) = sh("rpm", &["-qa", "--qf", "%{NAME}\t%{VERSION}-%{RELEASE}\n"]) {
        for linha in txt.lines() {
            let mut it = linha.split('\t');
            if let (Some(nome), Some(ver)) = (it.next(), it.next()) {
                pkgs.push(Pacote {
                    nome: nome.trim().to_string(),
                    detalhe: ver.trim().to_string(),
                    origem: Origem::Rpm,
                });
            }
        }
    }

    // Flatpak: ids de aplicativos
    if let Some(txt) = sh("flatpak", &["list", "--app", "--columns=application"]) {
        for linha in txt.lines() {
            let nome = linha.trim();
            if !nome.is_empty() {
                pkgs.push(Pacote {
                    nome: nome.to_string(),
                    detalhe: "aplicativo".into(),
                    origem: Origem::Flatpak,
                });
            }
        }
    }

    // Snap: pula cabeçalho
    if let Some(txt) = sh("snap", &["list"]) {
        for linha in txt.lines().skip(1) {
            let t: Vec<&str> = linha.split_whitespace().collect();
            if t.len() >= 2 {
                pkgs.push(Pacote {
                    nome: t[0].to_string(),
                    detalhe: t[1].to_string(),
                    origem: Origem::Snap,
                });
            }
        }
    }

    pkgs.sort_by(|a, b| {
        a.origem
            .rotulo()
            .cmp(b.origem.rotulo())
            .then(a.nome.to_lowercase().cmp(&b.nome.to_lowercase()))
    });
    pkgs
}

/// Executa a desinstalação em background, transmitindo linhas de saída.
pub fn desinstalar(pacote: &Pacote, tx: Sender<Msg>) {
    let (programa, args): (&str, Vec<String>) = match pacote.origem {
        Origem::Rpm => (
            "pkexec",
            vec![
                "dnf".into(),
                "remove".into(),
                "-y".into(),
                pacote.nome.clone(),
            ],
        ),
        Origem::Flatpak => (
            "flatpak",
            vec![
                "uninstall".into(),
                "-y".into(),
                "--delete-data".into(),
                pacote.nome.clone(),
            ],
        ),
        Origem::Snap => (
            "pkexec",
            vec!["snap".into(), "remove".into(), pacote.nome.clone()],
        ),
    };

    let _ = tx.send(Msg::Linha(format!(
        "Desinstalando {} ({})...",
        pacote.nome,
        pacote.origem.rotulo()
    )));

    let mut child = match Command::new(programa)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(Msg::Fim(Err(format!("Falha ao iniciar: {}", e))));
            return;
        }
    };

    use std::io::Read;
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
    drenar(child.stdout.take(), &tx);
    drenar(child.stderr.take(), &tx);
    match child.wait() {
        Ok(st) if st.success() => {
            let _ = tx.send(Msg::Linha("Desinstalação concluída.".into()));
            let _ = tx.send(Msg::Fim(Ok(crate::model::Resumo {
                bytes: 0,
                itens: 1,
                erros: 0,
                lixeira: false,
                linhas: vec![],
            })));
        }
        Ok(st) => {
            let _ = tx.send(Msg::Fim(Err(format!("Status de saída: {}", st))));
        }
        Err(e) => {
            let _ = tx.send(Msg::Fim(Err(e.to_string())));
        }
    }
}

// ===================== ESTATÍSTICAS =====================

pub fn registros() -> Vec<Registro> {
    crate::config::historico()
}
