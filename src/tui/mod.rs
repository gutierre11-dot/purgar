pub mod ui;

use std::collections::HashMap;
use std::io;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::categories;
use crate::cleaner::{self, Msg, Opcoes};
use crate::config::{self, Config, Registro};
use crate::model::{Categoria, Nivel, Resumo, ScanResult};
use crate::seguranca;
use crate::tools::{self, AutostartItem, Pacote};

pub const VERSAO: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tela {
    Home,
    Detalhe,
    Progresso,
    Resultado,
    Autostart,
    Desinstalar,
    Estatisticas,
    Config,
    Caminhos,
    Ajuda,
    Confirmar,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Ferramenta {
    LimparTudo,
    Autostart,
    Desinstalar,
    Estatisticas,
    Configuracoes,
    Ajuda,
}

struct ItemMenu {
    rotulo: String,
    secao: &'static str,
    cat_id: Option<String>,
    ferramenta: Option<Ferramenta>,
}

enum AcaoConfirm {
    Limpar(String),
    LimparTudo,
    ExcluirAutostart(PathBuf),
    Desinstalar(usize),
    RemoverCaminho(usize),
}

pub struct App {
    tela: Tela,
    sair: bool,
    config: Config,
    categorias: Vec<Categoria>,
    itens: Vec<ItemMenu>,
    sel: usize,

    // tela inicial
    tamanhos: HashMap<String, u64>,
    escaneando: bool,
    scan_rx: Option<Receiver<(String, ScanResult)>>,
    disco: Option<(u64, u64)>,

    // detalhe
    cat_atual: Option<String>,
    scan_cat_rx: Option<Receiver<ScanResult>>,
    scan_pendente: bool,
    scan_result: Option<ScanResult>,
    acao_idx: usize,
    processos: Vec<String>,
    aviso_tela: String,

    // trabalho em segundo plano
    job_rx: Option<Receiver<Msg>>,
    job_titulo: String,
    job_cat_id: Option<String>,
    job_total: u64,
    cancelar: Option<Arc<AtomicBool>>,
    pediu_cancelamento: bool,
    prog_bytes: u64,
    prog_itens: u64,
    logs: Vec<String>,
    /// Evita registrar o histórico duas vezes: uma vez via `Msg::Historico`
    /// e outra via `Msg::Fim` (limpeza única de categoria).
    historico_feito: bool,
    resultado: Option<Result<Resumo, String>>,
    resultado_titulo: String,

    // confirmação
    confirmar: Option<AcaoConfirm>,
    confirmar_idx: usize,
    tela_anterior: Tela,

    // autostart
    autostarts: Vec<AutostartItem>,
    autostart_sel: usize,

    // desinstalador (carregamento assíncrono)
    pacotes: Vec<Pacote>,
    pacote_sel: usize,
    filtro: String,
    carregando_pacotes: bool,
    pacotes_rx: Option<Receiver<Vec<Pacote>>>,

    // estatísticas
    registros: Vec<Registro>,

    // configurações
    cfg_sel: usize,
    digitando: bool,
    input_buf: String,

    frame: u64,
}

impl App {
    pub fn new() -> Self {
        let config = config::carregar();
        let categorias = categories::categorias(&config);
        let itens = Self::montar_itens(&categorias);
        let mut app = Self {
            tela: Tela::Home,
            sair: false,
            config,
            categorias,
            itens,
            sel: 0,
            tamanhos: HashMap::new(),
            escaneando: true,
            scan_rx: None,
            disco: cleaner::espaco_disco(),
            cat_atual: None,
            scan_cat_rx: None,
            scan_pendente: false,
            scan_result: None,
            acao_idx: 0,
            processos: Vec::new(),
            aviso_tela: String::new(),
            job_rx: None,
            job_titulo: String::new(),
            job_cat_id: None,
            job_total: 0,
            cancelar: None,
            pediu_cancelamento: false,
            prog_bytes: 0,
            prog_itens: 0,
            logs: Vec::new(),
            historico_feito: false,
            resultado: None,
            resultado_titulo: String::new(),
            confirmar: None,
            confirmar_idx: 0,
            tela_anterior: Tela::Home,
            autostarts: Vec::new(),
            autostart_sel: 0,
            pacotes: Vec::new(),
            pacote_sel: 0,
            filtro: String::new(),
            carregando_pacotes: false,
            pacotes_rx: None,
            registros: Vec::new(),
            cfg_sel: 0,
            digitando: false,
            input_buf: String::new(),
            frame: 0,
        };
        app.iniciar_scan_tudo();
        app
    }

    fn montar_itens(categorias: &[Categoria]) -> Vec<ItemMenu> {
        let mut itens = Vec::new();
        for nivel in [Nivel::Basico, Nivel::Avancado] {
            let secao = nivel.rotulo();
            for categoria in categorias.iter().filter(|c| c.nivel == nivel) {
                itens.push(ItemMenu {
                    rotulo: categoria.nome.to_string(),
                    secao,
                    cat_id: Some(categoria.id.to_string()),
                    ferramenta: None,
                });
            }
        }
        for (rotulo, ferramenta) in [
            ("Limpar tudo (categorias básicas)", Ferramenta::LimparTudo),
            ("Gerenciador de inicialização", Ferramenta::Autostart),
            ("Desinstalador de programas", Ferramenta::Desinstalar),
            ("Estatísticas de limpeza", Ferramenta::Estatisticas),
            ("Configurações", Ferramenta::Configuracoes),
            ("Ajuda e atalhos", Ferramenta::Ajuda),
        ] {
            itens.push(ItemMenu {
                rotulo: rotulo.to_string(),
                secao: "Ferramentas",
                cat_id: None,
                ferramenta: Some(ferramenta),
            });
        }
        itens
    }

    pub fn categoria(&self, id: &str) -> Option<&Categoria> {
        self.categorias.iter().find(|c| c.id == id)
    }

    fn iniciar_scan_tudo(&mut self) {
        let cats = self.categorias.clone();
        let (tx, rx) = channel();
        self.scan_rx = Some(rx);
        self.escaneando = true;
        self.tamanhos.clear();
        std::thread::spawn(move || {
            for c in &cats {
                let res = cleaner::escanear(c);
                if tx.send((c.id.to_string(), res)).is_err() {
                    break;
                }
            }
        });
    }

    fn entrar_detalhe(&mut self, id: &str) {
        let Some(cat) = self.categoria(id) else {
            return;
        };
        let cat = cat.clone();
        self.tela = Tela::Detalhe;
        self.cat_atual = Some(id.to_string());
        self.acao_idx = 0;
        self.scan_result = None;
        self.scan_pendente = true;
        self.aviso_tela.clear();
        self.processos = cleaner::processo_rodando(cat.bloqueio_proc);
        if self.processos.is_empty() {
            self.processos = cleaner::processo_rodando(cat.avisos_proc);
        }
        let (tx, rx) = channel();
        self.scan_cat_rx = Some(rx);
        std::thread::spawn(move || {
            let res = cleaner::escanear(&cat);
            let _ = tx.send(res);
        });
    }

    fn total_da_categoria(&self, id: &str) -> u64 {
        self.tamanhos
            .get(id)
            .copied()
            .or_else(|| {
                self.scan_result
                    .as_ref()
                    .filter(|_| self.cat_atual.as_deref() == Some(id))
                    .map(|s| s.bytes)
            })
            .unwrap_or(0)
    }

    fn iniciar_limpeza(&mut self, id: &str) {
        let Some(cat) = self.categoria(id) else {
            return;
        };
        let cat_run = cat.clone();
        let total = self.total_da_categoria(id);
        let cancelar = Arc::new(AtomicBool::new(false));
        let lixeira = self.config.usar_lixeira;
        let opcoes = Opcoes {
            usar_lixeira: lixeira,
            cancelar: cancelar.clone(),
            total_estimado: total,
            lixeira_raiz: None,
        };
        let (tx, rx) = channel();
        self.job_rx = Some(rx);
        self.job_titulo = format!("Limpando: {}", cat_run.nome);
        self.job_cat_id = Some(cat_run.id.to_string());
        self.job_total = total;
        self.cancelar = Some(cancelar);
        self.pediu_cancelamento = false;
        self.prog_bytes = 0;
        self.prog_itens = 0;
        self.logs.clear();
        self.tela = Tela::Progresso;
        std::thread::spawn(move || cleaner::limpar(&cat_run, &opcoes, tx));
    }

    fn iniciar_limpeza_todas(&mut self) {
        let cats: Vec<Categoria> = self
            .categorias
            .iter()
            .filter(|c| c.nivel == Nivel::Basico && !c.informativo)
            .cloned()
            .collect();
        if cats.is_empty() {
            return;
        }
        let total: u64 = cats
            .iter()
            .map(|c| self.tamanhos.get(c.id).copied().unwrap_or(0))
            .sum();
        let cancelar = Arc::new(AtomicBool::new(false));
        let opcoes = Opcoes {
            usar_lixeira: self.config.usar_lixeira,
            cancelar: cancelar.clone(),
            total_estimado: total,
            lixeira_raiz: None,
        };
        let (tx, rx) = channel();
        self.job_rx = Some(rx);
        self.job_titulo = "Limpando categorias básicas".into();
        self.job_cat_id = None;
        self.job_total = total;
        self.cancelar = Some(cancelar);
        self.pediu_cancelamento = false;
        self.prog_bytes = 0;
        self.prog_itens = 0;
        self.logs.clear();
        self.tela = Tela::Progresso;
        std::thread::spawn(move || cleaner::limpar_todas(&cats, &opcoes, tx));
    }

    fn iniciar_job<F>(&mut self, titulo: String, f: F)
    where
        F: FnOnce(Sender<Msg>) + Send + 'static,
    {
        let (tx, rx) = channel();
        self.job_rx = Some(rx);
        self.job_titulo = titulo;
        self.job_cat_id = None;
        self.job_total = 0;
        self.cancelar = None;
        self.pediu_cancelamento = false;
        self.prog_bytes = 0;
        self.prog_itens = 0;
        self.logs.clear();
        self.historico_feito = false;
        self.tela = Tela::Progresso;
        std::thread::spawn(move || f(tx));
    }

    fn drenar_jobs(&mut self) {
        self.drenar_scan_geral();
        self.drenar_scan_categoria();
        self.drenar_pacotes();
        self.drenar_job();
    }

    fn drenar_scan_geral(&mut self) {
        let mut terminado = false;
        if let Some(rx) = &self.scan_rx {
            loop {
                match rx.try_recv() {
                    Ok((id, res)) => {
                        if res.bytes > 0 || res.itens > 0 || res.requer_root {
                            self.tamanhos.insert(id, res.bytes);
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        terminado = true;
                        break;
                    }
                }
            }
        }
        if terminado {
            self.scan_rx = None;
            self.escaneando = false;
        }
    }

    fn drenar_scan_categoria(&mut self) {
        let mut novo = None;
        let mut fim = false;
        if let Some(rx) = &self.scan_cat_rx {
            match rx.try_recv() {
                Ok(res) => {
                    novo = Some(res);
                    fim = true;
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => fim = true,
            }
        }
        if let Some(res) = novo {
            self.scan_result = Some(res);
            self.scan_pendente = false;
            self.scan_cat_rx = None;
        } else if fim {
            self.scan_pendente = false;
            self.scan_cat_rx = None;
            self.aviso_tela = "não foi possível analisar esta categoria.".into();
        }
    }

    fn drenar_pacotes(&mut self) {
        let mut recebidos = None;
        let mut falhou = false;
        if let Some(rx) = &self.pacotes_rx {
            match rx.try_recv() {
                Ok(pacotes) => recebidos = Some(pacotes),
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => falhou = true,
            }
        }
        if let Some(pacotes) = recebidos {
            self.pacotes = pacotes;
            self.carregando_pacotes = false;
            self.pacotes_rx = None;
        } else if falhou {
            self.carregando_pacotes = false;
            self.pacotes_rx = None;
            self.aviso_tela = "falha ao listar os programas instalados.".into();
        }
    }

    fn drenar_job(&mut self) {
        let mut fim = None;
        let mut desconectado = false;
        if let Some(rx) = &self.job_rx {
            loop {
                match rx.try_recv() {
                    Ok(Msg::Progresso(bytes, itens)) => {
                        self.prog_bytes = bytes;
                        self.prog_itens = itens;
                    }
                    Ok(Msg::Linha(linha)) => {
                        self.logs.push(linha);
                        if self.logs.len() > 400 {
                            self.logs.drain(..200);
                        }
                    }
                    Ok(Msg::Historico {
                        categoria,
                        bytes,
                        movidos,
                        erros,
                    }) => {
                        config::registrar_historico(
                            categoria,
                            bytes,
                            movidos,
                            erros,
                            self.config.historico_max,
                        );
                        self.historico_feito = true;
                    }
                    Ok(Msg::Fim(resultado)) => {
                        fim = Some(resultado);
                        break;
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        desconectado = true;
                        break;
                    }
                }
            }
        }
        if fim.is_none() && desconectado {
            fim = Some(Err(
                "a operação terminou inesperadamente (thread encerrada sem resultado).".into(),
            ));
        }
        if let Some(resultado) = fim {
            // Registra o histórico apenas se não foi feito já via
            // `Msg::Historico` (limpar_todas emite um por categoria).
            if !self.historico_feito
                && let Ok(resumo) = &resultado
                && let Some(id) = self.job_cat_id.clone()
            {
                config::registrar_historico(
                    &id,
                    resumo.bytes,
                    resumo.movidos,
                    resumo.erros,
                    self.config.historico_max,
                );
                self.registros = config::historico();
            }
            self.resultado = Some(resultado);
            self.resultado_titulo = self.job_titulo.clone();
            self.job_rx = None;
            self.cancelar = None;
            self.pediu_cancelamento = false;
            self.disco = cleaner::espaco_disco();
            self.tela = Tela::Resultado;
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let _guarda = GuardaTerminal::novo()?;
        instalar_hook_panico();
        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;

        loop {
            if event::poll(Duration::from_millis(80))?
                && let Event::Key(key) = event::read()?
                && key.kind != KeyEventKind::Release
            {
                self.tecla(key);
            }
            self.frame = self.frame.wrapping_add(1);
            self.drenar_jobs();
            terminal.draw(|f| ui::desenhar(f, self))?;
            if self.sair {
                break;
            }
        }
        Ok(())
    }

    fn tecla(&mut self, key: KeyEvent) {
        use KeyCode::*;

        // Saída garantida de qualquer tela.
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, Char('c') | Char('q'))
        {
            self.sair = true;
            return;
        }

        // modo de digitação (adicionar caminho)
        if self.digitando {
            match key.code {
                Char(c) => {
                    if !c.is_control() {
                        self.input_buf.push(c);
                    }
                }
                Backspace => {
                    self.input_buf.pop();
                }
                Enter => {
                    let bruto = self.input_buf.trim().to_string();
                    self.input_buf.clear();
                    self.digitando = false;
                    self.adicionar_caminho(&bruto);
                }
                Esc => {
                    self.input_buf.clear();
                    self.digitando = false;
                }
                _ => {}
            }
            return;
        }

        // confirmação modal
        if self.tela == Tela::Confirmar {
            match key.code {
                Up | Left => self.confirmar_idx = 0,
                Down | Right => self.confirmar_idx = 1,
                Char('h') | Char('H') => self.confirmar_idx = 0,
                Char('c') | Char('C') | Tab => self.confirmar_idx = 1,
                Enter => {
                    let acao = self.confirmar.take();
                    let anterior = self.tela_anterior;
                    self.tela = anterior;
                    if let (Some(acao), 0) = (acao, self.confirmar_idx) {
                        self.executar_confirmacao(acao);
                    }
                }
                Esc => {
                    self.confirmar = None;
                    self.tela = self.tela_anterior;
                }
                _ => {}
            }
            return;
        }

        match self.tela {
            Tela::Home => self.tecla_home(key),
            Tela::Detalhe => self.tecla_detalhe(key),
            Tela::Progresso => {
                if key.code == Esc && self.cancelar.is_some() {
                    if let Some(cancelar) = &self.cancelar {
                        cancelar.store(true, Ordering::Relaxed);
                    }
                    self.pediu_cancelamento = true;
                }
            }
            Tela::Resultado => {
                if matches!(key.code, Enter | Esc) {
                    self.tela = Tela::Home;
                    self.resultado = None;
                    self.iniciar_scan_tudo();
                }
            }
            Tela::Autostart => self.tecla_autostart(key),
            Tela::Desinstalar => self.tecla_desinstalar(key),
            Tela::Estatisticas => {
                if matches!(key.code, Esc | Enter | Char('q')) {
                    self.tela = Tela::Home;
                }
            }
            Tela::Config => self.tecla_config(key),
            Tela::Caminhos => self.tecla_caminhos(key),
            Tela::Ajuda => {
                if matches!(key.code, Esc | Char('q') | Char('?')) {
                    self.tela = self.tela_anterior;
                }
            }
            Tela::Confirmar => {}
        }
    }

    fn abrir_ajuda(&mut self) {
        self.tela_anterior = self.tela;
        self.tela = Tela::Ajuda;
    }

    fn tecla_home(&mut self, key: KeyEvent) {
        use KeyCode::*;
        let n = self.itens.len();
        match key.code {
            Up => self.sel = self.sel.saturating_sub(1),
            Down => self.sel = (self.sel + 1).min(n.saturating_sub(1)),
            PageUp => self.sel = self.sel.saturating_sub(10),
            PageDown => self.sel = (self.sel + 10).min(n.saturating_sub(1)),
            Home => self.sel = 0,
            End => self.sel = n.saturating_sub(1),
            Enter => self.abrir_item(),
            Char('q') => self.sair = true,
            Char('?') | Char('h') | Char('H') => self.abrir_ajuda(),
            Char('r') | Char('R') => self.iniciar_scan_tudo(),
            Char('l') | Char('L') => self.pedir_limpar_tudo(),
            Char('c') | Char('C') => {
                self.tela = Tela::Config;
                self.cfg_sel = 0;
            }
            _ => {}
        }
    }

    fn abrir_item(&mut self) {
        let (cat_id, ferramenta) = match self.itens.get(self.sel) {
            Some(i) => (i.cat_id.clone(), i.ferramenta),
            None => return,
        };
        if let Some(id) = cat_id {
            self.entrar_detalhe(&id);
            return;
        }
        let Some(ferramenta) = ferramenta else {
            return;
        };
        match ferramenta {
            Ferramenta::LimparTudo => self.pedir_limpar_tudo(),
            Ferramenta::Ajuda => self.abrir_ajuda(),
            Ferramenta::Autostart => {
                self.tela = Tela::Autostart;
                self.autostarts = tools::listar_autostart();
                self.autostart_sel = 0;
            }
            Ferramenta::Desinstalar => self.abrir_desinstalador(),
            Ferramenta::Estatisticas => {
                self.tela = Tela::Estatisticas;
                self.registros = config::historico();
            }
            Ferramenta::Configuracoes => {
                self.tela = Tela::Config;
                self.cfg_sel = 0;
            }
        }
    }

    fn tecla_detalhe(&mut self, key: KeyEvent) {
        use KeyCode::*;
        let id = self.cat_atual.clone().unwrap_or_default();
        match key.code {
            Esc => {
                self.tela = Tela::Home;
                self.cat_atual = None;
            }
            Up => self.acao_idx = self.acao_idx.saturating_sub(1),
            Down | Right => self.acao_idx = (self.acao_idx + 1).min(2),
            Enter => match self.acao_idx {
                0 => self.entrar_detalhe(&id),
                1 => self.tentar_limpar(&id),
                _ => {
                    self.tela = Tela::Home;
                    self.cat_atual = None;
                }
            },
            Char('l') | Char('L') => self.tentar_limpar(&id),
            Char('r') | Char('R') => self.entrar_detalhe(&id),
            Char('?') => self.abrir_ajuda(),
            _ => {}
        }
    }

    fn tentar_limpar(&mut self, id: &str) {
        let Some(cat) = self.categoria(id) else {
            return;
        };
        if cat.informativo {
            self.aviso_tela = "categoria informativa — nada a limpar.".into();
            return;
        }
        let procs = cleaner::processo_rodando(cat.bloqueio_proc);
        if !procs.is_empty() {
            self.aviso_tela = format!("feche os navegadores antes de limpar: {}", procs.join(", "));
            return;
        }
        self.confirmar = Some(AcaoConfirm::Limpar(id.to_string()));
        self.confirmar_idx = 0;
        self.tela_anterior = self.tela;
        self.tela = Tela::Confirmar;
    }

    fn pedir_limpar_tudo(&mut self) {
        self.confirmar = Some(AcaoConfirm::LimparTudo);
        self.confirmar_idx = 0;
        self.tela_anterior = self.tela;
        self.tela = Tela::Confirmar;
    }

    fn executar_confirmacao(&mut self, acao: AcaoConfirm) {
        match acao {
            AcaoConfirm::Limpar(id) => self.iniciar_limpeza(&id),
            AcaoConfirm::LimparTudo => self.iniciar_limpeza_todas(),
            AcaoConfirm::ExcluirAutostart(path) => {
                if let Err(e) = tools::excluir_autostart(&path) {
                    self.aviso_tela = format!("falha ao excluir: {e}");
                } else {
                    self.autostarts = tools::listar_autostart();
                    self.aviso_tela = "inicialização removida.".into();
                }
            }
            AcaoConfirm::Desinstalar(indice) => {
                if let Some(pacote) = self.pacotes.get(indice).cloned() {
                    let titulo = format!("Desinstalando: {}", pacote.nome);
                    let apagar_dados = self.config.apagar_dados_flatpak;
                    self.iniciar_job(titulo, move |tx| {
                        tools::desinstalar(&pacote, apagar_dados, tx);
                    });
                }
            }
            AcaoConfirm::RemoverCaminho(indice) => {
                if indice < self.config.caminhos.len() {
                    let removido = self.config.caminhos.remove(indice);
                    let _ = config::salvar(&self.config);
                    self.recarregar_categorias();
                    self.aviso_tela = format!("removido: {removido}");
                }
            }
        }
    }

    fn adicionar_caminho(&mut self, bruto: &str) {
        if bruto.is_empty() {
            return;
        }
        let expandido = seguranca::expandir(bruto);
        match seguranca::validar_caminho_custom(&expandido) {
            Ok(canonico) => {
                let texto = canonico.to_string_lossy().into_owned();
                if self.config.caminhos.contains(&texto) {
                    self.aviso_tela = format!("caminho já adicionado: {texto}");
                    return;
                }
                self.config.caminhos.push(texto.clone());
                if let Err(e) = config::salvar(&self.config) {
                    self.aviso_tela = format!("falha ao salvar configuração: {e}");
                    return;
                }
                self.recarregar_categorias();
                self.aviso_tela = format!("caminho adicionado: {texto}");
            }
            Err(erro) => {
                self.aviso_tela = format!("caminho inválido: {erro}");
            }
        }
    }

    fn recarregar_categorias(&mut self) {
        self.categorias = categories::categorias(&self.config);
        self.itens = Self::montar_itens(&self.categorias);
        self.sel = self.sel.min(self.itens.len().saturating_sub(1));
        self.iniciar_scan_tudo();
    }

    fn abrir_desinstalador(&mut self) {
        self.tela = Tela::Desinstalar;
        self.filtro.clear();
        self.pacote_sel = 0;
        self.pacotes.clear();
        self.carregando_pacotes = true;
        self.aviso_tela.clear();
        let (tx, rx) = channel();
        self.pacotes_rx = Some(rx);
        std::thread::spawn(move || {
            let pacotes = tools::listar_pacotes();
            let _ = tx.send(pacotes);
        });
    }

    fn pacotes_filtrados(&self) -> Vec<usize> {
        let f = self.filtro.to_lowercase();
        self.pacotes
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                f.is_empty()
                    || p.nome.to_lowercase().contains(&f)
                    || p.detalhe.to_lowercase().contains(&f)
            })
            .map(|(i, _)| i)
            .collect()
    }

    // ---------------- Autostart ----------------
    fn tecla_autostart(&mut self, key: KeyEvent) {
        use KeyCode::*;
        let n = self.autostarts.len();
        match key.code {
            Up => self.autostart_sel = self.autostart_sel.saturating_sub(1),
            Down => self.autostart_sel = (self.autostart_sel + 1).min(n.saturating_sub(1)),
            Char('a') | Enter => {
                if let Some(item) = self.autostarts.get(self.autostart_sel) {
                    if item.usuario {
                        let path = item.caminho.clone();
                        if let Err(e) = tools::alternar_autostart(&path) {
                            self.aviso_tela = format!("falha: {e}");
                        } else {
                            self.autostarts = tools::listar_autostart();
                        }
                    } else {
                        self.aviso_tela =
                            "entradas do sistema só podem ser alteradas com apoio do administrador."
                                .into();
                    }
                }
            }
            Char('x') => {
                if let Some(item) = self.autostarts.get(self.autostart_sel) {
                    if item.usuario {
                        self.confirmar = Some(AcaoConfirm::ExcluirAutostart(item.caminho.clone()));
                        self.confirmar_idx = 0;
                        self.tela_anterior = self.tela;
                        self.tela = Tela::Confirmar;
                    } else {
                        self.aviso_tela =
                            "entradas do sistema não podem ser excluídas por aqui.".into();
                    }
                }
            }
            Char('r') => self.autostarts = tools::listar_autostart(),
            Esc | Char('q') => self.tela = Tela::Home,
            Char('?') => self.abrir_ajuda(),
            _ => {}
        }
    }

    // ---------------- Desinstalador ----------------
    fn tecla_desinstalar(&mut self, key: KeyEvent) {
        use KeyCode::*;
        if self.carregando_pacotes {
            if matches!(key.code, Esc | Char('q')) {
                self.tela = Tela::Home;
            }
            return;
        }
        let filtrados = self.pacotes_filtrados();
        let n = filtrados.len();
        match key.code {
            Up => self.pacote_sel = self.pacote_sel.saturating_sub(1),
            Down => self.pacote_sel = (self.pacote_sel + 1).min(n.saturating_sub(1)),
            PageUp => self.pacote_sel = self.pacote_sel.saturating_sub(15),
            PageDown => self.pacote_sel = (self.pacote_sel + 15).min(n.saturating_sub(1)),
            Char(c) if !c.is_control() => {
                self.filtro.push(c);
                self.pacote_sel = 0;
            }
            Backspace => {
                self.filtro.pop();
                self.pacote_sel = 0;
            }
            Enter => {
                if let Some(&indice) = filtrados.get(self.pacote_sel) {
                    if let Some(pacote) = self.pacotes.get(indice)
                        && tools::pacote_critico(&pacote.nome)
                    {
                        self.aviso_tela = format!(
                            "'{}' é essencial para o sistema e não pode ser removido.",
                            pacote.nome
                        );
                        return;
                    }
                    self.confirmar = Some(AcaoConfirm::Desinstalar(indice));
                    self.confirmar_idx = 0;
                    self.tela_anterior = self.tela;
                    self.tela = Tela::Confirmar;
                }
            }
            Esc | Char('q') => {
                self.filtro.clear();
                self.tela = Tela::Home;
            }
            Char('?') => self.abrir_ajuda(),
            _ => {}
        }
    }

    // ---------------- Configurações ----------------
    fn numero_de_configuracoes(&self) -> usize {
        4
    }

    fn ajustar_numero(&mut self, delta: i64) {
        match self.cfg_sel {
            1 => {
                self.config.dias_tmp = (self.config.dias_tmp as i64 + delta).clamp(1, 365) as u64;
            }
            2 => {
                self.config.dias_logs =
                    (self.config.dias_logs as i64 + delta).clamp(1, 3650) as u64;
            }
            _ => return,
        }
        let _ = config::salvar(&self.config);
        self.recarregar_categorias();
    }

    fn tecla_config(&mut self, key: KeyEvent) {
        use KeyCode::*;
        match key.code {
            Up => self.cfg_sel = self.cfg_sel.saturating_sub(1),
            Down => {
                self.cfg_sel = (self.cfg_sel + 1).min(self.numero_de_configuracoes());
            }
            Left => self.ajustar_numero(-1),
            Right => self.ajustar_numero(1),
            Enter | Char(' ') => match self.cfg_sel {
                0 => {
                    self.config.usar_lixeira = !self.config.usar_lixeira;
                    let _ = config::salvar(&self.config);
                }
                3 => {
                    self.config.apagar_dados_flatpak = !self.config.apagar_dados_flatpak;
                    let _ = config::salvar(&self.config);
                }
                4 => self.tela = Tela::Caminhos,
                _ => {}
            },
            Esc | Char('q') => self.tela = Tela::Home,
            Char('?') => self.abrir_ajuda(),
            _ => {}
        }
    }

    // ---------------- Caminhos personalizados ----------------
    fn tecla_caminhos(&mut self, key: KeyEvent) {
        use KeyCode::*;
        let n = self.config.caminhos.len();
        match key.code {
            Up => self.cfg_sel = self.cfg_sel.saturating_sub(1),
            Down => self.cfg_sel = (self.cfg_sel + 1).min(n.saturating_sub(1)),
            Char('a') => {
                self.digitando = true;
                self.input_buf.clear();
            }
            Char('x') | Enter => {
                if self.cfg_sel < n {
                    self.confirmar = Some(AcaoConfirm::RemoverCaminho(self.cfg_sel));
                    self.confirmar_idx = 0;
                    self.tela_anterior = self.tela;
                    self.tela = Tela::Confirmar;
                }
            }
            Esc | Char('q') => self.tela = Tela::Config,
            Char('?') => self.abrir_ajuda(),
            _ => {}
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

// ===================== terminal =====================

struct GuardaTerminal;

impl GuardaTerminal {
    fn novo() -> anyhow::Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(Self)
    }
}

impl Drop for GuardaTerminal {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

fn instalar_hook_panico() {
    let anterior = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        anterior(info);
    }));
}

pub fn run() -> anyhow::Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        anyhow::bail!(
            "a interface TUI exige um terminal interativo; \
             use 'purgar scan' ou 'purgar --help'"
        );
    }
    let mut app = App::new();
    app.run()
}
