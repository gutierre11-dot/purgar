use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;

use crate::cleaner::Msg;
use crate::model::Resumo;
use crate::util::drenar;

// ===================== AUTOSTART =====================

pub struct AutostartItem {
    pub nome: String,
    pub caminho: PathBuf,
    pub usuario: bool,
    pub habilitado: bool,
    pub exec: String,
}

#[derive(Default)]
struct EntradaDesktop {
    nome: String,
    nome_pt: bool,
    exec: String,
    hidden: bool,
    x_gnome: Option<bool>,
}

fn parse_entrada(conteudo: &str) -> EntradaDesktop {
    let mut entrada = EntradaDesktop::default();
    let mut grupo = String::new();
    for linha in conteudo.lines() {
        let l = linha.trim();
        if l.starts_with('[') && l.ends_with(']') {
            grupo = l.to_string();
            continue;
        }
        if grupo != "[Desktop Entry]" {
            continue;
        }
        if let Some(v) = l.strip_prefix("Name[pt_BR]=") {
            entrada.nome = v.trim().to_string();
            entrada.nome_pt = true;
        } else if let Some(v) = l.strip_prefix("Name=") {
            if !entrada.nome_pt && entrada.nome.is_empty() {
                entrada.nome = v.trim().to_string();
            }
        } else if let Some(v) = l.strip_prefix("Exec=") {
            if entrada.exec.is_empty() {
                entrada.exec = v.trim().to_string();
            }
        } else if let Some(v) = l.strip_prefix("Hidden=") {
            entrada.hidden = v.trim().eq_ignore_ascii_case("true");
        } else if let Some(v) = l.strip_prefix("X-GNOME-Autostart-enabled=") {
            entrada.x_gnome = Some(!v.trim().eq_ignore_ascii_case("false"));
        }
    }
    entrada
}

fn habilitado(entrada: &EntradaDesktop) -> bool {
    !entrada.hidden && entrada.x_gnome.unwrap_or(true)
}

pub fn listar_autostart() -> Vec<AutostartItem> {
    let mut itens = Vec::new();
    let usuario_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("autostart");
    let sistema_dir = PathBuf::from("/etc/xdg/autostart");

    let nomes_usuario: std::collections::HashSet<String> = fs::read_dir(&usuario_dir)
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| e.file_name().to_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    for (dir, usuario) in [(usuario_dir, true), (sistema_dir, false)] {
        let Ok(rd) = fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let caminho = e.path();
            if caminho.extension().and_then(|x| x.to_str()) != Some("desktop") {
                continue;
            }
            // Entrada do usuário sobrescreve a do sistema com o mesmo nome.
            if !usuario
                && let Some(nome) = caminho.file_name().and_then(|n| n.to_str())
                && nomes_usuario.contains(nome)
            {
                continue;
            }
            let Ok(txt) = fs::read_to_string(&caminho) else {
                continue;
            };
            let entrada = parse_entrada(&txt);
            let habilitado_agora = habilitado(&entrada);
            let nome = if entrada.nome.is_empty() {
                caminho
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default()
            } else {
                entrada.nome
            };
            itens.push(AutostartItem {
                nome,
                caminho,
                usuario,
                habilitado: habilitado_agora,
                exec: entrada.exec,
            });
        }
    }
    itens.sort_by_key(|a| a.nome.to_lowercase());
    itens
}

/// Define o estado de inicialização dentro do grupo `[Desktop Entry]`,
/// preservando os demais grupos e chaves do arquivo.
pub fn aplicar_habilitado(txt: &str, habilitar: bool) -> String {
    let valor_hidden = if habilitar { "false" } else { "true" };
    let valor_gnome = if habilitar { "true" } else { "false" };
    let mut out = String::with_capacity(txt.len() + 64);
    let mut dentro = false;
    let mut achou_grupo = false;
    let mut viu_hidden = false;
    let mut viu_gnome = false;
    let finalizar_grupo = |out: &mut String, viu_hidden: bool, viu_gnome: bool| {
        if !viu_hidden {
            out.push_str(&format!("Hidden={valor_hidden}\n"));
        }
        if !viu_gnome {
            out.push_str(&format!("X-GNOME-Autostart-enabled={valor_gnome}\n"));
        }
    };

    for linha in txt.lines() {
        let t = linha.trim();
        if t.starts_with('[') && t.ends_with(']') {
            if dentro {
                finalizar_grupo(&mut out, viu_hidden, viu_gnome);
            }
            dentro = t == "[Desktop Entry]";
            if dentro {
                achou_grupo = true;
                viu_hidden = false;
                viu_gnome = false;
            }
            out.push_str(linha);
            out.push('\n');
            continue;
        }
        if dentro {
            if t.starts_with("Hidden=") {
                out.push_str(&format!("Hidden={valor_hidden}\n"));
                viu_hidden = true;
                continue;
            }
            if t.starts_with("X-GNOME-Autostart-enabled=") {
                out.push_str(&format!("X-GNOME-Autostart-enabled={valor_gnome}\n"));
                viu_gnome = true;
                continue;
            }
        }
        out.push_str(linha);
        out.push('\n');
    }
    if dentro {
        finalizar_grupo(&mut out, viu_hidden, viu_gnome);
    }
    if !achou_grupo {
        out.push_str("\n[Desktop Entry]\n");
        finalizar_grupo(&mut out, false, false);
    }
    out
}

