use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::model::{Alvo, Categoria, Nivel};
use crate::seguranca;

pub fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
}

fn p(caminho: &str) -> PathBuf {
    seguranca::expandir(caminho)
}

/// Nomes de diretórios de cache dentro de `~/.cache` que pertencem a
/// categorias próprias (navegadores, dev, miniaturas, flatpak/dnf) e não devem
/// ser apagados pela categoria genérica "Cache de aplicativos".
const CACHE_EXCLUIR: &[&str] = &[
    // navegadores
    "mozilla",
    "firefox",
    "google-chrome",
    "google-chrome-unstable",
    "chromium",
    "chromium-browser",
    "brave",
    "brave-origin",
    "microsoft-edge",
    "microsoft-edge-dev",
    "opera",
    "opera-gx",
    "vivaldi",
    "zen",
    "floorp",
    "librewolf",
    "waterfox",
    "thorium",
    "epiphany",
    "falkon",
    "qutebrowser",
    // desenvolvimento
    "pip",
    "yarn",
    "composer",
    "go-build",
    "deno",
    // miniaturas
    "thumbnails",
    "thumbnails-large",
    "thumbnails-normal",
    "thumbnails-fail",
    // categorias próprias
    "flatpak",
    "dnf",
    "JetBrains",
];

const PROCESSOS_NAVEGADORES: &[&str] = &[
    "firefox",
    "chrome",
    "chromium",
    "brave",
    "msedge",
    "microsoft-edge",
    "opera",
    "vivaldi",
    "zen",
    "zen-beta",
    "floorp",
    "librewolf",
    "waterfox",
    "thorium",
    "epiphany",
    "qutebrowser",
    "falkon",
    "navigator",
];

/// Diretórios-nome de cache procurados dentro de raízes de navegadores.
const NOMES_CACHE_NAVEGADOR: &[&str] = &[
    "cache2",
    "cache",
    "Cache",
    "Code Cache",
    "GPUCache",
    "cache8",
    "startupCache",
    "ShaderCache",
    "GrShaderCache",
    "GraphiteDawnCache",
    "DawnGraphiteCache",
    "DawnWebGPUCache",
    "Service Worker",
];

/// Caches da família Chromium que ficam em `~/.cache/<navegador>`.
const CACHE_NAVEGADORES_DIRS: &[&str] = &[
    "google-chrome",
    "google-chrome-unstable",
    "chromium",
    "chromium-browser",
    "brave",
    "brave-origin",
    "microsoft-edge",
    "microsoft-edge-dev",
    "opera",
    "opera-gx",
    "vivaldi",
    "zen",
    "floorp",
    "librewolf",
    "waterfox",
    "thorium",
    "epiphany",
    "falkon",
    "qutebrowser",
];

/// Raízes de perfis de navegadores e a profundidade máxima de busca.
const PERFIS_NAVEGADORES: &[(&str, usize)] = &[
    ("~/.mozilla", 6),
    ("~/.config/google-chrome", 4),
    ("~/.config/google-chrome-unstable", 4),
    ("~/.config/chromium", 4),
    ("~/.config/BraveSoftware/Brave-Browser", 4),
    ("~/.config/microsoft-edge", 4),
    ("~/.config/microsoft-edge-dev", 4),
    ("~/.config/opera", 4),
    ("~/.config/vivaldi", 4),
    ("~/.config/zen", 4),
    ("~/.config/librewolf", 4),
    ("~/.config/floorp", 4),
    ("~/.config/waterfox", 4),
    ("~/.config/thorium", 4),
    ("~/.config/epiphany", 4),
    ("~/.config/qutebrowser", 4),
];

