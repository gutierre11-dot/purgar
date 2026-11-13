use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Gauge, Paragraph, Wrap};

use super::{App, Ferramenta, Tela, VERSAO};
use crate::fsutil;
use crate::tools::Origem;

// ---------- paleta ----------
const BG: Color = Color::Rgb(13, 17, 23);
const PANEL: Color = Color::Rgb(22, 27, 34);
const PANEL2: Color = Color::Rgb(28, 35, 45);
const BORDA: Color = Color::Rgb(48, 54, 61);
const ACCENT: Color = Color::Rgb(88, 166, 255);
const ACCENT_DIM: Color = Color::Rgb(56, 90, 140);
const VERDE: Color = Color::Rgb(63, 185, 80);
const AMARELO: Color = Color::Rgb(210, 153, 34);
const VERMELHO: Color = Color::Rgb(248, 81, 73);
const TEXTO: Color = Color::Rgb(201, 209, 217);
const DIM: Color = Color::Rgb(139, 148, 158);
const ROXO: Color = Color::Rgb(188, 140, 255);

fn estilo_base() -> Style {
    Style::default().fg(TEXTO).bg(BG)
}

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

fn spinner(frame: u64) -> &'static str {
    SPINNER[(frame as usize) % SPINNER.len()]
}

/// Quebra texto simples por largura.
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    for paragrafo in text.split("\n") {
        let mut linha = String::new();
        for palavra in paragrafo.split_whitespace() {
            if linha.len() + palavra.len() + 1 > width && !linha.is_empty() {
                out.push(std::mem::take(&mut linha));
            }
            if !linha.is_empty() {
                linha.push(' ');
            }
            linha.push_str(palavra);
        }
        out.push(linha);
    }
    out
}

pub fn desenhar(f: &mut Frame, app: &mut App) {
    let area = f.area();
    // fundo
    let bg = Paragraph::new("").style(estilo_base());
    f.render_widget(bg, area);

    match app.tela {
        Tela::Home => desenhar_home(f, app, area),
        Tela::Detalhe => desenhar_detalhe(f, app, area),
        Tela::Progresso => desenhar_progresso(f, app, area),
        Tela::Resultado => desenhar_resultado(f, app, area),
        Tela::Autostart => desenhar_autostart(f, app, area),
        Tela::Desinstalar => desenhar_desinstalar(f, app, area),
        Tela::Estatisticas => desenhar_estatisticas(f, app, area),
        Tela::Config => desenhar_config(f, app, area),
        Tela::Caminhos => desenhar_caminhos(f, app, area),
        Tela::Confirmar => {
            // desenha a tela anterior por baixo
            desenhar_sob_confirmar(f, app, area);
            desenhar_modal_confirmar(f, app, area);
        }
    }
}

fn desenhar_cabecalho(f: &mut Frame, app: &App, area: Rect, titulo: &str) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(38)])
        .split(area);

    let logo = Line::from(vec![
        Span::styled(
            " PURGAR ",
            Style::default()
                .fg(Color::Black)
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("v{}  ", VERSAO), Style::default().fg(DIM)),
        Span::styled(
            titulo,
            Style::default().fg(TEXTO).add_modifier(Modifier::BOLD),
        ),
    ]);
    let header = Paragraph::new(logo).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(BORDA))
            .style(Style::default().bg(PANEL)),
    );
    f.render_widget(header, chunks[0]);

    // disco à direita
    if let Some((total, livre)) = app.disco {
        let usado_pct = if total > 0 {
            (total.saturating_sub(livre)) as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        let gauge = Gauge::default()
            .gauge_style(
                Style::default()
                    .fg(if usado_pct > 90.0 { VERMELHO } else { ACCENT })
                    .bg(PANEL2)
                    .add_modifier(Modifier::BOLD),
            )
            .percent(usado_pct as u16)
            .label(format!(
                " Disco: {} livres de {} ",
                fsutil::humano(livre),
                fsutil::humano(total)
            ));
        f.render_widget(gauge, chunks[1]);
    }
}

fn rodape(f: &mut Frame, area: Rect, texto: &str) {
    let p = Paragraph::new(Span::styled(texto.to_string(), Style::default().fg(DIM)))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(BORDA))
                .style(Style::default().bg(PANEL)),
        );
    f.render_widget(p, area);
}

// ===================== HOME =====================

