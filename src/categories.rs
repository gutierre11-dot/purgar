use std::path::PathBuf;

use crate::model::{Alvo, Categoria, Nivel};

pub fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
}

fn p(s: &str) -> PathBuf {
    let h = home();
    if let Some(rest) = s.strip_prefix("~/") {
        h.join(rest)
    } else {
        PathBuf::from(s)
    }
}

/// Nomes de diretórios de cache dentro de ~/.cache que pertencem a
/// categorias próprias (navegadores, dev, miniaturas) e não devem ser
/// apagados pela categoria genérica "Cache de aplicativos".
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

fn cat(
    id: &'static str,
    nome: &'static str,
    descricao: &'static str,
    nivel: Nivel,
    alvos: Vec<Alvo>,
) -> Categoria {
    Categoria {
        id,
        nome,
        descricao,
        nivel,
        alvos,
        avisos_proc: &[],
        bloqueio_proc: &[],
        informativo: false,
    }
}

pub fn categorias(caminhos_custom: &[String]) -> Vec<Categoria> {
    let mut cats = vec![
        // ======================= BÁSICO =======================
        cat(
            "cache_apps",
            "Cache de aplicativos",
            "Remove arquivos temporários gerados por programas em ~/.cache \
             (exceto navegadores, ferramentas de desenvolvimento e miniaturas, \
             que possuem categorias próprias). Os programas recriam esses \
             arquivos automaticamente — o primeiro uso pode ser um pouco mais lento.",
            Nivel::Basico,
            vec![Alvo::Esvaziar {
                dir: p("~/.cache"),
                excluir: CACHE_EXCLUIR,
            }],
        ),
        cat(
            "cache_navegadores",
            "Cache de navegadores",
            "Remove caches do Firefox, Chrome, Chromium, Brave, Edge e outros \
             navegadores (cache de páginas, shaders e service workers). \
             Senhas, histórico, cookies e favoritos NÃO são afetados. \
             Feche os navegadores antes de executar.",
            Nivel::Basico,
            vec![
                Alvo::Esvaziar {
                    dir: p("~/.cache/mozilla"),
                    excluir: &[],
                },
                Alvo::Nomes {
                    raiz: p("~/.mozilla"),
                    nomes: NOMES_CACHE_NAVEGADOR,
                    prof: 6,
                },
                Alvo::Nomes {
                    raiz: p("~/.config/google-chrome"),
                    nomes: NOMES_CACHE_NAVEGADOR,
                    prof: 4,
                },
                Alvo::Nomes {
                    raiz: p("~/.config/chromium"),
                    nomes: NOMES_CACHE_NAVEGADOR,
                    prof: 4,
                },
                Alvo::Nomes {
                    raiz: p("~/.config/BraveSoftware/Brave-Browser"),
                    nomes: NOMES_CACHE_NAVEGADOR,
                    prof: 4,
                },
                Alvo::Nomes {
                    raiz: p("~/.config/microsoft-edge"),
                    nomes: NOMES_CACHE_NAVEGADOR,
                    prof: 4,
                },
                Alvo::Nomes {
                    raiz: p("~/.config/opera"),
                    nomes: NOMES_CACHE_NAVEGADOR,
                    prof: 4,
                },
                Alvo::Nomes {
                    raiz: p("~/.config/vivaldi"),
                    nomes: NOMES_CACHE_NAVEGADOR,
                    prof: 4,
                },
                Alvo::Nomes {
                    raiz: p("~/.var/app"),
                    nomes: NOMES_CACHE_NAVEGADOR,
                    prof: 8,
                },
            ],
        ),
        cat(
            "lixeira",
            "Lixeira",
            "Esvazia a lixeira do usuário (~/.local/share/Trash). \
             Itens apagados permanentemente — essa ação não pode ser desfeita.",
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
                Alvo::Esvaziar { dir: p("~/.npm"), excluir: &[] },
                Alvo::Esvaziar { dir: p("~/.cache/pip"), excluir: &[] },
                Alvo::Esvaziar { dir: p("~/.cache/yarn"), excluir: &[] },
                Alvo::Esvaziar { dir: p("~/.cache/composer"), excluir: &[] },
                Alvo::Esvaziar { dir: p("~/.cache/go-build"), excluir: &[] },
                Alvo::Esvaziar { dir: p("~/.cache/deno"), excluir: &[] },
                Alvo::Esvaziar { dir: p("~/.bun/install/cache"), excluir: &[] },
                Alvo::Esvaziar { dir: p("~/.cargo/registry"), excluir: &[] },
                Alvo::Esvaziar { dir: p("~/.cargo/git"), excluir: &[] },
                Alvo::Esvaziar { dir: p("~/go/pkg/mod"), excluir: &[] },
                Alvo::Esvaziar { dir: p("~/.gradle/caches"), excluir: &[] },
                Alvo::Esvaziar { dir: p("~/.m2/repository"), excluir: &[] },
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
            vec![],
        ),
        // ===================== AVANÇADO =====================
        cat(
            "logs_sistema",
            "Logs do sistema",
            "Remove logs rotacionados com mais de 30 dias em /var/log \
             (arquivos .gz e numéricos). Requer privilégio de administrador.",
            Nivel::Avancado,
            vec![Alvo::ComandoRoot(vec![
                "sh".into(),
                "-c".into(),
                "find /var/log -xdev -type f \\( -name '*.gz' -o -name '*.[0-9]' -o -name '*.old' \\) -mtime +30 -delete".into(),
            ])],
        ),
        cat(
            "journal_sistema",
            "Journal do sistema",
            "Limita o journal do systemd aos registros dos últimos 7 dias \
             (journalctl --vacuum-time). Requer privilégio de administrador.",
            Nivel::Avancado,
            vec![Alvo::ComandoRoot(vec![
                "journalctl".into(),
                "--vacuum-time=7d".into(),
            ])],
        ),
        cat(
            "cache_sistema",
            "Cache de pacotes do sistema",
            "Remove caches do DNF/YUM, PackageKit e fontes em /var/cache. \
             Requer privilégio de administrador; os caches são recriados \
             automaticamente nas próximas operações.",
            Nivel::Avancado,
            vec![Alvo::ComandoRoot(vec![
                "sh".into(),
                "-c".into(),
                "rm -rf /var/cache/dnf /var/cache/yum /var/cache/PackageKit /var/cache/fontconfig".into(),
            ])],
        ),
        cat(
            "tmp_antigos",
            "Arquivos temporários antigos",
            "Remove arquivos em /tmp e /var/tmp sem uso há mais de 7 dias. \
             Diretórios de sockets ativos (.X11-unix, systemd-private-*, ssh-*) \
             são preservados. A parte do sistema requer administrador.",
            Nivel::Avancado,
            vec![
                Alvo::Antigos {
                    dirs: vec![p("/tmp")],
                    dias: 7,
                    sudo: false,
                },
                Alvo::Antigos {
                    dirs: vec![p("/var/tmp"), p("/tmp")],
                    dias: 7,
                    sudo: true,
                },
            ],
        ),
        cat(
            "dnf_cache",
            "Cache do gerenciador de pacotes",
            "Executa 'dnf clean all': remove metadados e pacotes baixados \
             em ~/.cache/dnf e /var/cache/dnf. A próxima atualização de \
             pacotes será mais lenta. Requer administrador para a parte do sistema.",
            Nivel::Avancado,
            vec![
                Alvo::Esvaziar { dir: p("~/.cache/dnf"), excluir: &[] },
                Alvo::ComandoRoot(vec!["dnf".into(), "clean".into(), "all".into()]),
            ],
        ),
        cat(
            "flatpak_cache",
            "Cache do Flatpak",
            "Remove o cache de downloads do Flatpak (~/.cache/flatpak). \
             Aplicativos e runtimes instalados não são afetados.",
            Nivel::Avancado,
            vec![Alvo::Esvaziar {
                dir: p("~/.cache/flatpak"),
                excluir: &[],
            }],
        ),
        cat(
            "coredumps",
            "Despejos de memória (coredumps)",
            "Remove registros de crashes do systemd-coredump em \
             /var/lib/systemd/coredump. Requer privilégio de administrador.",
            Nivel::Avancado,
            vec![Alvo::ComandoRoot(vec![
                "sh".into(),
                "-c".into(),
                "find /var/lib/systemd/coredump -type f -mtime +7 -delete 2>/dev/null; true".into(),
            ])],
        ),
    ];

    // downloads é informativo: aponta para ~/Downloads ou ~/Área de Trabalho/Downloads
    if let Some(c) = cats.iter_mut().find(|c| c.id == "downloads") {
        let dir = {
            let dl = home().join("Downloads");
            if dl.is_dir() {
                dl
            } else {
                home().join("Área de Trabalho").join("Downloads")
            }
        };
        c.alvos = vec![Alvo::Remover(dir)]; // somente para medição; informativo não limpa
        c.informativo = true;
    }

    // navegaadores: bloqueio se processo ativo
    if let Some(c) = cats.iter_mut().find(|c| c.id == "cache_navegadores") {
        c.bloqueio_proc = PROCESSOS_NAVEGADORES;
    }
    if let Some(c) = cats.iter_mut().find(|c| c.id == "cache_apps") {
        c.avisos_proc = PROCESSOS_NAVEGADORES;
    }

    // caminhos personalizados do usuário
    if !caminhos_custom.is_empty() {
        let alvos: Vec<Alvo> = caminhos_custom
            .iter()
            .map(|s| {
                let path = PathBuf::from(s);
                if path.is_dir() {
                    Alvo::Esvaziar {
                        dir: path,
                        excluir: &[],
                    }
                } else {
                    Alvo::Remover(path)
                }
            })
            .collect();
        cats.push(cat(
            "custom",
            "Caminhos personalizados",
            "Diretórios e arquivos adicionados manualmente nas configurações. \
             Diretórios têm apenas o conteúdo removido (a pasta é mantida).",
            Nivel::Avancado,
            alvos,
        ));
    }

    cats
}