/// IDs Flatpak de navegadores conhecidos (cache fica em `~/.var/app/<id>/cache`).
const FLATPAK_NAVEGADORES: &[&str] = &[
    "org.mozilla.firefox",
    "org.mozilla.firefox.BaseApp",
    "com.google.Chrome",
    "org.chromium.Chromium",
    "com.brave.Browser",
    "com.microsoft.Edge",
    "com.opera.Opera",
    "io.github.zen_browser.zen",
    "io.gitlab.librewolf-community",
    "one.ablaze.floorp",
    "net.waterfox.waterfox",
    "org.gnome.Epiphany",
    "org.qutebrowser.qutebrowser",
];

fn existe_no_path(programa: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    for dir in std::env::split_paths(&path) {
        let candidato = dir.join(programa);
        if let Ok(meta) = std::fs::metadata(&candidato)
            && meta.is_file()
            && meta.permissions().mode() & 0o111 != 0
        {
            return true;
        }
    }
    false
}

fn cat(
    id: &'static str,
    nome: &'static str,
    descricao: impl Into<String>,
    nivel: Nivel,
    alvos: Vec<Alvo>,
) -> Categoria {
    Categoria {
        id,
        nome,
        descricao: descricao.into(),
        nivel,
        alvos,
        avisos_proc: &[],
        bloqueio_proc: &[],
        informativo: false,
    }
}

/// Monta a lista de categorias disponíveis para a configuração informada.
pub fn categorias(cfg: &Config) -> Vec<Categoria> {
    let mut cats = basicas(cfg);
    cats.extend(avancadas(cfg));
    if let Some(custom) = categoria_custom(cfg) {
        cats.push(custom);
    }
    cats
}