fn desenhar_home(f: &mut Frame, app: &App, area: Rect) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(area);
    desenhar_cabecalho(f, app, vertical[0], "Painel de limpeza");

    let corpo = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
        .split(vertical[1]);

    // ---- lista esquerda ----
    let lista_area = corpo[0];
    let mut linhas: Vec<Line> = Vec::new();
    let mut idx = 0usize;
    let mut secao_atual = "";
    for item in &app.itens {
        if item.secao != secao_atual {
            secao_atual = item.secao;
            linhas.push(Line::from(Span::styled(
                format!(" {}", secao_atual.to_uppercase()),
                Style::default().fg(ACCENT_DIM).add_modifier(Modifier::BOLD),
            )));
            linhas.push(Line::from(""));
        }
        let selecionado = idx == app.sel;
        let prefixo = if selecionado { "▸ " } else { "  " };
        let tamanho = item
            .cat_id
            .as_ref()
            .and_then(|id| app.tamanhos.get(id))
            .copied()
            .unwrap_or(0);
        let tam_txt = if tamanho > 0 {
            format!("  {}", fsutil::humano(tamanho))
        } else {
            String::new()
        };
        let estilo = if selecionado {
            Style::default().fg(Color::White).bg(PANEL2)
        } else {
            Style::default().fg(TEXTO)
        };
        linhas.push(Line::from(vec![
            Span::styled(prefixo, estilo),
            Span::styled(item.rotulo.clone(), estilo),
            Span::styled(
                tam_txt,
                Style::default().fg(if selecionado { VERDE } else { DIM }),
            ),
        ]));
        idx += 1;
    }

    // rolagem simples
    let visiveis = lista_area.height as usize;
    let offset = app.sel.saturating_sub(visiveis.saturating_sub(3));
    let linhas: Vec<Line> = linhas.into_iter().skip(offset).collect();
    let p = Paragraph::new(linhas).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDA))
            .title(Span::styled(" Categorias ", Style::default().fg(DIM)))
            .style(estilo_base()),
    );
    f.render_widget(p, lista_area);

    // ---- painel direito ----
    let info = corpo[1];
    if let Some(item) = app.itens.get(app.sel) {
        let mut linhas: Vec<Line> = Vec::new();
        linhas.push(Line::from(Span::styled(
            item.secao.to_uppercase(),
            Style::default().fg(ROXO).add_modifier(Modifier::BOLD),
        )));
        linhas.push(Line::from(""));
        linhas.push(Line::from(Span::styled(
            item.rotulo.clone(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )));
        linhas.push(Line::from(""));

        if let Some(id) = &item.cat_id {
            if let Some(cat) = app.categoria(id) {
                for l in wrap_text(cat.descricao, (info.width as usize).saturating_sub(4)) {
                    linhas.push(Line::from(Span::styled(l, Style::default().fg(DIM))));
                }
                linhas.push(Line::from(""));
                let tam = app.tamanhos.get(id).copied().unwrap_or(0);
                if app.escaneando && tam == 0 {
                    linhas.push(Line::from(vec![
                        Span::styled(spinner(app.frame), Style::default().fg(ACCENT)),
                        Span::styled(" analisando...", Style::default().fg(DIM)),
                    ]));
                } else if tam > 0 {
                    linhas.push(Line::from(vec![
                        Span::styled("Espaço recuperável: ", Style::default().fg(DIM)),
                        Span::styled(
                            fsutil::humano(tam),
                            Style::default().fg(VERDE).add_modifier(Modifier::BOLD),
                        ),
                    ]));
                } else {
                    linhas.push(Line::from(Span::styled(
                        "Nada a limpar — sistema já está otimizado.",
                        Style::default().fg(VERDE),
                    )));
                }
                if !cat.bloqueio_proc.is_empty() {
                    linhas.push(Line::from(""));
                    linhas.push(Line::from(Span::styled(
                        "Requer navegadores fechados",
                        Style::default().fg(AMARELO),
                    )));
                }
                if cat.informativo {
                    linhas.push(Line::from(Span::styled(
                        "Somente análise",
                        Style::default().fg(AMARELO),
                    )));
                }
            }
        } else {
            let desc = match item.ferramenta {
                Some(Ferramenta::Autostart) => {
                    "Gerencia programas que iniciam automaticamente com o sistema."
                }
                Some(Ferramenta::Desinstalar) => {
                    "Lista e remove programas instalados (RPM, Flatpak, Snap)."
                }
                Some(Ferramenta::Estatisticas) => {
                    "Histórico de espaço liberado nas limpezas anteriores."
                }
                Some(Ferramenta::Configuracoes) => {
                    "Preferências do aplicativo e caminhos personalizados."
                }
                None => "",
            };
            for l in wrap_text(desc, (info.width as usize).saturating_sub(4)) {
                linhas.push(Line::from(Span::styled(l, Style::default().fg(DIM))));
            }
        }

        let p = Paragraph::new(linhas).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDA))
                .title(Span::styled(" Detalhes ", Style::default().fg(DIM)))
                .style(estilo_base()),
        );
        f.render_widget(p, info);
    }

    rodape(
        f,
        vertical[2],
        "↑↓ navegar  •  Enter abrir  •  PageUp/PageDown pular  •  q sair  •  Ctrl+C sair",
    );
}

