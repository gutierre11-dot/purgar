use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;
use std::sync::mpsc;

use purgar::categories;
use purgar::cleaner::{self, Msg, Opcoes};
use purgar::config::{self, Config};
use purgar::fsutil;
use purgar::model::{Categoria, Nivel, Resumo};

const VERSAO: &str = env!("CARGO_PKG_VERSION");

fn main() -> ExitCode {
    match executar() {
        Ok(codigo) => ExitCode::from(codigo),
        Err(erro) => {
            eprintln!("erro: {erro:#}");
            ExitCode::from(1)
        }
    }
}

enum Comando {
    Tui,
    Scan {
        json: bool,
        detalhes: bool,
        categoria: Option<String>,
    },
    Clean {
        ids: Vec<String>,
        todos_basicos: bool,
        sim: bool,
        dry_run: bool,
        json: bool,
        lixeira: Option<bool>,
    },
    Lista {
        json: bool,
    },
    Configuracao {
        json: bool,
    },
    Ajuda,
    Versao,
}

fn executar() -> anyhow::Result<u8> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cfg = config::carregar();
    let cats = categories::categorias(&cfg);

    let comando = match analisar_args(&args) {
        Ok(c) => c,
        Err(erro) => {
            eprintln!("{erro}");
            eprintln!("Use 'purgar --help' para ajuda.");
            return Ok(2);
        }
    };

    match comando {
        Comando::Ajuda => {
            imprimir_ajuda(&cats);
            Ok(0)
        }
        Comando::Versao => {
            println!("purgar {VERSAO}");
            Ok(0)
        }
        Comando::Tui => {
            if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
                eprintln!(
                    "purgar: a interface TUI exige um terminal interativo.\n\
                     Use 'purgar scan' para analisar ou 'purgar --help' para ver os comandos."
                );
                return Ok(2);
            }
            purgar::tui::run()?;
            Ok(0)
        }
        Comando::Lista { json } => {
            imprimir_lista(&cats, json);
            Ok(0)
        }
        Comando::Configuracao { json } => {
            imprimir_config(&cfg, json);
            Ok(0)
        }
        Comando::Scan {
            json,
            detalhes,
            categoria,
        } => scan(&cats, json, detalhes, categoria),
        Comando::Clean {
            ids,
            todos_basicos,
            sim,
            dry_run,
            json,
            lixeira,
        } => clean(&cfg, &cats, ids, todos_basicos, sim, dry_run, json, lixeira),
    }
}

fn proximo_valor(args: &[String], i: &mut usize, flag: &str) -> Result<String, String> {
    if let Some(v) = args.get(*i + 1) {
        *i += 1;
        Ok(v.clone())
    } else {
        Err(format!("a opção {flag} exige um valor"))
    }
}