/// Alterna o estado de inicialização de um arquivo `.desktop` do usuário.
pub fn alternar_autostart(caminho: &Path) -> Result<(), String> {
    let txt = fs::read_to_string(caminho).map_err(|e| e.to_string())?;
    let entrada = parse_entrada(&txt);
    let novo = aplicar_habilitado(&txt, !habilitado(&entrada));
    fs::write(caminho, novo).map_err(|e| e.to_string())
}

pub fn excluir_autostart(caminho: &Path) -> Result<(), String> {
    fs::remove_file(caminho).map_err(|e| e.to_string())
}

// ===================== DESINSTALADOR =====================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

#[derive(Clone, Debug)]
pub struct Pacote {
    pub nome: String,
    pub detalhe: String,
    pub origem: Origem,
}

/// Pacotes que nunca devem ser removidos pelo desinstalador.
///
/// A lista cobre gerenciadores de pacotes, kernel, init, bibliotecas base e
/// runtimes essenciais do Flatpak/Snap.
pub fn pacote_critico(nome: &str) -> bool {
    let nome = nome.to_lowercase();
    const EXATOS: &[&str] = &[
        "core",
        "core18",
        "core20",
        "core22",
        "core24",
        "bare",
        "snapd",
        "flatpak",
        "dnf",
        "rpm",
        "bash",
        "sudo",
        "systemd",
        "kernel",
        "glibc",
        "glib2",
        "polkit",
        "dbus",
        "grub2",
        "shim",
        "gnome-shell",
        "plasma-workspace",
        "kwin",
        "sddm",
        "gdm",
        "networkmanager",
        "firewalld",
        "openssh-server",
        "selinux-policy",
        "ca-certificates",
    ];
    const PREFIXOS: &[&str] = &[
        "kernel-",
        "systemd-",
        "glibc-",
        "gnome-shell-",
        "plasma-workspace-",
        "networkmanager-",
        "selinux-policy-",
        "ca-certificates-",
        "grub2-",
        "shim-",
        "polkit-",
        "dbus-",
        "snapd-",
        "dnf-",
        "rpm-",
    ];
    EXATOS.contains(&nome.as_str()) || PREFIXOS.iter().any(|p| nome.starts_with(p))
}

fn sh(programa: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(programa).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Lista pacotes RPM, Flatpak e Snap disponíveis no sistema.
///
/// Deve ser chamada em uma thread separada: `rpm -qa`, `flatpak list` e
/// `snap list` podem levar segundos.
pub fn listar_pacotes() -> Vec<Pacote> {
    let mut pkgs = Vec::new();

    if let Some(txt) = sh("rpm", &["-qa", "--qf", "%{NAME}\t%{VERSION}-%{RELEASE}\n"]) {
        for linha in txt.lines() {
            let mut it = linha.split('\t');
            if let (Some(nome), Some(ver)) = (it.next(), it.next()) {
                let nome = nome.trim();
                if nome.is_empty() {
                    continue;
                }
                pkgs.push(Pacote {
                    nome: nome.to_string(),
                    detalhe: ver.trim().to_string(),
                    origem: Origem::Rpm,
                });
            }
        }
    }

    if let Some(txt) = sh(
        "flatpak",
        &["list", "--app", "--columns=application,version"],
    ) {
        for linha in txt.lines() {
            let mut it = linha.split('\t');
            let nome = it.next().unwrap_or("").trim();
            if nome.is_empty() {
                continue;
            }
            let versao = it.next().unwrap_or("").trim();
            pkgs.push(Pacote {
                nome: nome.to_string(),
                detalhe: if versao.is_empty() {
                    "aplicativo".into()
                } else {
                    versao.to_string()
                },
                origem: Origem::Flatpak,
            });
        }
    }

    if let Some(txt) = sh("snap", &["list"]) {
        let mut linhas = txt.lines();
        let cabecalho: Vec<&str> = linhas
            .next()
            .map(|l| l.split_whitespace().collect())
            .unwrap_or_default();
        let idx_nome = cabecalho.iter().position(|c| *c == "Name").unwrap_or(0);
        let idx_versao = cabecalho.iter().position(|c| *c == "Version").unwrap_or(1);
        for linha in linhas {
            let colunas: Vec<&str> = linha.split_whitespace().collect();
            if let Some(nome) = colunas.get(idx_nome) {
                pkgs.push(Pacote {
                    nome: nome.to_string(),
                    detalhe: colunas
                        .get(idx_versao)
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "snap".into()),
                    origem: Origem::Snap,
                });
            }
        }
    }

    pkgs.sort_by(|a, b| {
        a.origem
            .rotulo()
            .cmp(b.origem.rotulo())
            .then_with(|| a.nome.to_lowercase().cmp(&b.nome.to_lowercase()))
    });
    pkgs
}