// ===================== DETALHE =====================

fn desenhar_detalhe(f: &mut Frame, app: &App, area: Rect) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(area);
    let nome = app
        .cat_atual
        .as_ref()
        .and_then(|id| app.categoria(id))
        .map(|c| c.nome)
        .unwrap_or("?");
    desenhar_cabecalho(f, app, vertical[0], &format!("Categoria: {}", nome));

    let corpo = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(vertical[1]);

    let cat = app.cat_atual.as_ref().and_then(|id| app.categoria(id));
    let mut linhas: Vec<Line> = Vec::new();
    if let Some(cat) = cat {
        for l in wrap_text(cat.descricao, (corpo[0].width as usize).saturating_sub(4)) {
            linhas.push(Line::from(Span::styled(l, Style::default().fg(TEXTO))));
        }
        linhas.push(Line::from(""));

        // tamanho / status
        if app.scan_pendente {
            linhas.push(Line::from(vec![
                Span::styled(spinner(app.frame), Style::default().fg(ACCENT)),
                Span::styled(" analisando itens...", Style::default().fg(DIM)),
            ]));
        } else if let Some(scan) = &app.scan_result {
            if cat.informativo {
                linhas.push(Line::from(vec![
                    Span::styled("Tamanho: ", Style::default().fg(DIM)),
                    Span::styled(
                        fsutil::humano(scan.bytes),
                        Style::default().fg(VERDE).add_modifier(Modifier::BOLD),
                    ),
                ]));
            } else {
                linhas.push(Line::from(vec![
                    Span::styled("Será liberado: ", Style::default().fg(DIM)),
                    Span::styled(
                        fsutil::humano(scan.bytes),
                        Style::default().fg(VERDE).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  ({} itens)", scan.itens),
                        Style::default().fg(DIM),
                    ),
                ]));
                if scan.requer_root {
                    linhas.push(Line::from(Span::styled(
                        "Parte da limpeza requerá autenticação de administrador.",
                        Style::default().fg(AMARELO),
                    )));
                }
            }
            linhas.push(Line::from(""));
            linhas.push(Line::from(Span::styled(
                "O que será afetado:",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )));
            for d in scan.detalhes.iter().take(14) {
                for l in wrap_text(d, (corpo[0].width as usize).saturating_sub(6)) {
                    linhas.push(Line::from(Span::styled(
                        format!("  • {}", l),
                        Style::default().fg(DIM),
                    )));
                }
            }
            if scan.detalhes.len() > 14 {
                linhas.push(Line::from(Span::styled(
                    format!("  ... e mais {} item(ns)", scan.detalhes.len() - 14),
                    Style::default().fg(DIM),
                )));
            }
        }

        if !cat.avisos_proc.is_empty() || !cat.bloqueio_proc.is_empty() {
            linhas.push(Line::from(""));
            if !app.processos.is_empty() {
                linhas.push(Line::from(Span::styled(
                    format!("Processos detectados: {}", app.processos.join(", ")),
                    Style::default().fg(if cat.bloqueio_proc.is_empty() {
                        AMARELO
                    } else {
                        VERMELHO
                    }),
                )));
            }
        }
        if !app.aviso_tela.is_empty() {
            linhas.push(Line::from(""));
            linhas.push(Line::from(Span::styled(
                app.aviso_tela.clone(),
                Style::default().fg(AMARELO),
            )));
        }
        if app.config.usar_lixeira {
            linhas.push(Line::from(""));
            linhas.push(Line::from(Span::styled(
                "Modo lixeira ativo: itens serão movidos, não apagados.",
                Style::default().fg(ROXO),
            )));
        }
    }

    let p = Paragraph::new(linhas).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDA))
            .title(Span::styled(" Descrição ", Style::default().fg(DIM)))
            .style(estilo_base()),
    );
    f.render_widget(p, corpo[0]);

    // ações
    let acoes = ["Reanalisar", "Limpar agora", "Voltar"];
    let mut spans = Vec::new();
    for (i, a) in acoes.iter().enumerate() {
        let sel = i == app.acao_idx;
        spans.push(Span::styled(
            format!(" {} ", a),
            if sel {
                Style::default()
                    .fg(Color::Black)
                    .bg(ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(DIM)
            },
        ));
        spans.push(Span::raw("  "));
    }
    let p = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDA))
            .style(Style::default().bg(PANEL)),
    );
    f.render_widget(p, corpo[1]);

    rodape(
        f,
        vertical[2],
        "↑↓ escolher ação  •  Enter executar  •  Esc voltar  •  L limpar direto",
    );
}