fn analisar_args(args: &[String]) -> Result<Comando, String> {
    let Some(primeiro) = args.first() else {
        return Ok(Comando::Tui);
    };
    let resto = &args[1..];
    match primeiro.as_str() {
        "-h" | "--help" | "help" => Ok(Comando::Ajuda),
        "-V" | "--version" | "version" => Ok(Comando::Versao),
        "scan" => {
            let (mut json, mut detalhes, mut categoria) = (false, false, None);
            let mut i = 0;
            while i < resto.len() {
                let arg = resto[i].as_str();
                match arg {
                    "--json" => json = true,
                    "--detalhes" | "-d" => detalhes = true,
                    "--categoria" | "-c" => {
                        categoria = Some(proximo_valor(resto, &mut i, arg)?);
                    }
                    outro if outro.starts_with("--categoria=") => {
                        categoria = Some(outro["--categoria=".len()..].to_string());
                    }
                    "-h" | "--help" => return Ok(Comando::Ajuda),
                    outro => return Err(format!("opção desconhecida para scan: {outro}")),
                }
                i += 1;
            }
            Ok(Comando::Scan {
                json,
                detalhes,
                categoria,
            })
        }
        "clean" => {
            let mut ids = Vec::new();
            let mut todos_basicos = false;
            let mut sim = false;
            let mut dry_run = false;
            let mut json = false;
            let mut lixeira = None;
            let mut i = 0;
            while i < resto.len() {
                let arg = resto[i].as_str();
                match arg {
                    "--todos-basicos" | "--todos" => todos_basicos = true,
                    "--sim" | "--yes" | "-y" => sim = true,
                    "--dry-run" | "-n" => dry_run = true,
                    "--json" => json = true,
                    "--lixeira" => lixeira = Some(true),
                    "--sem-lixeira" => lixeira = Some(false),
                    "-h" | "--help" => return Ok(Comando::Ajuda),
                    outro if outro.starts_with('-') => {
                        return Err(format!("opção desconhecida para clean: {outro}"));
                    }
                    id => ids.push(id.to_string()),
                }
                i += 1;
            }
            Ok(Comando::Clean {
                ids,
                todos_basicos,
                sim,
                dry_run,
                json,
                lixeira,
            })
        }
        "lista" | "list" => {
            let mut json = false;
            for arg in resto {
                match arg.as_str() {
                    "--json" => json = true,
                    "-h" | "--help" => return Ok(Comando::Ajuda),
                    outro => return Err(format!("opção desconhecida para lista: {outro}")),
                }
            }
            Ok(Comando::Lista { json })
        }
        "config" | "configuracao" => {
            let mut json = false;
            for arg in resto {
                match arg.as_str() {
                    "--json" => json = true,
                    "-h" | "--help" => return Ok(Comando::Ajuda),
                    outro => return Err(format!("opção desconhecida para config: {outro}")),
                }
            }
            Ok(Comando::Configuracao { json })
        }
        outro => Err(format!("comando desconhecido: {outro}")),
    }
}

fn largura(texto: &str) -> usize {
    use unicode_width::UnicodeWidthStr;
    texto.width()
}

fn truncar(texto: &str, limite: usize) -> String {
    if largura(texto) <= limite {
        return texto.to_string();
    }
    let mut out = String::new();
    let mut usado = 0;
    for c in texto.chars() {
        let w = unicode_width::UnicodeWidthChar::width(c).unwrap_or(1);
        if usado + w > limite.saturating_sub(1) {
            break;
        }
        out.push(c);
        usado += w;
    }
    out.push('…');
    out
}

fn imprimir_ajuda(cats: &[Categoria]) {
    println!(
        "purgar {VERSAO} — limpeza de cache e arquivos temporários (Linux)\n\n\
         USO:\n\
         \x20   purgar                                      Abre a interface TUI\n\
         \x20   purgar scan [--json] [--detalhes] [--categoria ID]  Analisa as categorias\n\
         \x20   purgar clean <ID>... [opções]              Limpa uma ou mais categorias\n\
         \x20   purgar clean --todos-basicos [opções]      Limpa todas as categorias básicas\n\
         \x20   purgar lista [--json]                      Lista as categorias disponíveis\n\
         \x20   purgar config [--json]                     Mostra configuração e histórico\n\
         \x20   purgar --version | -V                      Mostra a versão\n\
         \x20   purgar --help | -h                         Mostra esta ajuda\n\n\
         OPÇÕES DE clean:\n\
         \x20   --sim, --yes, -y     Não pede confirmação (obrigatório sem TTY)\n\
         \x20   --dry-run, -n        Apenas analisa e mostra o que seria removido\n\
         \x20   --json               Saída em JSON\n\
         \x20   --lixeira            Força o modo lixeira nesta execução\n\
         \x20   --sem-lixeira        Desativa o modo lixeira nesta execução\n\
         \x20   --todos-basicos      Limpa todas as categorias de nível básico\n\n\
         CÓDIGOS DE SAÍDA:\n\
         \x20   0 sucesso   1 falhas   2 uso inválido   130 não confirmado\n\n\
         ARQUIVOS:\n\
         \x20   ~/.config/purgar/config.json\n\
         \x20   ~/.local/share/purgar/historico.jsonl\n\n\
         CATEGORIAS:"
    );
    for c in cats {
        let admin = if c.requer_admin() { " [admin]" } else { "" };
        let info = if c.informativo { " [análise]" } else { "" };
        println!(
            "  {:<8} {:<18} {}{}{}",
            c.nivel.rotulo(),
            c.id,
            c.nome,
            admin,
            info
        );
    }
    println!(
        "\nSegurança: caminhos personalizados são validados; em modo lixeira os \
         itens vão para a lixeira do freedesktop; ações administrativas usam pkexec."
    );
}