fn basicas(_cfg: &Config) -> Vec<Categoria> {
    let mut navegadores: Vec<Alvo> = vec![Alvo::Esvaziar {
        dir: p("~/.cache/mozilla"),
        excluir: &[],
    }];
    for nome in CACHE_NAVEGADORES_DIRS {
        navegadores.push(Alvo::Esvaziar {
            dir: p(&format!("~/.cache/{nome}")),
            excluir: &[],
        });
    }
    for (raiz, prof) in PERFIS_NAVEGADORES {
        navegadores.push(Alvo::Nomes {
            raiz: p(raiz),
            nomes: NOMES_CACHE_NAVEGADOR,
            prof: *prof,
        });
    }
    for id in FLATPAK_NAVEGADORES {
        navegadores.push(Alvo::Esvaziar {
            dir: p(&format!("~/.var/app/{id}/cache")),
            excluir: &[],
        });
    }

    let mut cats = vec![
        cat(
            "cache_apps",
            "Cache de aplicativos",
            "Remove arquivos temporários gerados por programas em ~/.cache \
             (exceto navegadores, ferramentas de desenvolvimento, miniaturas, \
             Flatpak e DNF, que possuem categorias próprias). Os programas \
             recriam esses arquivos automaticamente — o primeiro uso pode ser \
             um pouco mais lento.",
            Nivel::Basico,
            vec![Alvo::Esvaziar {
                dir: p("~/.cache"),
                excluir: CACHE_EXCLUIR,
            }],
        ),
        cat(
            "cache_navegadores",
            "Cache de navegadores",
            "Remove caches do Firefox, Chrome, Chromium, Brave, Edge, Vivaldi, \
             Opera, Zen, LibreWolf, Floorp e outros navegadores — tanto em \
             ~/.cache quanto nos perfis e em instalações Flatpak. Senhas, \
             histórico, cookies e favoritos NÃO são afetados. Feche os \
             navegadores antes de executar.",
            Nivel::Basico,
            navegadores,
        ),
        cat(
            "lixeira",
            "Lixeira",
            "Esvazia a lixeira do usuário (~/.local/share/Trash). \
             Itens apagados permanentemente — essa ação não pode ser desfeita. \
             Se o modo lixeira estiver ativo, desative-o para esvaziar de fato.",
            Nivel::Basico,
            vec![
                Alvo::Esvaziar {
                    dir: p("~/.local/share/Trash/files"),
                    excluir: &[],
                },
                Alvo::Esvaziar {
                    dir: p("~/.local/share/Trash/info"),
                    excluir: &[],
                },
            ],
        ),
        cat(
            "miniaturas",
            "Miniaturas de imagens",
            "Remove as miniaturas geradas pelo sistema para imagens e vídeos. \
             Elas são recriadas automaticamente ao navegar pelas pastas.",
            Nivel::Basico,
            vec![Alvo::Esvaziar {
                dir: p("~/.cache/thumbnails"),
                excluir: &[],
            }],
        ),
        cat(
            "cache_dev",
            "Cache de desenvolvimento",
            "Limpa caches de ferramentas de programação: npm, pip, Yarn, \
             Composer, Cargo, Go, Gradle, Maven, Deno e Bun. Os pacotes \
             serão baixados novamente quando necessário.",
            Nivel::Basico,
            vec![
                Alvo::Esvaziar {
                    dir: p("~/.npm"),
                    excluir: &[],
                },
                Alvo::Esvaziar {
                    dir: p("~/.cache/pip"),
                    excluir: &[],
                },
                Alvo::Esvaziar {
                    dir: p("~/.cache/yarn"),
                    excluir: &[],
                },
                Alvo::Esvaziar {
                    dir: p("~/.cache/composer"),
                    excluir: &[],
                },
                Alvo::Esvaziar {
                    dir: p("~/.cache/go-build"),
                    excluir: &[],
                },
                Alvo::Esvaziar {
                    dir: p("~/.cache/deno"),
                    excluir: &[],
                },
                Alvo::Esvaziar {
                    dir: p("~/.bun/install/cache"),
                    excluir: &[],
                },
                Alvo::Esvaziar {
                    dir: p("~/.cargo/registry"),
                    excluir: &[],
                },
                Alvo::Esvaziar {
                    dir: p("~/.cargo/git"),
                    excluir: &[],
                },
                Alvo::Esvaziar {
                    dir: p("~/go/pkg/mod"),
                    excluir: &[],
                },
                Alvo::Esvaziar {
                    dir: p("~/.gradle/caches"),
                    excluir: &[],
                },
                Alvo::Esvaziar {
                    dir: p("~/.m2/repository"),
                    excluir: &[],
                },
            ],
        ),
        cat(
            "logs_usuario",
            "Logs de aplicativos",
            "Reduz o journal do usuário (systemd --user) mantendo os registros \
             dos últimos 7 dias. Não remove configurações nem dados pessoais.",
            Nivel::Basico,
            vec![Alvo::ComandoUser(vec![
                "journalctl".into(),
                "--user".into(),
                "--vacuum-time=7d".into(),
            ])],
        ),
        cat(
            "downloads",
            "Pasta Downloads (análise)",
            "Mostra o espaço ocupado pela pasta de downloads. \
             Categoria informativa: a limpeza deve ser feita manualmente \
             para evitar remoção acidental de arquivos importantes.",
            Nivel::Basico,
            vec![Alvo::Medir(pasta_downloads())],
        ),
    ];

    // Cache de aplicativos: aviso (não bloqueio) se navegador estiver aberto.
    if let Some(c) = cats.iter_mut().find(|c| c.id == "cache_apps") {
        c.avisos_proc = PROCESSOS_NAVEGADORES;
    }
    if let Some(c) = cats.iter_mut().find(|c| c.id == "cache_navegadores") {
        c.bloqueio_proc = PROCESSOS_NAVEGADORES;
    }
    if let Some(c) = cats.iter_mut().find(|c| c.id == "downloads") {
        c.informativo = true;
    }

    cats
}

fn pasta_downloads() -> PathBuf {
    let padrao = home().join("Downloads");
    if padrao.is_dir() {
        return padrao;
    }
    let localizado = home().join("Área de Trabalho").join("Downloads");
    if localizado.is_dir() {
        return localizado;
    }
    padrao
}