// ===================== PROGRESSO =====================

fn desenhar_progresso(f: &mut Frame, app: &App, area: Rect) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(area);
    desenhar_cabecalho(f, app, vertical[0], "Operação em andamento");

    let centro = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Min(3),
        ])
        .split(vertical[1]);

    let titulo = Paragraph::new(Line::from(vec![
        Span::styled(spinner(app.frame), Style::default().fg(ACCENT)),
        Span::raw(" "),
        Span::styled(
            app.job_titulo.clone(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ACCENT_DIM))
            .style(Style::default().bg(PANEL)),
    );
    f.render_widget(titulo, centro[0]);

    let stats = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Processado: ", Style::default().fg(DIM)),
            Span::styled(
                fsutil::humano(app.prog_bytes),
                Style::default().fg(VERDE).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  •  {} item(ns)", app.prog_itens),
                Style::default().fg(DIM),
            ),
        ]),
        Line::from(Span::styled(
            "Não feche o terminal durante a operação.",
            Style::default().fg(AMARELO),
        )),
    ])
    .alignment(Alignment::Center)
    .block(Block::default().style(estilo_base()));
    f.render_widget(stats, centro[1]);

    // log
    let inicio = app.logs.len().saturating_sub(centro[2].height as usize);
    let linhas: Vec<Line> = app.logs[inicio..]
        .iter()
        .map(|l| Line::from(Span::styled(l.clone(), Style::default().fg(DIM))))
        .collect();
    let log = Paragraph::new(linhas).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDA))
            .title(Span::styled(" Atividade ", Style::default().fg(DIM)))
            .style(estilo_base()),
    );
    f.render_widget(log, centro[2]);

    rodape(f, vertical[2], "Aguarde a conclusão...");
}

// ===================== RESULTADO =====================