fn imprimir_lista(cats: &[Categoria], json: bool) {
    if json {
        let itens: Vec<serde_json::Value> = cats
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "nome": c.nome,
                    "nivel": c.nivel.rotulo(),
                    "admin": c.requer_admin(),
                    "informativo": c.informativo,
                })
            })
            .collect();
        if let Ok(txt) = serde_json::to_string_pretty(&itens) {
            println!("{txt}");
        }
        return;
    }

    let w_id = cats.iter().map(|c| largura(c.id)).max().unwrap_or(4).max(4);
    let w_nome = cats
        .iter()
        .map(|c| largura(c.nome))
        .max()
        .unwrap_or(10)
        .max(10);
    println!(
        "{:<8}  {:<w_id$}  {:<w_nome$}  OBS",
        "NÍVEL", "ID", "CATEGORIA"
    );
    for c in cats {
        let mut obs = Vec::new();
        if c.requer_admin() {
            obs.push("admin");
        }
        if c.informativo {
            obs.push("somente análise");
        }
        println!(
            "{:<8}  {:<w_id$}  {:<w_nome$}  {}",
            c.nivel.rotulo(),
            c.id,
            c.nome,
            obs.join(", ")
        );
    }
}

fn imprimir_config(cfg: &Config, json: bool) {
    let registros = config::historico().len();
    if json {
        let valor = serde_json::json!({
            "config": config::caminho_config(),
            "historico": config::caminho_historico(),
            "usar_lixeira": cfg.usar_lixeira,
            "dias_tmp": cfg.dias_tmp,
            "dias_logs": cfg.dias_logs,
            "historico_max": cfg.historico_max,
            "apagar_dados_flatpak": cfg.apagar_dados_flatpak,
            "caminhos": cfg.caminhos,
            "registros_historico": registros,
        });
        if let Ok(txt) = serde_json::to_string_pretty(&valor) {
            println!("{txt}");
        }
        return;
    }
    println!(
        "Arquivo de configuração : {}",
        config::caminho_config().display()
    );
    println!(
        "Arquivo de histórico    : {}",
        config::caminho_historico().display()
    );
    println!("Modo lixeira            : {}", sim_nao(cfg.usar_lixeira));
    println!("Dias para /tmp          : {}", cfg.dias_tmp);
    println!("Dias para logs          : {}", cfg.dias_logs);
    println!("Máximo de registros     : {}", cfg.historico_max);
    println!(
        "Apagar dados do Flatpak : {}",
        sim_nao(cfg.apagar_dados_flatpak)
    );
    println!("Caminhos personalizados : {}", cfg.caminhos.len());
    for caminho in &cfg.caminhos {
        println!("  - {caminho}");
    }
    println!("Registros no histórico  : {registros}");
}

fn sim_nao(valor: bool) -> &'static str {
    if valor { "sim" } else { "não" }
}

