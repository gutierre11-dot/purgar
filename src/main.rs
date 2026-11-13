mod categories;
mod cleaner;
mod config;
mod fsutil;
mod model;
mod tools;
mod tui;

use std::process::exit;

use anyhow::Result;

const AJUDA: &str = "\
purgar — limpeza de cache e arquivos temporários

USO:
    purgar                     Abre a interface TUI
    purgar scan                Analisa todas as categorias e mostra tamanhos
    purgar clean <id>          Executa a limpeza de uma categoria
    purgar lista               Lista as categorias disponíveis
    purgar --help              Mostra esta ajuda

CATEGORIAS (id):
";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cfg = config::carregar();
    let cats = categories::categorias(&cfg.caminhos);

    match args.first().map(|s| s.as_str()) {
        None => tui::run(),
        Some("--help") | Some("-h") | Some("help") => {
            println!("{}", AJUDA);
            for c in &cats {
                println!("  {:<18} {}", c.id, c.nome);
            }
            Ok(())
        }
        Some("lista") => {
            for c in &cats {
                println!(
                    "{:>8}  {:<18}  {}",
                    match c.nivel {
                        model::Nivel::Basico => "básico",
                        model::Nivel::Avancado => "avanç.",
                    },
                    c.id,
                    c.nome
                );
            }
            Ok(())
        }
        Some("scan") => {
            // scan em paralelo
            use rayon::prelude::*;
            let mut resultados: Vec<(&str, &str, model::ScanResult)> = cats
                .par_iter()
                .map(|c| (c.id, c.nome, cleaner::escanear(c)))
                .collect();
            resultados.sort_by_key(|(id, _, _)| *id);
            println!("{:<22} {:>12}  {}", "CATEGORIA", "TAMANHO", "ITENS");
            println!("{}", "─".repeat(60));
            let mut total = 0u64;
            for (id, nome, res) in &resultados {
                println!(
                    "{:<22} {:>12}  {}",
                    format!("{} ({})", nome, id),
                    fsutil::humano(res.bytes),
                    res.itens
                );
                total += res.bytes;
            }
            println!("{}", "─".repeat(60));
            println!("{:<22} {:>12}", "TOTAL", fsutil::humano(total));
            Ok(())
        }
        Some("clean") => {
            let id = args.get(1).map(|s| s.as_str()).unwrap_or("");
            let cat = cats.iter().find(|c| c.id == id);
            let Some(cat) = cat else {
                eprintln!("Categoria desconhecida: {}", id);
                eprintln!("Use 'purgar lista' para ver as opções.");
                exit(2);
            };
            if cat.informativo {
                let res = cleaner::escanear(cat);
                println!("{}: {}", cat.nome, fsutil::humano(res.bytes));
                return Ok(());
            }
            let (tx, rx) = std::sync::mpsc::channel();
            let tx_clone = tx.clone();
            let cat_owned = clone_categoria(cat);
            let lixeira = cfg.usar_lixeira;
            let handle = std::thread::spawn(move || cleaner::limpar(&cat_owned, lixeira, tx_clone));
            // consome mensagens no thread principal
            let mut resumo = None;
            while let Ok(msg) = rx.recv() {
                match msg {
                    cleaner::Msg::Progresso(b, i) => {
                        eprint!("\r\x1b[KProcessado: {} ({} itens)", fsutil::humano(b), i);
                    }
                    cleaner::Msg::Linha(l) => eprintln!("\n{}", l),
                    cleaner::Msg::Fim(r) => {
                        resumo = Some(r);
                        break;
                    }
                }
            }
            drop(tx);
            let _ = handle.join();
            eprintln!();
            match resumo {
                Some(Ok(r)) => {
                    println!(
                        "{}: {} liberados em {} item(ns), {} erro(s).",
                        cat.nome,
                        fsutil::humano(r.bytes),
                        r.itens,
                        r.erros
                    );
                    config::registrar_historico(cat.id, r.bytes, r.erros);
                    if r.erros > 0 {
                        exit(1);
                    }
                }
                Some(Err(e)) => {
                    eprintln!("Falha: {}", e);
                    exit(1);
                }
                None => exit(1),
            }
            Ok(())
        }
        Some(outro) => {
            eprintln!("Comando desconhecido: {}", outro);
            eprintln!("Use 'purgar --help' para ajuda.");
            exit(2);
        }
    }
}

fn clone_categoria(c: &model::Categoria) -> model::Categoria {
    use model::Alvo::*;
    let alvos = c
        .alvos
        .iter()
        .map(|a| match a {
            Esvaziar { dir, excluir } => Esvaziar {
                dir: dir.clone(),
                excluir,
            },
            Remover(p) => Remover(p.clone()),
            Nomes { raiz, nomes, prof } => Nomes {
                raiz: raiz.clone(),
                nomes,
                prof: *prof,
            },
            Antigos { dirs, dias, sudo } => Antigos {
                dirs: dirs.clone(),
                dias: *dias,
                sudo: *sudo,
            },
            ComandoUser(cmd) => ComandoUser(cmd.clone()),
            ComandoRoot(cmd) => ComandoRoot(cmd.clone()),
        })
        .collect();
    model::Categoria {
        id: c.id,
        nome: c.nome,
        descricao: c.descricao,
        nivel: c.nivel,
        alvos,
        avisos_proc: c.avisos_proc,
        bloqueio_proc: c.bloqueio_proc,
        informativo: c.informativo,
    }
}