fn desenhar_resultado(f: &mut Frame, app: &App, area: Rect) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(area);
    desenhar_cabecalho(f, app, vertical[0], "Resultado");

    let centro = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(3)])
        .split(vertical[1]);

    let mut linhas: Vec<Line> = Vec::new();
    match &app.resultado {
        Some(Ok(resumo)) => {
            let icone = if resumo.erros > 0 { "⚠" } else { "✓" };
            let cor_icon = if resumo.erros > 0 { AMARELO } else { VERDE };
            linhas.push(Line::from(vec![
                Span::styled(format!(" {} ", icone), Style::default().fg(cor_icon)),
                Span::styled(
                    app.resultado_titulo.clone(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            linhas.push(Line::from(""));
            if resumo.lixeira {
                linhas.push(Line::from(vec![
                    Span::styled("Itens movidos para a lixeira: ", Style::default().fg(DIM)),
                    Span::styled(
                        format!("{}", resumo.itens),
                        Style::default().fg(TEXTO).add_modifier(Modifier::BOLD),
                    ),
                ]));
                linhas.push(Line::from(Span::styled(
                    "Esvazie a lixeira para liberar o espaço de fato.",
                    Style::default().fg(DIM),
                )));
            } else {
                linhas.push(Line::from(vec![
                    Span::styled("Espaço liberado: ", Style::default().fg(DIM)),
                    Span::styled(
                        fsutil::humano(resumo.bytes),
                        Style::default().fg(VERDE).add_modifier(Modifier::BOLD),
                    ),
                ]));
            }
            if resumo.erros > 0 {
                linhas.push(Line::from(vec![
                    Span::styled("Erros: ", Style::default().fg(DIM)),
                    Span::styled(
                        format!("{}", resumo.erros),
                        Style::default().fg(VERMELHO).add_modifier(Modifier::BOLD),
                    ),
                ]));
            }
            for l in resumo.linhas.iter().take(3) {
                linhas.push(Line::from(Span::styled(
                    l.clone(),
                    Style::default().fg(DIM),
                )));
            }
        }
        Some(Err(e)) => {
            linhas.push(Line::from(Span::styled(
                "✗ Operação falhou",
                Style::default().fg(VERMELHO).add_modifier(Modifier::BOLD),
            )));
            linhas.push(Line::from(""));
            for l in wrap_text(e, (centro[0].width as usize).saturating_sub(4)) {
                linhas.push(Line::from(Span::styled(l, Style::default().fg(TEXTO))));
            }
            let inicio = app.logs.len().saturating_sub(6);
            for l in &app.logs[inicio..] {
                linhas.push(Line::from(Span::styled(
                    l.clone(),
                    Style::default().fg(DIM),
                )));
            }
        }
        None => {}
    }

    let p = Paragraph::new(linhas).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDA))
            .title(Span::styled(" Concluído ", Style::default().fg(DIM)))
            .style(estilo_base()),
    );
    f.render_widget(p, centro[0]);

    let acoes = Paragraph::new(Line::from(vec![
        Span::styled(" [Enter] ", Style::default().fg(Color::Black).bg(ACCENT)),
        Span::styled("Voltar ao início", Style::default().fg(DIM)),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(acoes, centro[1]);

    rodape(f, vertical[2], "Enter/Esc — voltar ao painel inicial");
}

// ===================== AUTOSTART =====================

fn desenhar_autostart(f: &mut Frame, app: &App, area: Rect) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(area);
    desenhar_cabecalho(f, app, vertical[0], "Inicialização de programas");

    let n_usuario = app.autostarts.iter().filter(|a| a.usuario).count();
    let n_sistema = app.autostarts.len() - n_usuario;

    let mut linhas: Vec<Line> = vec![Line::from(Span::styled(
        format!(
            " {} programa(s) do usuário • {} do sistema • a/Enter alternar • x excluir",
            n_usuario, n_sistema
        ),
        Style::default().fg(DIM),
    ))];
    linhas.push(Line::from(""));

    let visiveis = vertical[1].height as usize;
    let offset = app.autostart_sel.saturating_sub(visiveis.saturating_sub(3));
    for (i, item) in app.autostarts.iter().enumerate().skip(offset) {
        let sel = i == app.autostart_sel;
        let estilo = if sel {
            Style::default().fg(Color::White).bg(PANEL2)
        } else {
            Style::default().fg(TEXTO)
        };
        let (badge, cor) = if !item.usuario {
            ("SISTEMA ", DIM)
        } else if item.habilitado {
            ("ATIVO   ", VERDE)
        } else {
            ("INATIVO ", AMARELO)
        };
        linhas.push(Line::from(vec![
            Span::styled(if sel { "▸ " } else { "  " }, estilo),
            Span::styled(badge, Style::default().fg(cor)),
            Span::styled(item.nome.clone(), estilo.add_modifier(Modifier::BOLD)),
        ]));
        if sel {
            let exec = if item.exec.len() > 60 {
                format!("{}...", &item.exec[..57])
            } else {
                item.exec.clone()
            };
            linhas.push(Line::from(Span::styled(
                format!("    {}", exec),
                Style::default().fg(DIM),
            )));
        }
    }
    if app.autostarts.is_empty() {
        linhas.push(Line::from(Span::styled(
            "Nenhum programa de inicialização encontrado.",
            Style::default().fg(DIM),
        )));
    }

    let p = Paragraph::new(linhas).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDA))
            .title(Span::styled(" Programas ", Style::default().fg(DIM)))
            .style(estilo_base()),
    );
    f.render_widget(p, vertical[1]);

    rodape(
        f,
        vertical[2],
        "↑↓ navegar  •  a/Enter ativar-desativar  •  x excluir  •  r atualizar  •  Esc voltar",
    );
}

// ===================== DESINSTALADOR =====================