fn scan(
    cats: &[Categoria],
    json: bool,
    detalhes: bool,
    filtro: Option<String>,
) -> anyhow::Result<u8> {
    use rayon::prelude::*;
    let selecionadas: Vec<&Categoria> = match &filtro {
        Some(id) => {
            let achadas: Vec<&Categoria> = cats.iter().filter(|c| c.id == id).collect();
            if achadas.is_empty() {
                eprintln!("categoria desconhecida: {id}");
                return Ok(2);
            }
            achadas
        }
        None => cats.iter().collect(),
    };

    let resultados: Vec<_> = selecionadas
        .par_iter()
        .map(|c| cleaner::escanear(c))
        .collect();

    if json {
        let itens: Vec<serde_json::Value> = selecionadas
            .iter()
            .zip(resultados.iter())
            .map(|(c, r)| {
                serde_json::json!({
                    "id": c.id,
                    "nome": c.nome,
                    "nivel": c.nivel.rotulo(),
                    "bytes": r.bytes,
                    "itens": r.itens,
                    "requer_admin": c.requer_admin(),
                    "informativo": c.informativo,
                    "detalhes": r.detalhes,
                    "avisos": r.avisos,
                })
            })
            .collect();
        let saida = serde_json::json!({
            "categorias": itens,
            "total_limpavel": total_limpavel(&selecionadas, &resultados),
        });
        if let Ok(txt) = serde_json::to_string_pretty(&saida) {
            println!("{txt}");
        }
        return Ok(0);
    }

    let w_nome = selecionadas
        .iter()
        .map(|c| largura(c.nome))
        .max()
        .unwrap_or(10)
        .clamp(10, 34);
    let w_id = selecionadas
        .iter()
        .map(|c| largura(c.id))
        .max()
        .unwrap_or(4)
        .max(4);
    println!(
        "{:<w_nome$}  {:<w_id$}  {:>10}  {:>7}  OBS",
        "CATEGORIA", "ID", "TAMANHO", "ITENS"
    );
    println!("{}", "─".repeat(78));
    for (c, r) in selecionadas.iter().zip(resultados.iter()) {
        let mut obs = Vec::new();
        if c.requer_admin() {
            obs.push("admin");
        }
        if c.informativo {
            obs.push("análise");
        }
        println!(
            "{:<w_nome$}  {:<w_id$}  {:>10}  {:>7}  {}",
            truncar(c.nome, w_nome),
            c.id,
            fsutil::humano(r.bytes),
            r.itens,
            obs.join(", ")
        );
        if detalhes {
            for d in &r.detalhes {
                println!("    • {d}");
            }
            for a in &r.avisos {
                println!("    ! {a}");
            }
        }
    }
    println!("{}", "─".repeat(78));
    let limpavel = total_limpavel(&selecionadas, &resultados);
    let analise: u64 = selecionadas
        .iter()
        .zip(resultados.iter())
        .filter(|(c, _)| c.informativo)
        .map(|(_, r)| r.bytes)
        .sum();
    println!(
        "{:<w_nome$}  {:<w_id$}  {:>10}",
        "TOTAL LIMPÁVEL",
        "",
        fsutil::humano(limpavel)
    );
    if analise > 0 {
        println!(
            "{:<w_nome$}  {:<w_id$}  {:>10}",
            "Somente análise (não limpo)",
            "",
            fsutil::humano(analise)
        );
    }
    Ok(0)
}

fn total_limpavel(cats: &[&Categoria], resultados: &[purgar::model::ScanResult]) -> u64 {
    cats.iter()
        .zip(resultados.iter())
        .filter(|(c, _)| !c.informativo)
        .map(|(_, r)| r.bytes)
        .sum()
}