/// Executa a desinstalação em background, transmitindo linhas de saída.
pub fn desinstalar(pacote: &Pacote, apagar_dados: bool, tx: Sender<Msg>) {
    if pacote_critico(&pacote.nome) {
        let _ = tx.send(Msg::Linha(format!(
            "'{}' é um pacote essencial do sistema e não pode ser removido.",
            pacote.nome
        )));
        let _ = tx.send(Msg::Fim(Err(
            "pacote essencial protegido pelo purgar".into()
        )));
        return;
    }

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
        Origem::Flatpak => {
            let mut args: Vec<String> = vec!["uninstall".into(), "-y".into(), pacote.nome.clone()];
            if apagar_dados {
                args.insert(2, "--delete-data".into());
            }
            ("flatpak", args)
        }
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
    let _ = tx.send(Msg::Linha(format!(
        "Executando: {programa} {}",
        args.join(" ")
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
            let _ = tx.send(Msg::Fim(Err(format!("falha ao iniciar: {e}"))));
            return;
        }
    };

    drenar(child.stdout.take(), &tx);
    drenar(child.stderr.take(), &tx);
    match child.wait() {
        Ok(status) if status.success() => {
            let _ = tx.send(Msg::Linha("Desinstalação concluída.".into()));
            let _ = tx.send(Msg::Fim(Ok(Resumo {
                itens: 1,
                ..Resumo::default()
            })));
        }
        Ok(status) => {
            let _ = tx.send(Msg::Fim(Err(format!("status de saída: {status}"))));
        }
        Err(e) => {
            let _ = tx.send(Msg::Fim(Err(e.to_string())));
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn parse_entrada_prefere_nome_pt_br() {
        let txt =
            "[Desktop Entry]\nName=Files\nName[pt_BR]=Arquivos\nExec=nautilus %U\nHidden=false\n";
        let e = parse_entrada(txt);
        assert_eq!(e.nome, "Arquivos");
        assert_eq!(e.exec, "nautilus %U");
        assert!(habilitado(&e));
    }

    #[test]
    fn alternancia_fica_no_grupo_desktop_entry() {
        let txt = "[Desktop Entry]\nType=Application\nName=App\nExec=app\n\n[Desktop Action new]\nName=New\nExec=app --new\n";
        let nova = aplicar_habilitado(txt, false);
        let pos_hidden = nova.find("Hidden=true").expect("Hidden ausente");
        let pos_acao = nova.find("[Desktop Action new]").expect("ação ausente");
        assert!(
            pos_hidden < pos_acao,
            "Hidden deve ficar no grupo [Desktop Entry]"
        );
        assert!(nova.contains("X-GNOME-Autostart-enabled=false"));
        // A chave não pode cair no grupo de ação.
        let depois_acao = nova.split("[Desktop Action new]").nth(1).unwrap();
        assert!(!depois_acao.contains("Hidden="));
        let reabilitado = aplicar_habilitado(&nova, true);
        assert!(reabilitado.contains("Hidden=false"));
        assert!(reabilitado.contains("X-GNOME-Autostart-enabled=true"));
    }

    #[test]
    fn alternancia_substitui_chaves_existentes() {
        let txt = "[Desktop Entry]\nHidden=true\nX-GNOME-Autostart-enabled=false\n";
        let nova = aplicar_habilitado(txt, true);
        assert_eq!(nova.matches("Hidden=").count(), 1);
        assert!(nova.contains("Hidden=false"));
        assert!(nova.contains("X-GNOME-Autostart-enabled=true"));
    }

    #[test]
    fn pacotes_criticos_sao_protegidos() {
        for nome in ["kernel-core", "systemd", "glibc", "dnf", "snapd", "core22"] {
            assert!(pacote_critico(nome), "{nome} deveria ser protegido");
        }
        for nome in ["firefox", "neovim", "htop", "spotify"] {
            assert!(!pacote_critico(nome), "{nome} não deveria ser protegido");
        }
    }
}