fn desenhar_desinstalar(f: &mut Frame, app: &App, area: Rect) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(area);
    desenhar_cabecalho(f, app, vertical[0], "Desinstalador de programas");

    let filtrados = app.pacotes_filtrados();
    let rpm = app
        .pacotes
        .iter()
        .filter(|p| p.origem == Origem::Rpm)
        .count();
    let flat = app
        .pacotes
        .iter()
        .filter(|p| p.origem == Origem::Flatpak)
        .count();
    let snap = app
        .pacotes
        .iter()
        .filter(|p| p.origem == Origem::Snap)
        .count();

    let filtro_p = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Filtro: ", Style::default().fg(DIM)),
            Span::styled(
                if app.filtro.is_empty() {
                    "digite para filtrar...".to_string()
                } else {
                    app.filtro.clone()
                },
                Style::default().fg(ACCENT),
            ),
        ]),
        Line::from(Span::styled(
            format!(
                "{} programas ({} RPM • {} Flatpak • {} Snap) — {} correspondência(s)",
                app.pacotes.len(),
                rpm,
                flat,
                snap,
                filtrados.len()
            ),
            Style::default().fg(DIM),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDA))
            .style(estilo_base()),
    );
    f.render_widget(filtro_p, vertical[1]);

    let visiveis = vertical[2].height as usize;
    let offset = app.pacote_sel.saturating_sub(visiveis.saturating_sub(1));
    let mut linhas: Vec<Line> = Vec::new();
    for (pos, &idx) in filtrados.iter().enumerate().skip(offset) {
        let p = &app.pacotes[idx];
        let sel = pos == app.pacote_sel;
        let estilo = if sel {
            Style::default().fg(Color::White).bg(PANEL2)
        } else {
            Style::default().fg(TEXTO)
        };
        let cor_orig = match p.origem {
            Origem::Rpm => ACCENT,
            Origem::Flatpak => ROXO,
            Origem::Snap => VERDE,
        };
        linhas.push(Line::from(vec![
            Span::styled(if sel { "▸ " } else { "  " }, estilo),
            Span::styled(
                format!("[{:8}] ", p.origem.rotulo()),
                Style::default().fg(cor_orig),
            ),
            Span::styled(p.nome.clone(), estilo.add_modifier(Modifier::BOLD)),
            Span::styled(format!("  {}", p.detalhe), Style::default().fg(DIM)),
        ]));
    }
    if filtrados.is_empty() {
        linhas.push(Line::from(Span::styled(
            "Nenhum programa corresponde ao filtro.",
            Style::default().fg(DIM),
        )));
    }

    let p = Paragraph::new(linhas).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDA))
            .title(Span::styled(
                " Programas instalados ",
                Style::default().fg(DIM),
            ))
            .style(estilo_base()),
    );
    f.render_widget(p, vertical[2]);

    rodape(
        f,
        vertical[3],
        "Digite para filtrar  •  ↑↓ navegar  •  Enter desinstalar  •  Backspace apagar  •  Esc voltar",
    );
}

// ===================== ESTATÍSTICAS =====================

fn desenhar_estatisticas(f: &mut Frame, app: &App, area: Rect) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(area);
    desenhar_cabecalho(f, app, vertical[0], "Estatísticas de limpeza");

    let total: u64 = app.registros.iter().map(|r| r.bytes).sum();
    let mut por_cat: Vec<(String, u64)> = Vec::new();
    for r in &app.registros {
        if let Some(e) = por_cat.iter_mut().find(|(n, _)| *n == r.categoria) {
            e.1 += r.bytes;
        } else {
            por_cat.push((r.categoria.clone(), r.bytes));
        }
    }
    por_cat.sort_by(|a, b| b.1.cmp(&a.1));
    let max_cat = por_cat.first().map(|(_, b)| *b).unwrap_or(1).max(1);

    let mut linhas: Vec<Line> = Vec::new();
    linhas.push(Line::from(vec![
        Span::styled("Total liberado historicamente: ", Style::default().fg(DIM)),
        Span::styled(
            fsutil::humano(total),
            Style::default()
                .fg(VERDE)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
        ),
    ]));
    linhas.push(Line::from(vec![
        Span::styled("Operações registradas: ", Style::default().fg(DIM)),
        Span::styled(
            format!("{}", app.registros.len()),
            Style::default().fg(TEXTO),
        ),
    ]));
    linhas.push(Line::from(""));

    if por_cat.is_empty() {
        linhas.push(Line::from(Span::styled(
            "Nenhuma limpeza registrada ainda. Execute uma limpeza para começar a registrar.",
            Style::default().fg(DIM),
        )));
    } else {
        linhas.push(Line::from(Span::styled(
            "Por categoria:",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )));
        for (nome, bytes) in por_cat.iter().take(10) {
            let barras = ((bytes * 20) / max_cat).max(1) as usize;
            let barra = "█".repeat(barras);
            linhas.push(Line::from(vec![
                Span::styled(format!("  {:<20}", nome), Style::default().fg(TEXTO)),
                Span::styled(barra, Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}", fsutil::humano(*bytes)),
                    Style::default().fg(DIM),
                ),
            ]));
        }
        linhas.push(Line::from(""));
        linhas.push(Line::from(Span::styled(
            "Últimas operações:",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )));
        for r in app.registros.iter().rev().take(8) {
            linhas.push(Line::from(vec![
                Span::styled(format!("  {}  ", r.ts), Style::default().fg(DIM)),
                Span::styled(r.categoria.clone(), Style::default().fg(TEXTO)),
                Span::styled(
                    format!("  {}", fsutil::humano(r.bytes)),
                    Style::default().fg(VERDE),
                ),
                if r.erros > 0 {
                    Span::styled(
                        format!("  ({} erro)", r.erros),
                        Style::default().fg(VERMELHO),
                    )
                } else {
                    Span::raw("")
                },
            ]));
        }
    }

    let p = Paragraph::new(linhas).wrap(Wrap { trim: false }).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDA))
            .title(Span::styled(" Relatório ", Style::default().fg(DIM)))
            .style(estilo_base()),
    );
    f.render_widget(p, vertical[1]);

    rodape(f, vertical[2], "Esc/Enter — voltar");
}