fn avancadas(cfg: &Config) -> Vec<Categoria> {
    let mut cats = Vec::new();

    cats.push(cat(
        "logs_sistema",
        "Logs do sistema",
        format!(
            "Remove logs rotacionados com mais de {} dias em /var/log \
             (arquivos .gz, numéricos e .old). Requer privilégio de administrador.",
            cfg.dias_logs
        ),
        Nivel::Avancado,
        vec![Alvo::ComandoRoot(vec![
            "sh".into(),
            "-c".into(),
            format!(
                "find /var/log -xdev -type f \\( -name '*.gz' -o -name '*.[0-9]' -o -name '*.old' \\) -mtime +{} -delete",
                cfg.dias_logs
            ),
        ])],
    ));

    cats.push(cat(
        "journal_sistema",
        "Journal do sistema",
        "Limita o journal do systemd aos registros dos últimos 7 dias \
         (journalctl --vacuum-time). Requer privilégio de administrador.",
        Nivel::Avancado,
        vec![Alvo::ComandoRoot(vec![
            "journalctl".into(),
            "--vacuum-time=7d".into(),
        ])],
    ));

    // Cache de pacotes do sistema: comandos específicos do gerenciador detectado.
    let mut alvos_sistema = Vec::new();
    if existe_no_path("dnf") {
        alvos_sistema.push(Alvo::ComandoRoot(vec![
            "dnf".into(),
            "clean".into(),
            "all".into(),
        ]));
    } else if existe_no_path("apt-get") {
        alvos_sistema.push(Alvo::ComandoRoot(vec!["apt-get".into(), "clean".into()]));
    } else if existe_no_path("pacman") {
        alvos_sistema.push(Alvo::ComandoRoot(vec![
            "pacman".into(),
            "-Sc".into(),
            "--noconfirm".into(),
        ]));
    } else if existe_no_path("zypper") {
        alvos_sistema.push(Alvo::ComandoRoot(vec![
            "zypper".into(),
            "clean".into(),
            "--all".into(),
        ]));
    }
    alvos_sistema.push(Alvo::ComandoRoot(vec![
        "sh".into(),
        "-c".into(),
        "rm -rf /var/cache/PackageKit /var/cache/fontconfig".into(),
    ]));
    cats.push(cat(
        "cache_sistema",
        "Cache de pacotes do sistema",
        "Remove caches do gerenciador de pacotes detectado (DNF, APT, Pacman ou \
         Zypper), do PackageKit e de fontes em /var/cache. Requer privilégio de \
         administrador; os caches são recriados automaticamente.",
        Nivel::Avancado,
        alvos_sistema,
    ));

    cats.push(cat(
        "tmp_antigos",
        "Arquivos temporários antigos",
        format!(
            "Remove arquivos em /tmp e /var/tmp sem uso há mais de {} dias. \
             Diretórios e sockets ativos (.X11-unix, systemd-private-*, ssh-*, \
             .dbus) são preservados; como usuário, só remove arquivos do \
             próprio usuário. A parte do sistema requer administrador.",
            cfg.dias_tmp
        ),
        Nivel::Avancado,
        vec![
            Alvo::Antigos {
                dirs: vec![p("/tmp")],
                dias: cfg.dias_tmp,
                sudo: false,
            },
            Alvo::Antigos {
                dirs: vec![p("/var/tmp"), p("/tmp")],
                dias: cfg.dias_tmp,
                sudo: true,
            },
        ],
    ));

    if existe_no_path("dnf") {
        cats.push(cat(
            "dnf_cache",
            "Cache do DNF",
            "Executa 'dnf clean all' e remove o cache do usuário em ~/.cache/dnf. \
             A próxima atualização de pacotes será mais lenta. Requer administrador \
             para a parte do sistema.",
            Nivel::Avancado,
            vec![
                Alvo::Esvaziar {
                    dir: p("~/.cache/dnf"),
                    excluir: &[],
                },
                Alvo::ComandoRoot(vec!["dnf".into(), "clean".into(), "all".into()]),
            ],
        ));
    }

    if existe_no_path("apt-get") {
        cats.push(cat(
            "apt_cache",
            "Cache do APT",
            "Executa 'apt-get clean' e remove pacotes .deb baixados em \
             /var/cache/apt/archives. A próxima instalação será mais lenta. \
             Requer administrador.",
            Nivel::Avancado,
            vec![Alvo::ComandoRoot(vec!["apt-get".into(), "clean".into()])],
        ));
    }

    if existe_no_path("flatpak") {
        cats.push(cat(
            "flatpak_cache",
            "Cache do Flatpak",
            "Remove o cache de downloads do Flatpak (~/.cache/flatpak). \
             Aplicativos e runtimes instalados não são afetados.",
            Nivel::Avancado,
            vec![Alvo::Esvaziar {
                dir: p("~/.cache/flatpak"),
                excluir: &[],
            }],
        ));
        cats.push(cat(
            "flatpak_unused",
            "Runtimes não utilizados do Flatpak",
            "Remove runtimes do Flatpak que nenhum aplicativo instalado usa \
             (flatpak uninstall --unused). Libera espaço sem afetar aplicativos.",
            Nivel::Avancado,
            vec![Alvo::ComandoUser(vec![
                "flatpak".into(),
                "uninstall".into(),
                "--unused".into(),
                "-y".into(),
            ])],
        ));
    }

    if Path::new("/var/lib/systemd/coredump").is_dir() {
        cats.push(cat(
            "coredumps",
            "Despejos de memória (coredumps)",
            "Remove registros de crashes do systemd-coredump com mais de 7 dias \
             em /var/lib/systemd/coredump. Requer privilégio de administrador.",
            Nivel::Avancado,
            vec![Alvo::Antigos {
                dirs: vec![PathBuf::from("/var/lib/systemd/coredump")],
                dias: 7,
                sudo: true,
            }],
        ));
    }

    if existe_no_path("docker") {
        cats.push(cat(
            "docker_cache",
            "Cache do Docker",
            "Executa 'docker system prune -f': remove containers parados, redes \
             e imagens não utilizadas (dangling). Volumes e imagens em uso são \
             preservados. Requer administrador.",
            Nivel::Avancado,
            vec![Alvo::ComandoRoot(vec![
                "docker".into(),
                "system".into(),
                "prune".into(),
                "-f".into(),
            ])],
        ));
    }

    // Caches de editores/IDEs: só entram na lista se existirem no sistema.
    let mut vscode_alvos = Vec::new();
    for base in [
        "~/.config/Code",
        "~/.config/Code - Insiders",
        "~/.config/VSCodium",
    ] {
        for sub in ["Cache", "CachedData", "GPUCache", "Code Cache", "logs"] {
            let dir = p(&format!("{base}/{sub}"));
            if dir.exists() {
                vscode_alvos.push(Alvo::Esvaziar { dir, excluir: &[] });
            }
        }
    }
    if !vscode_alvos.is_empty() {
        cats.push(cat(
            "vscode_cache",
            "Cache do VS Code",
            "Remove caches do Visual Studio Code, VSCodium e Insiders \
             (Cache, CachedData, GPUCache, logs). Extensões e configurações \
             não são afetadas.",
            Nivel::Avancado,
            vscode_alvos,
        ));
    }

    let mut jetbrains_alvos = Vec::new();
    let jetbrains = p("~/.cache/JetBrains");
    if jetbrains.is_dir() {
        jetbrains_alvos.push(Alvo::Esvaziar {
            dir: jetbrains,
            excluir: &[],
        });
    }
    // Android Studio e afins em ~/.cache/Google/<produto>/<versao>/caches.
    let google = p("~/.cache/Google");
    if google.is_dir() {
        jetbrains_alvos.push(Alvo::Nomes {
            raiz: google,
            nomes: &["caches", "Caches", "log", "logs"],
            prof: 3,
        });
    }
    let local = p("~/.local/share/JetBrains");
    if local.is_dir() {
        jetbrains_alvos.push(Alvo::Nomes {
            raiz: local,
            nomes: &["caches", "Caches", "log", "logs"],
            prof: 2,
        });
    }
    if !jetbrains_alvos.is_empty() {
        cats.push(cat(
            "jetbrains_cache",
            "Cache de IDEs JetBrains",
            "Remove caches e logs de IDEs JetBrains (IntelliJ IDEA, PyCharm, \
             WebStorm, RustRover, Android Studio). Projetos e configurações \
             não são afetados.",
            Nivel::Avancado,
            jetbrains_alvos,
        ));
    }

    cats
}

