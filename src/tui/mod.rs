pub mod ui;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::categories;
use crate::cleaner::{self, Msg};
use crate::config::{self, Config, Registro};
use crate::model::{Categoria, Resumo, ScanResult};
use crate::tools::{self, AutostartItem, Pacote};

const VERSAO: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tela {
    Home,
    Detalhe,
    Progresso,
    Resultado,
    Autostart,
    Desinstalar,
    Estatisticas,
    Config,
    Caminhos,
    Confirmar,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Ferramenta {
    Autostart,
    Desinstalar,
    Estatisticas,
    Configuracoes,
}

struct ItemMenu {
    rotulo: String,
    secao: &'static str,
    cat_id: Option<String>,
    ferramenta: Option<Ferramenta>,
}

enum AcaoConfirm {
    Limpar(String),
    ExcluirAutostart(PathBuf),
    Desinstalar(usize), // índice em self.pacotes
    RemoverCaminho(usize),
}

pub struct App {
    tela: Tela,
    sair: bool,
    config: Config,
    categorias: Vec<Categoria>,
    itens: Vec<ItemMenu>,
    sel: usize,

    // dados da tela inicial
    tamanhos: HashMap<String, u64>,
    escaneando: bool,
    scan_rx: Option<Receiver<(String, ScanResult)>>,
    disco: Option<(u64, u64)>,

    // detalhe
    cat_atual: Option<String>,
    scan_cat_rx: Option<Receiver<ScanResult>>,
    scan_pendente: bool,
    scan_result: Option<ScanResult>,
    acao_idx: usize, // 0=escanear 1=limpar 2=voltar
    processos: Vec<String>,
    aviso_tela: String,

    // trabalho em segundo plano
    job_rx: Option<Receiver<Msg>>,
    job_titulo: String,
    prog_bytes: u64,
    prog_itens: u64,
    logs: Vec<String>,
    resultado: Option<Result<Resumo, String>>,
    resultado_titulo: String,

    // confirmação
    confirmar: Option<AcaoConfirm>,
    confirmar_idx: usize, // 0=confirmar 1=cancelar
    tela_anterior: Tela,

    // autostart
    autostarts: Vec<AutostartItem>,
    autostart_sel: usize,

    // desinstalador
    pacotes: Vec<Pacote>,
    pacote_sel: usize,
    filtro: String,

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
        let categorias = categories::categorias(&config.caminhos);
        let mut itens = Vec::new();
        for nivel in [crate::model::Nivel::Basico, crate::model::Nivel::Avancado] {
            let secao = nivel.rotulo();
            for c in categorias.iter().filter(|c| c.nivel == nivel) {
                itens.push(ItemMenu {
                    rotulo: c.nome.to_string(),
                    secao,
                    cat_id: Some(c.id.to_string()),
                    ferramenta: None,
                });
            }
        }
        for (rotulo, ferramenta, secao) in [
            (
                "Gerenciador de inicialização",
                Ferramenta::Autostart,
                "Ferramentas",
            ),
            (
                "Desinstalador de programas",
                Ferramenta::Desinstalar,
                "Ferramentas",
            ),
            (
                "Estatísticas de limpeza",
                Ferramenta::Estatisticas,
                "Ferramentas",
            ),
            ("Configurações", Ferramenta::Configuracoes, "Ferramentas"),
        ] {
            itens.push(ItemMenu {
                rotulo: rotulo.to_string(),
                secao,
                cat_id: None,
                ferramenta: Some(ferramenta),
            });
        }

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
            prog_bytes: 0,
            prog_itens: 0,
            logs: Vec::new(),
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
            registros: Vec::new(),
            cfg_sel: 0,
            digitando: false,
            input_buf: String::new(),
            frame: 0,
        };
        app.iniciar_scan_tudo();
        app
    }

    fn iniciar_scan_tudo(&mut self) {
        let cats = self.categorias.clone_shallow();
        let (tx, rx) = channel();
        self.scan_rx = Some(rx);
        self.escaneando = true;
        std::thread::spawn(move || {
            for c in &cats {
                let res = cleaner::escanear(c);
                let _ = tx.send((c.id.to_string(), res));
            }
        });
    }

    fn categoria(&self, id: &str) -> Option<&Categoria> {
        self.categorias.iter().find(|c| c.id == id)
    }

    fn entrar_detalhe(&mut self, id: &str) {
        self.tela = Tela::Detalhe;
        self.cat_atual = Some(id.to_string());
        self.acao_idx = 0;
        self.scan_result = None;
        self.scan_pendente = true;
        self.aviso_tela.clear();
        let (bloqueio, avisos) = match self.categoria(id) {
            Some(cat) => (cat.bloqueio_proc, cat.avisos_proc),
            None => (&[][..], &[][..]),
        };
        self.processos = cleaner::processo_rodando(bloqueio);
        if self.processos.is_empty() {
            self.processos = cleaner::processo_rodando(avisos);
        }
        let cat = match self.categoria(id) {
            Some(c) => c,
            None => return,
        };
        // precisa de 'static: clonamos o que o scan usa
        let alvos = cat.alvos.iter().map(|a| clone_alvo(a)).collect::<Vec<_>>();
        let cat_scan = Categoria {
            id: cat.id,
            nome: cat.nome,
            descricao: cat.descricao,
            nivel: cat.nivel,
            alvos,
            avisos_proc: cat.avisos_proc,
            bloqueio_proc: cat.bloqueio_proc,
            informativo: cat.informativo,
        };
        let (tx, rx) = channel();
        self.scan_cat_rx = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(cleaner::escanear(&cat_scan));
        });
    }

    fn iniciar_limpeza(&mut self, id: &str) {
        let cat = match self.categoria(id) {
            Some(c) => c,
            None => return,
        };
        let alvos = cat.alvos.iter().map(|a| clone_alvo(a)).collect::<Vec<_>>();
        let cat_run = Categoria {
            id: cat.id,
            nome: cat.nome,
            descricao: cat.descricao,
            nivel: cat.nivel,
            alvos,
            avisos_proc: cat.avisos_proc,
            bloqueio_proc: cat.bloqueio_proc,
            informativo: cat.informativo,
        };
        let lixeira = self.config.usar_lixeira;
        let (tx, rx) = channel();
        self.job_rx = Some(rx);
        self.job_titulo = format!("Limpando: {}", cat_run.nome);
        self.prog_bytes = 0;
        self.prog_itens = 0;
        self.logs.clear();
        self.tela = Tela::Progresso;
        std::thread::spawn(move || cleaner::limpar(&cat_run, lixeira, tx));
    }

    fn iniciar_job<F>(&mut self, titulo: String, f: F)
    where
        F: FnOnce(Sender<Msg>) + Send + 'static,
    {
        let (tx, rx) = channel();
        self.job_rx = Some(rx);
        self.job_titulo = titulo;
        self.prog_bytes = 0;
        self.prog_itens = 0;
        self.logs.clear();
        self.tela = Tela::Progresso;
        std::thread::spawn(move || f(tx));
    }

    fn drenar_jobs(&mut self) {
        // scan geral da home
        if let Some(rx) = &self.scan_rx {
            let mut done = false;
            while let Ok((id, res)) = rx.try_recv() {
                if res.bytes > 0 || res.itens > 0 {
                    self.tamanhos.insert(id.clone(), res.bytes);
                }
                if id == "downloads" {
                    // downloads também é medido
                }
            }
            if self
                .scan_rx
                .as_ref()
                .map(|r| r.try_recv().is_err())
                .unwrap_or(true)
            {
                // canal pode ter mais itens; verifica se thread acabou é difícil.
                // usamos flag: quando try_recv retorna Empty por muito tempo, assumimos ok.
            }
            let _ = &mut done;
        }
        // scan da categoria aberta
        if let Some(rx) = &self.scan_cat_rx {
            if let Ok(res) = rx.try_recv() {
                self.scan_result = Some(res);
                self.scan_pendente = false;
                self.scan_cat_rx = None;
            }
        }
        // job principal
        if let Some(rx) = &self.job_rx {
            let mut fim = None;
            loop {
                match rx.try_recv() {
                    Ok(Msg::Progresso(b, i)) => {
                        self.prog_bytes = b;
                        self.prog_itens = i;
                    }
                    Ok(Msg::Linha(l)) => {
                        self.logs.push(l);
                        if self.logs.len() > 200 {
                            self.logs.drain(..100);
                        }
                    }
                    Ok(Msg::Fim(r)) => {
                        fim = Some(r);
                        break;
                    }
                    Err(_) => break,
                }
            }
            if let Some(r) = fim {
                match &r {
                    Ok(resumo) => {
                        if self.job_titulo.starts_with("Limpando") {
                            let cat_id = self
                                .cat_atual
                                .clone()
                                .unwrap_or_else(|| "desconhecida".into());
                            config::registrar_historico(&cat_id, resumo.bytes, resumo.erros);
                            self.registros = tools::registros();
                        }
                    }
                    Err(e) => {
                        self.logs.push(format!("Falha: {}", e));
                    }
                }
                self.resultado = Some(r);
                self.resultado_titulo = self.job_titulo.clone();
                self.job_rx = None;
                self.tela = Tela::Resultado;
            }
        }
    }

    fn marca_scan_completo(&mut self) {
        // chamado quando o canal de scan geral está vazio
        self.escaneando = false;
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;

        loop {
            if event::poll(std::time::Duration::from_millis(80))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind != KeyEventKind::Release {
                        self.tecla(key);
                    }
                }
            }
            self.frame += 1;
            self.drenar_jobs();
            // detecta fim do scan geral: canal fechado E todos recebidos
            if self.escaneando {
                if let Some(rx) = &self.scan_rx {
                    // try_recv já drenado; se disconnect → finished
                    match rx.try_recv() {
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            self.marca_scan_completo();
                        }
                        _ => {}
                    }
                }
            }
            terminal.draw(|f| ui::desenhar(f, self))?;
            if self.sair {
                break;
            }
        }

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        Ok(())
    }

    fn tecla(&mut self, key: crossterm::event::KeyEvent) {
        use KeyCode::*;
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
                    let caminho = self.input_buf.trim().to_string();
                    self.input_buf.clear();
                    self.digitando = false;
                    if !caminho.is_empty() {
                        if !self.config.caminhos.contains(&caminho) {
                            self.config.caminhos.push(caminho.clone());
                            let _ = config::salvar(&self.config);
                            self.recarregar_categorias();
                            self.aviso_tela = format!("Caminho adicionado: {}", caminho);
                        }
                    }
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
                if key.code == Esc {
                    // não permite cancelar; apenas ignora
                }
            }
            Tela::Resultado => {
                if matches!(key.code, Enter | Esc) {
                    self.tela = Tela::Home;
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
            Tela::Confirmar => {}
        }

        if key.code == Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.sair = true;
        }
    }

    fn tecla_home(&mut self, key: crossterm::event::KeyEvent) {
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
        } else if let Some(ferramenta) = ferramenta {
            match ferramenta {
                Ferramenta::Autostart => {
                    self.tela = Tela::Autostart;
                    self.autostarts = tools::listar_autostart();
                    self.autostart_sel = 0;
                }
                Ferramenta::Desinstalar => {
                    self.tela = Tela::Desinstalar;
                    self.filtro.clear();
                    self.pacote_sel = 0;
                    self.carregar_pacotes();
                }
                Ferramenta::Estatisticas => {
                    self.tela = Tela::Estatisticas;
                    self.registros = tools::registros();
                }
                Ferramenta::Configuracoes => {
                    self.tela = Tela::Config;
                    self.cfg_sel = 0;
                }
            }
        }
    }

    fn tecla_detalhe(&mut self, key: crossterm::event::KeyEvent) {
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
                0 => self.resscan_categoria(),
                1 => self.tentar_limpar(&id),
                _ => {
                    self.tela = Tela::Home;
                    self.cat_atual = None;
                }
            },
            Char('l') | Char('L') => self.tentar_limpar(&id),
            _ => {}
        }
    }

    fn tentar_limpar(&mut self, id: &str) {
        let cat = match self.categoria(id) {
            Some(c) => c,
            None => return,
        };
        if cat.informativo {
            self.aviso_tela = "Categoria informativa — nada a limpar.".into();
            return;
        }
        // bloqueio por processos
        let procs = cleaner::processo_rodando(cat.bloqueio_proc);
        if !procs.is_empty() {
            self.aviso_tela = format!("Feche os navegadores antes de limpar: {}", procs.join(", "));
            return;
        }
        self.confirmar = Some(AcaoConfirm::Limpar(id.to_string()));
        self.confirmar_idx = 0;
        self.tela_anterior = self.tela;
        self.tela = Tela::Confirmar;
    }

    fn executar_confirmacao(&mut self, acao: AcaoConfirm) {
        match acao {
            AcaoConfirm::Limpar(id) => {
                let volt = self.tela;
                let _ = volt;
                self.iniciar_limpeza(&id);
            }
            AcaoConfirm::ExcluirAutostart(path) => {
                if let Err(e) = tools::excluir_autostart(&path) {
                    self.aviso_tela = format!("Falha ao excluir: {}", e);
                } else {
                    self.autostarts = tools::listar_autostart();
                    self.aviso_tela = "Inicialização removida.".into();
                }
            }
            AcaoConfirm::Desinstalar(idx) => {
                if let Some(p) = self.pacotes.get(idx).cloned() {
                    let nome = p.nome.clone();
                    self.iniciar_job(format!("Desinstalando: {}", nome), move |tx| {
                        tools::desinstalar(&p, tx);
                    });
                }
            }
            AcaoConfirm::RemoverCaminho(idx) => {
                if idx < self.config.caminhos.len() {
                    let removido = self.config.caminhos.remove(idx);
                    let _ = config::salvar(&self.config);
                    self.recarregar_categorias();
                    self.aviso_tela = format!("Removido: {}", removido);
                }
            }
        }
    }

    fn resscan_categoria(&mut self) {
        let id = match self.cat_atual.clone() {
            Some(i) => i,
            None => return,
        };
        self.entrar_detalhe(&id);
    }

    fn recarregar_categorias(&mut self) {
        self.categorias = categories::categorias(&self.config.caminhos);
        // remonta itens preservando ferramentas
        let ferramentas = [
            ("Gerenciador de inicialização", Ferramenta::Autostart),
            ("Desinstalador de programas", Ferramenta::Desinstalar),
            ("Estatísticas de limpeza", Ferramenta::Estatisticas),
            ("Configurações", Ferramenta::Configuracoes),
        ];
        let mut itens = Vec::new();
        for nivel in [crate::model::Nivel::Basico, crate::model::Nivel::Avancado] {
            let secao = nivel.rotulo();
            for c in self.categorias.iter().filter(|c| c.nivel == nivel) {
                itens.push(ItemMenu {
                    rotulo: c.nome.to_string(),
                    secao,
                    cat_id: Some(c.id.to_string()),
                    ferramenta: None,
                });
            }
        }
        for (rotulo, ferramenta) in ferramentas {
            itens.push(ItemMenu {
                rotulo: rotulo.to_string(),
                secao: "Ferramentas",
                cat_id: None,
                ferramenta: Some(ferramenta),
            });
        }
        self.itens = itens;
        self.sel = self.sel.min(self.itens.len().saturating_sub(1));
        self.tamanhos.clear();
        self.iniciar_scan_tudo();
    }

    fn carregar_pacotes(&mut self) {
        let pacotes = tools::listar_pacotes();
        self.pacotes = pacotes;
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
    fn tecla_autostart(&mut self, key: crossterm::event::KeyEvent) {
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
                            self.aviso_tela = format!("Falha: {}", e);
                        } else {
                            self.autostarts = tools::listar_autostart();
                        }
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
                    }
                }
            }
            Char('r') => self.autostarts = tools::listar_autostart(),
            Esc | Char('q') => self.tela = Tela::Home,
            _ => {}
        }
    }

    // ---------------- Desinstalador ----------------
    fn tecla_desinstalar(&mut self, key: crossterm::event::KeyEvent) {
        use KeyCode::*;
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
                if let Some(&idx) = filtrados.get(self.pacote_sel) {
                    self.confirmar = Some(AcaoConfirm::Desinstalar(idx));
                    self.confirmar_idx = 0;
                    self.tela_anterior = self.tela;
                    self.tela = Tela::Confirmar;
                }
            }
            Esc | Char('q') => {
                self.filtro.clear();
                self.tela = Tela::Home;
            }
            _ => {}
        }
    }

    // ---------------- Configurações ----------------
    fn tecla_config(&mut self, key: crossterm::event::KeyEvent) {
        use KeyCode::*;
        match key.code {
            Up => self.cfg_sel = self.cfg_sel.saturating_sub(1),
            Down => self.cfg_sel = (self.cfg_sel + 1).min(1),
            Enter | Char(' ') => match self.cfg_sel {
                0 => {
                    self.config.usar_lixeira = !self.config.usar_lixeira;
                    let _ = config::salvar(&self.config);
                }
                _ => {
                    self.tela = Tela::Caminhos;
                }
            },
            Esc | Char('q') => self.tela = Tela::Home,
            _ => {}
        }
    }

    // ---------------- Caminhos personalizados ----------------
    fn tecla_caminhos(&mut self, key: crossterm::event::KeyEvent) {
        use KeyCode::*;
        let n = self.config.caminhos.len();
        match key.code {
            Up => self.cfg_sel = self.cfg_sel.saturating_sub(1),
            Down => self.cfg_sel = (self.cfg_sel + 1).min(n.saturating_sub(1)),
            Char('a') => {
                self.digitando = true;
                self.input_buf.clear();
            }
            Char('x') => {
                if self.cfg_sel < n {
                    self.confirmar = Some(AcaoConfirm::RemoverCaminho(self.cfg_sel));
                    self.confirmar_idx = 0;
                    self.tela_anterior = self.tela;
                    self.tela = Tela::Confirmar;
                }
            }
            Enter => {
                if self.cfg_sel < n {
                    self.confirmar = Some(AcaoConfirm::RemoverCaminho(self.cfg_sel));
                    self.confirmar_idx = 0;
                    self.tela_anterior = self.tela;
                    self.tela = Tela::Confirmar;
                }
            }
            Esc | Char('q') => self.tela = Tela::Config,
            _ => {}
        }
    }
}

// ===================== helpers =====================

trait CloneShallow {
    fn clone_shallow(&self) -> Vec<Categoria>;
}

impl CloneShallow for Vec<Categoria> {
    fn clone_shallow(&self) -> Vec<Categoria> {
        self.iter()
            .map(|c| Categoria {
                id: c.id,
                nome: c.nome,
                descricao: c.descricao,
                nivel: c.nivel,
                alvos: c.alvos.iter().map(clone_alvo).collect(),
                avisos_proc: c.avisos_proc,
                bloqueio_proc: c.bloqueio_proc,
                informativo: c.informativo,
            })
            .collect()
    }
}

fn clone_alvo(a: &crate::model::Alvo) -> crate::model::Alvo {
    use crate::model::Alvo::*;
    match a {
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
    }
}

pub fn run() -> anyhow::Result<()> {
    let mut app = App::new();
    app.run()
}