// ===================== CONFIG =====================

fn desenhar_config(f: &mut Frame, app: &App, area: Rect) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(area);
    desenhar_cabecalho(f, app, vertical[0], "Configurações");

    let itens = [
        (
            "Mover itens para a Lixeira",
            app.config.usar_lixeira,
            "Em vez de apagar permanentemente, move para ~/.local/share/Trash",
        ),
        (
            "Caminhos personalizados",
            false,
            &format!("{} caminho(s) configurado(s)", app.config.caminhos.len()),
        ),
    ];

    let mut linhas: Vec<Line> = Vec::new();
    for (i, (nome, ativo, desc)) in itens.iter().enumerate() {
        let sel = i == app.cfg_sel;
        let estilo = if sel {
            Style::default().fg(Color::White).bg(PANEL2)
        } else {
            Style::default().fg(TEXTO)
        };
        linhas.push(Line::from(vec![
            Span::styled(if sel { "▸ " } else { "  " }, estilo),
            Span::styled(
                if *ativo { "[x] " } else { "[ ] " },
                Style::default().fg(if *ativo { VERDE } else { DIM }),
            ),
            Span::styled(nome.to_string(), estilo.add_modifier(Modifier::BOLD)),
        ]));
        linhas.push(Line::from(Span::styled(
            format!("    {}", desc),
            Style::default().fg(DIM),
        )));
        linhas.push(Line::from(""));
    }

    let p = Paragraph::new(linhas).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDA))
            .title(Span::styled(" Opções ", Style::default().fg(DIM)))
            .style(estilo_base()),
    );
    f.render_widget(p, vertical[1]);

    rodape(
        f,
        vertical[2],
        "↑↓ navegar  •  Enter/ESPAÇO alternar ou abrir  •  Esc voltar",
    );
}

// ===================== CAMINHOS =====================

fn desenhar_caminhos(f: &mut Frame, app: &App, area: Rect) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(area);
    desenhar_cabecalho(f, app, vertical[0], "Caminhos personalizados");

    let mut linhas: Vec<Line> = Vec::new();
    if app.config.caminhos.is_empty() {
        linhas.push(Line::from(Span::styled(
            "Nenhum caminho configurado.",
            Style::default().fg(DIM),
        )));
    } else {
        for (i, caminho) in app.config.caminhos.iter().enumerate() {
            let sel = i == app.cfg_sel;
            let estilo = if sel {
                Style::default().fg(Color::White).bg(PANEL2)
            } else {
                Style::default().fg(TEXTO)
            };
            linhas.push(Line::from(vec![
                Span::styled(if sel { "▸ " } else { "  " }, estilo),
                Span::styled(caminho.clone(), estilo),
            ]));
        }
    }
    linhas.push(Line::from(""));
    linhas.push(Line::from(Span::styled(
        "Diretórios têm o conteúdo esvaziado; arquivos são removidos.",
        Style::default().fg(DIM),
    )));

    let p = Paragraph::new(linhas).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDA))
            .title(Span::styled(" Caminhos ", Style::default().fg(DIM)))
            .style(estilo_base()),
    );
    f.render_widget(p, vertical[1]);

    let rodape_txt = if app.digitando {
        format!(" Digite o caminho e pressione Enter: {}█", app.input_buf)
    } else {
        "a adicionar  •  Enter/x remover selecionado  •  Esc voltar".to_string()
    };
    rodape(f, vertical[2], &rodape_txt);
}

