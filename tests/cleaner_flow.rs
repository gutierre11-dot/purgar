//! Testes de integração do fluxo de limpeza (análise, remoção, lixeira,
//! cancelamento e categorias informativas) em diretórios temporários.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, channel};
use std::time::{SystemTime, UNIX_EPOCH};

use purgar::cleaner::{self, Msg, Opcoes};
use purgar::model::{Alvo, Categoria, Nivel, Resumo};

const SEM_EXCLUSAO: &[&str] = &[];
const EXCLUIR_MANTER: &[&str] = &["manter"];

fn temp_dir(nome: &str) -> PathBuf {
    let carimbo = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir =
        std::env::temp_dir().join(format!("purgar-it-{nome}-{}-{carimbo}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn categoria(nome: &'static str, alvos: Vec<Alvo>, informativo: bool) -> Categoria {
    Categoria {
        id: "teste",
        nome,
        descricao: "categoria de teste".into(),
        nivel: Nivel::Basico,
        alvos,
        avisos_proc: &[],
        bloqueio_proc: &[],
        informativo,
    }
}

fn executar(cat: Categoria, opcoes: Opcoes) -> Resumo {
    let (tx, rx): (_, Receiver<Msg>) = channel();
    let handle = std::thread::spawn(move || cleaner::limpar(&cat, &opcoes, tx));
    let resumo = coletar_fim(&rx);
    handle.join().unwrap();
    resumo
}

fn coletar_fim(rx: &Receiver<Msg>) -> Resumo {
    while let Ok(msg) = rx.recv() {
        if let Msg::Fim(resultado) = msg {
            return resultado.expect("limpeza deveria concluir com sucesso");
        }
    }
    panic!("canal fechou sem Msg::Fim");
}

fn criar_arquivo(caminho: &Path, bytes: usize) {
    fs::write(caminho, vec![b'x'; bytes]).unwrap();
}

#[test]
fn escanear_e_limpar_esvaziando_diretorio() {
    let base = temp_dir("flow");
    criar_arquivo(&base.join("a.txt"), 100);
    fs::create_dir_all(base.join("sub")).unwrap();
    criar_arquivo(&base.join("sub/b.txt"), 50);

    let cat = categoria(
        "Teste",
        vec![Alvo::Esvaziar {
            dir: base.clone(),
            excluir: SEM_EXCLUSAO,
        }],
        false,
    );

    let scan = cleaner::escanear(&cat);
    assert_eq!(scan.bytes, 150);
    assert_eq!(scan.itens, 2);

    let resumo = executar(cat, Opcoes::novo(false));
    assert_eq!(resumo.bytes, 150);
    assert_eq!(resumo.itens, 2);
    assert_eq!(resumo.erros, 0);
    assert!(!resumo.lixeira);
    assert!(base.exists(), "o diretório raiz deve ser mantido");
    assert_eq!(fs::read_dir(&base).unwrap().count(), 0);

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn esvaziar_respeita_lista_de_exclusao() {
    let base = temp_dir("excluir");
    criar_arquivo(&base.join("manter"), 10);
    criar_arquivo(&base.join("apagar"), 20);

    let cat = categoria(
        "Teste",
        vec![Alvo::Esvaziar {
            dir: base.clone(),
            excluir: EXCLUIR_MANTER,
        }],
        false,
    );
    assert_eq!(cleaner::escanear(&cat).itens, 1);

    let resumo = executar(cat, Opcoes::novo(false));
    assert_eq!(resumo.bytes, 20);
    assert!(base.join("manter").exists());
    assert!(!base.join("apagar").exists());
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn modo_lixeira_move_sem_liberar_espaco() {
    let base = temp_dir("lixeira");
    let trash = temp_dir("trash");
    criar_arquivo(&base.join("dados.bin"), 64);

    let opcoes = Opcoes {
        usar_lixeira: true,
        cancelar: Arc::new(AtomicBool::new(false)),
        total_estimado: 0,
        lixeira_raiz: Some(trash.clone()),
    };
    let cat = categoria(
        "Teste",
        vec![Alvo::Esvaziar {
            dir: base.clone(),
            excluir: SEM_EXCLUSAO,
        }],
        false,
    );

    let resumo = executar(cat, opcoes);
    assert!(resumo.lixeira);
    assert_eq!(resumo.bytes, 0, "modo lixeira não libera espaço");
    assert_eq!(resumo.movidos, 64);
    assert!(!base.join("dados.bin").exists());
    assert_eq!(fs::read_dir(trash.join("files")).unwrap().count(), 1);

    let _ = fs::remove_dir_all(&base);
    let _ = fs::remove_dir_all(&trash);
}

#[test]
fn cancelamento_preserva_arquivos() {
    let base = temp_dir("cancelar");
    criar_arquivo(&base.join("a"), 10);
    criar_arquivo(&base.join("b"), 10);

    let cat = categoria(
        "Teste",
        vec![Alvo::Esvaziar {
            dir: base.clone(),
            excluir: SEM_EXCLUSAO,
        }],
        false,
    );
    let cancelar = Arc::new(AtomicBool::new(true));
    let opcoes = Opcoes {
        usar_lixeira: false,
        cancelar,
        total_estimado: 20,
        lixeira_raiz: None,
    };
    let resumo = executar(cat, opcoes);
    assert!(resumo.cancelado);
    assert_eq!(resumo.itens, 0);
    assert!(base.join("a").exists());
    assert!(base.join("b").exists());
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn categoria_informativa_apenas_mede() {
    let base = temp_dir("medir");
    let arquivo = base.join("documento.pdf");
    criar_arquivo(&arquivo, 128);

    let cat = categoria("Downloads", vec![Alvo::Medir(arquivo.clone())], true);
    let scan = cleaner::escanear(&cat);
    assert_eq!(scan.bytes, 128);

    let resumo = executar(cat, Opcoes::novo(false));
    assert_eq!(resumo.itens, 0);
    assert!(arquivo.exists(), "alvo Medir nunca remove");
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn limpar_todas_emite_historico_por_categoria() {
    let base_a = temp_dir("lote-a");
    let base_b = temp_dir("lote-b");
    criar_arquivo(&base_a.join("a"), 30);
    criar_arquivo(&base_b.join("b"), 70);

    let cats = vec![
        categoria(
            "A",
            vec![Alvo::Esvaziar {
                dir: base_a.clone(),
                excluir: SEM_EXCLUSAO,
            }],
            false,
        ),
        categoria(
            "B",
            vec![Alvo::Esvaziar {
                dir: base_b.clone(),
                excluir: SEM_EXCLUSAO,
            }],
            false,
        ),
    ];

    let (tx, rx) = channel();
    let opcoes = Opcoes::novo(false);
    let handle = std::thread::spawn(move || cleaner::limpar_todas(&cats, &opcoes, tx));

    let mut historicos = Vec::new();
    let mut total = None;
    while let Ok(msg) = rx.recv() {
        match msg {
            Msg::Historico {
                categoria: id,
                bytes,
                movidos,
                erros,
            } => historicos.push((id, bytes, movidos, erros)),
            Msg::Fim(resultado) => {
                total = Some(resultado.expect("limpeza em lote falhou"));
                break;
            }
            _ => {}
        }
    }
    handle.join().unwrap();

    let total = total.expect("sem Msg::Fim");
    assert_eq!(total.bytes, 100);
    assert_eq!(historicos.len(), 2);
    assert_eq!(historicos.iter().map(|h| h.1).sum::<u64>(), 100);
    assert!(historicos.iter().all(|h| h.3 == 0));
    let _ = fs::remove_dir_all(&base_a);
    let _ = fs::remove_dir_all(&base_b);
}

#[test]
fn removedor_de_arquivo_unico() {
    let base = temp_dir("remover");
    let arquivo = base.join("arquivo.tmp");
    criar_arquivo(&arquivo, 32);
    let cat = categoria("Arquivo", vec![Alvo::Remover(arquivo.clone())], false);
    let resumo = executar(cat, Opcoes::novo(false));
    assert_eq!(resumo.bytes, 32);
    assert!(!arquivo.exists());
    let _ = fs::remove_dir_all(&base);
}