#[allow(clippy::too_many_arguments)]
fn clean(
    cfg: &Config,
    cats: &[Categoria],
    ids: Vec<String>,
    todos_basicos: bool,
    sim: bool,
    dry_run: bool,
    json: bool,
    lixeira: Option<bool>,
) -> anyhow::Result<u8> {
    let ids: Vec<String> = if todos_basicos {
        cats.iter()
            .filter(|c| c.nivel == Nivel::Basico && !c.informativo)
            .map(|c| c.id.to_string())
            .collect()
    } else {
        ids
    };

    if ids.is_empty() {
        eprintln!("nenhuma categoria informada.");
        eprintln!("Use 'purgar lista' para ver as opções ou --todos-basicos.");
        return Ok(2);
    }

    let mut vistos = std::collections::HashSet::new();
    let mut selecionadas: Vec<&Categoria> = Vec::new();
    for id in &ids {
        if !vistos.insert(id.clone()) {
            continue;
        }
        match cats.iter().find(|c| c.id == id) {
            Some(c) => selecionadas.push(c),
            None => {
                eprintln!("categoria desconhecida: {id}");
                eprintln!("Use 'purgar lista' para ver as opções.");
                return Ok(2);
            }
        }
    }

    let usar_lixeira = lixeira.unwrap_or(cfg.usar_lixeira);
    let estimativas: Vec<purgar::model::ScanResult> =
        selecionadas.iter().map(|c| cleaner::escanear(c)).collect();

    // Aviso explícito quando o usuário não usa lixeira (aplica a dry-run também).
    if !usar_lixeira && !json {
        eprintln!(
            "AVISO: modo lixeira desativado — os itens serão removidos PERMANENTEMENTE."
        );
        eprintln!("       Use --lixeira para mover para a lixeira (reversível).");
    }

    if dry_run {
        let saida: Vec<serde_json::Value> = selecionadas
            .iter()
            .zip(estimativas.iter())
            .map(|(c, r)| {
                let caminhos = cleaner::paths_para_remocao(c);
                if !json {
                    println!("[dry-run] {} ({})", c.nome, c.id);
                    for alvo in cleaner::detalhar_alvos(c) {
                        println!("    - {alvo}");
                    }
                    println!(
                        "    {} item(ns) / {}",
                        caminhos.len(),
                        fsutil::humano(r.bytes)
                    );
                }
                serde_json::json!({
                    "id": c.id,
                    "nome": c.nome,
                    "bytes": r.bytes,
                    "itens": caminhos.len(),
                    "caminhos": caminhos,
                })
            })
            .collect();
        if json {
            let total: u64 = estimativas.iter().map(|r| r.bytes).sum();
            let valor = serde_json::json!({
                "dry_run": true,
                "lixeira": usar_lixeira,
                "categorias": saida,
                "total_estimado": total,
            });
            if let Ok(txt) = serde_json::to_string_pretty(&valor) {
                println!("{txt}");
            }
        }
        return Ok(0);
    }

    // Bloqueio de navegadores abertos (mesmo comportamento da TUI).
    for c in &selecionadas {
        let procs = cleaner::processo_rodando(c.bloqueio_proc);
        if !procs.is_empty() {
            eprintln!(
                "operação bloqueada: feche {} antes de limpar '{}'.",
                procs.join(", "),
                c.nome
            );
            return Ok(2);
        }
    }

    if !sim {
        if !io::stdin().is_terminal() {
            eprintln!(
                "ambiente não interativo: use --sim para confirmar a limpeza \
                 ou --dry-run para apenas analisar."
            );
            return Ok(2);
        }
        println!("Categorias a limpar:");
        for (c, r) in selecionadas.iter().zip(estimativas.iter()) {
            println!(
                "  • {:<30} {}",
                truncar(c.nome, 30),
                fsutil::humano(r.bytes)
            );
        }
        println!(
            "Modo: {}",
            if usar_lixeira {
                "mover para a lixeira"
            } else {
                "remover permanentemente"
            }
        );
        print!("Confirmar limpeza? [s/N] ");
        io::stdout().flush()?;
        let mut resposta = String::new();
        io::stdin().read_line(&mut resposta)?;
        let resposta = resposta.trim().to_lowercase();
        if !matches!(resposta.as_str(), "s" | "sim" | "y" | "yes") {
            eprintln!("cancelado.");
            return Ok(130);
        }
    }

    let mut total = Resumo {
        lixeira: usar_lixeira,
        ..Resumo::default()
    };
    let mut saida_json: Vec<serde_json::Value> = Vec::new();
    let mut houve_erro = false;
    let progresso_visivel = !json && io::stderr().is_terminal();

    for (indice, categoria) in selecionadas.iter().enumerate() {
        if categoria.informativo {
            if !json {
                println!(
                    "{}: categoria somente informativa ({}), nada a limpar.",
                    categoria.nome,
                    fsutil::humano(estimativas[indice].bytes)
                );
            }
            continue;
        }
        let _ = io::stderr().flush();
        let (tx, rx) = mpsc::channel();
        let cat_clone = (*categoria).clone();
        let opcoes = Opcoes::com_total(usar_lixeira, estimativas[indice].bytes);
        let handle = std::thread::spawn(move || cleaner::limpar(&cat_clone, &opcoes, tx));

        let mut resumo = None;
        while let Ok(msg) = rx.recv() {
            match msg {
                Msg::Progresso(bytes, itens) => {
                    if progresso_visivel {
                        eprint!(
                            "\r\x1b[K{} — processado: {} ({} itens)",
                            categoria.nome,
                            fsutil::humano(bytes),
                            itens
                        );
                    }
                }
                Msg::Linha(linha) => {
                    if !json && progresso_visivel {
                        eprintln!("\r\x1b[K{linha}");
                    }
                }
                Msg::Historico { .. } => {}
                Msg::Fim(resultado) => {
                    resumo = Some(resultado);
                    break;
                }
            }
        }
        let _ = handle.join();
        if progresso_visivel {
            eprintln!();
        }

        match resumo {
            Some(Ok(r)) => {
                total.bytes += r.bytes;
                total.movidos += r.movidos;
                total.itens += r.itens;
                total.erros += r.erros;
                if r.cancelado {
                    total.cancelado = true;
                }
                config::registrar_historico(
                    categoria.id,
                    r.bytes,
                    r.movidos,
                    r.erros,
                    cfg.historico_max,
                );
                if !json {
                    println!(
                        "{}: {} liberados, {} movidos para a lixeira, {} item(ns), {} erro(s).",
                        categoria.nome,
                        fsutil::humano(r.bytes),
                        fsutil::humano(r.movidos),
                        r.itens,
                        r.erros
                    );
                }
                saida_json.push(serde_json::json!({
                    "id": categoria.id,
                    "nome": categoria.nome,
                    "bytes": r.bytes,
                    "movidos": r.movidos,
                    "itens": r.itens,
                    "erros": r.erros,
                    "cancelado": r.cancelado,
                }));
                if r.erros > 0 {
                    houve_erro = true;
                }
            }
            Some(Err(erro)) => {
                houve_erro = true;
                total.erros += 1;
                if !json {
                    eprintln!("{}: falha: {erro}", categoria.nome);
                }
                saida_json.push(serde_json::json!({
                    "id": categoria.id,
                    "nome": categoria.nome,
                    "erro": erro,
                }));
            }
            None => {
                houve_erro = true;
                total.erros += 1;
                if !json {
                    eprintln!(
                        "{}: a operação terminou inesperadamente (thread encerrada sem resultado).",
                        categoria.nome
                    );
                }
                saida_json.push(serde_json::json!({
                    "id": categoria.id,
                    "nome": categoria.nome,
                    "erro": "thread encerrada sem resultado",
                }));
            }
        }
    }

    if json {
        let valor = serde_json::json!({
            "lixeira": usar_lixeira,
            "categorias": saida_json,
            "total": {
                "bytes": total.bytes,
                "movidos": total.movidos,
                "itens": total.itens,
                "erros": total.erros,
            },
        });
        if let Ok(txt) = serde_json::to_string_pretty(&valor) {
            println!("{txt}");
        }
    } else {
        println!(
            "Total: {} liberados, {} movidos para a lixeira, {} erro(s).",
            fsutil::humano(total.bytes),
            fsutil::humano(total.movidos),
            total.erros
        );
    }

    if houve_erro || total.erros > 0 {
        Ok(1)
    } else {
        Ok(0)
    }
}