// ===================== MODAL =====================

fn desenhar_sob_confirmar(f: &mut Frame, app: &App, area: Rect) {
    match app.tela_anterior {
        Tela::Home => desenhar_home(f, app, area),
        Tela::Detalhe => desenhar_detalhe(f, app, area),
        Tela::Autostart => desenhar_autostart(f, app, area),
        Tela::Desinstalar => desenhar_desinstalar(f, app, area),
        Tela::Caminhos => desenhar_caminhos(f, app, area),
        Tela::Config => desenhar_config(f, app, area),
        _ => desenhar_home(f, app, area),
    }
}

fn desenhar_modal_confirmar(f: &mut Frame, app: &App, area: Rect) {
    let w = area.width.min(64);
    let h = area.height.min(16);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let modal = Rect::new(x, y, w, h);

    f.render_widget(Clear, modal);

    let (titulo, detalhes) = match &app.confirmar {
        Some(crate::tui::AcaoConfirm::Limpar(id)) => {
            let cat = app.categoria(id);
            let tam = app
                .tamanhos
                .get(id.as_str())
                .copied()
                .or_else(|| app.scan_result.as_ref().map(|s| s.bytes))
                .unwrap_or(0);
            (
                "Confirmar limpeza".to_string(),
                vec![
                    format!("Categoria: {}", cat.map(|c| c.nome).unwrap_or("?")),
                    format!("Espaço a liberar: {}", fsutil::humano(tam)),
                    "Esta ação remove arquivos de cache e temporários.".to_string(),
                    "Itens essenciais (senhas, documentos) não são afetados.".to_string(),
                ],
            )
        }
        Some(crate::tui::AcaoConfirm::ExcluirAutostart(_)) => (
            "Excluir inicialização?".to_string(),
            vec!["O programa não iniciará mais com o sistema.".to_string()],
        ),
        Some(crate::tui::AcaoConfirm::Desinstalar(idx)) => {
            let p = app.pacotes.get(*idx);
            (
                "Desinstalar programa?".to_string(),
                vec![
                    format!(
                        "Programa: {}",
                        p.map(|p| p.nome.clone()).unwrap_or_default()
                    ),
                    format!("Origem: {}", p.map(|p| p.origem.rotulo()).unwrap_or("?")),
                    "Esta ação não pode ser desfeita facilmente.".to_string(),
                ],
            )
        }
        Some(crate::tui::AcaoConfirm::RemoverCaminho(idx)) => (
            "Remover caminho?".to_string(),
            vec![
                format!(
                    "Caminho: {}",
                    app.config.caminhos.get(*idx).cloned().unwrap_or_default()
                ),
                "O caminho deixará de ser limpo pelo aplicativo.".to_string(),
            ],
        ),
        None => ("Confirmar?".to_string(), vec![]),
    };

    let mut linhas: Vec<Line> = Vec::new();
    linhas.push(Line::from(Span::styled(
        titulo,
        Style::default().fg(AMARELO).add_modifier(Modifier::BOLD),
    )));
    linhas.push(Line::from(""));
    for d in detalhes {
        for l in wrap_text(&d, (w as usize).saturating_sub(6)) {
            linhas.push(Line::from(Span::styled(l, Style::default().fg(TEXTO))));
        }
    }
    linhas.push(Line::from(""));

    let botoes = ["Confirmar", "Cancelar"];
    let mut spans = Vec::new();
    for (i, b) in botoes.iter().enumerate() {
        let sel = i == app.confirmar_idx;
        spans.push(Span::styled(
            format!(" {} ", b),
            if sel {
                Style::default()
                    .fg(Color::Black)
                    .bg(ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(DIM)
            },
        ));
        spans.push(Span::raw("  "));
    }
    linhas.push(Line::from(spans));

    let p = Paragraph::new(linhas)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(AMARELO))
                .style(Style::default().bg(PANEL)),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(p, modal);
}