fn categoria_custom(cfg: &Config) -> Option<Categoria> {
    if cfg.caminhos.is_empty() {
        return None;
    }
    let mut alvos = Vec::new();
    for bruto in &cfg.caminhos {
        let expandido = seguranca::expandir(bruto);
        match seguranca::validar_caminho_custom(&expandido) {
            Ok(caminho) => {
                if caminho.is_dir() {
                    alvos.push(Alvo::Esvaziar {
                        dir: caminho,
                        excluir: &[],
                    });
                } else {
                    alvos.push(Alvo::Remover(caminho));
                }
            }
            Err(erro) => {
                eprintln!("aviso: caminho personalizado ignorado: {erro}");
            }
        }
    }
    if alvos.is_empty() {
        return None;
    }
    Some(cat(
        "custom",
        "Caminhos personalizados",
        "Diretórios e arquivos adicionados manualmente nas configurações. \
         Diretórios têm apenas o conteúdo removido (a pasta é mantida). \
         Caminhos do sistema e a própria home são rejeitados por segurança.",
        Nivel::Avancado,
        alvos,
    ))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn ids_sao_unicos() {
        let cats = categorias(&Config::default());
        let mut ids: Vec<&str> = cats.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        let antes = ids.len();
        ids.dedup();
        assert_eq!(antes, ids.len(), "ids duplicados: {ids:?}");
    }

    #[test]
    fn categorias_basicas_tem_alvos_ou_sao_informativas() {
        for c in categorias(&Config::default()) {
            assert!(
                !c.alvos.is_empty() || c.informativo,
                "categoria {} sem alvos e não informativa",
                c.id
            );
        }
    }

    #[test]
    fn downloads_e_informativa_e_nunca_remove() {
        let cats = categorias(&Config::default());
        let downloads = cats.iter().find(|c| c.id == "downloads").unwrap();
        assert!(downloads.informativo);
        assert!(matches!(downloads.alvos[0], Alvo::Medir(_)));
    }

    #[test]
    fn cobre_cache_chromium_em_ponto_cache() {
        let cats = categorias(&Config::default());
        let nav = cats.iter().find(|c| c.id == "cache_navegadores").unwrap();
        let cobre = nav.alvos.iter().any(|a| match a {
            Alvo::Esvaziar { dir, .. } => dir.ends_with(".cache/google-chrome"),
            _ => false,
        });
        assert!(
            cobre,
            "cache_navegadores deve cobrir ~/.cache/google-chrome"
        );
    }

    #[test]
    fn caminho_custom_invalido_e_ignorado() {
        let mut cfg = Config::default();
        cfg.caminhos.push("/".into());
        cfg.caminhos.push("/usr".into());
        let cats = categorias(&cfg);
        assert!(!cats.iter().any(|c| c.id == "custom"));
    }

    #[test]
    fn requer_admin_e_derivado_dos_alvos() {
        let cats = categorias(&Config::default());
        let logs = cats.iter().find(|c| c.id == "logs_sistema").unwrap();
        assert!(logs.requer_admin());
        let apps = cats.iter().find(|c| c.id == "cache_apps").unwrap();
        assert!(!apps.requer_admin());
    }

    #[test]
    fn nao_duplica_limpeza_de_flatpak_no_cache_generico() {
        assert!(CACHE_EXCLUIR.contains(&"flatpak"));
        assert!(CACHE_EXCLUIR.contains(&"dnf"));
    }
}
