//! Testes de integração da CLI: executam o binário real com entradas controladas.

use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_purgar")
}

fn rodar(args: &[&str]) -> std::process::Output {
    Command::new(bin())
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()
        .expect("falha ao executar o binário purgar")
}

#[test]
fn versao_e_help() {
    let saida = rodar(&["--version"]);
    assert!(saida.status.success());
    let txt = String::from_utf8_lossy(&saida.stdout);
    assert!(txt.starts_with("purgar "), "saída inesperada: {txt}");

    let saida = rodar(&["--help"]);
    assert!(saida.status.success());
    let txt = String::from_utf8_lossy(&saida.stdout);
    assert!(txt.contains("USO:"));
    assert!(txt.contains("clean"));
    assert!(txt.contains("cache_apps"));
}

#[test]
fn lista_em_texto_e_json() {
    let saida = rodar(&["lista"]);
    assert!(saida.status.success());
    let txt = String::from_utf8_lossy(&saida.stdout);
    assert!(txt.contains("cache_apps"));
    assert!(txt.contains("Cache de aplicativos"));

    let saida = rodar(&["lista", "--json"]);
    assert!(saida.status.success());
    let txt = String::from_utf8_lossy(&saida.stdout);
    assert!(txt.contains("\"id\": \"cache_apps\""));
    assert!(txt.contains("\"nivel\""));
}

#[test]
fn scan_json_tem_estrutura_esperada() {
    let saida = rodar(&["scan", "--json"]);
    assert!(saida.status.success());
    let txt = String::from_utf8_lossy(&saida.stdout);
    assert!(txt.contains("\"categorias\""));
    assert!(txt.contains("\"total_limpavel\""));
    assert!(txt.contains("\"requer_admin\""));
}

#[test]
fn dry_run_nao_limpa_e_termina_com_sucesso() {
    let saida = rodar(&["clean", "miniaturas", "--dry-run"]);
    assert!(saida.status.success());
    let txt = String::from_utf8_lossy(&saida.stdout);
    assert!(txt.contains("[dry-run]"));
    assert!(txt.contains("miniaturas"));
}

#[test]
fn clean_sem_confirmacao_em_ambiente_nao_interativo_falha() {
    let saida = rodar(&["clean", "miniaturas"]);
    assert_eq!(saida.status.code(), Some(2));
    let txt = String::from_utf8_lossy(&saida.stderr);
    assert!(txt.contains("não interativo"));
    assert!(txt.contains("--sim") || txt.contains("--dry-run"));
}

#[test]
fn categoria_desconhecida_falha_com_codigo_2() {
    let saida = rodar(&["clean", "categoria_que_nao_existe", "--sim"]);
    assert_eq!(saida.status.code(), Some(2));
    let txt = String::from_utf8_lossy(&saida.stderr);
    assert!(txt.contains("desconhecida"));

    let saida = rodar(&["comando-invalido"]);
    assert_eq!(saida.status.code(), Some(2));
}

#[test]
fn config_mostra_caminhos() {
    let saida = rodar(&["config"]);
    assert!(saida.status.success());
    let txt = String::from_utf8_lossy(&saida.stdout);
    assert!(txt.contains("config.json"));
    assert!(txt.contains("historico.jsonl"));
    assert!(txt.contains("Modo lixeira"));
}

#[test]
fn scan_filtrado_por_categoria() {
    let saida = rodar(&["scan", "--categoria", "miniaturas", "--detalhes"]);
    assert!(saida.status.success());
    let txt = String::from_utf8_lossy(&saida.stdout);
    assert!(txt.contains("miniaturas"));
    assert!(!txt.contains("cache_navegadores"));

    let saida = rodar(&["scan", "--categoria", "nao_existe"]);
    assert_eq!(saida.status.code(), Some(2));
}

#[test]
fn clean_ids_duplicados_sao_deduzidos() {
    // O segundo "miniaturas" deve ser ignorado (não duplicar processamento)
    let saida = rodar(&["clean", "miniaturas", "miniaturas", "--sim", "--dry-run"]);
    assert!(saida.status.success());
    let txt = String::from_utf8_lossy(&saida.stdout);
    // A categoria deve aparecer apenas uma vez na saída
    let count = txt.matches("miniaturas").count();
    assert_eq!(count, 1, "categoria duplicada não foi deduzida: {txt}");
}

#[test]
fn clean_sem_lixeira_mostra_aviso() {
    // Com --sim e sem --lixeira, deve emitir aviso no stderr
    let saida = rodar(&["clean", "miniaturas", "--sim", "--sem-lixeira", "--dry-run"]);
    assert!(saida.status.success());
    let err = String::from_utf8_lossy(&saida.stderr);
    assert!(err.contains("AVISO"), "falta aviso de remoção permanente: {err}");
    assert!(err.contains("PERMANENTEMENTE"), "falta destaque permanente: {err}");
}
