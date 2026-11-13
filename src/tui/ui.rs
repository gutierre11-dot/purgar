use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Gauge, Paragraph, Wrap};

use super::{AcaoConfirm, App, Ferramenta, Tela, VERSAO};
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

fn largura(texto: &str) -> usize {
    use unicode_width::UnicodeWidthStr;
    texto.width()
}

/// Trunca o texto respeitando a largura de display (acentos/emoji).
fn truncar_texto(texto: &str, limite: usize) -> String {
    if largura(texto) <= limite {
        return texto.to_string();
    }
    use unicode_width::UnicodeWidthChar;
    let mut out = String::new();
    let mut usado = 0;
    for c in texto.chars() {
        let w = UnicodeWidthChar::width(c).unwrap_or(1);
        if usado + w > limite.saturating_sub(1) {
            break;
        }
        out.push(c);
        usado += w;
    }
    out.push('…');
    out
}

/// Quebra texto simples por largura de display.
fn wrap_text(texto: &str, largura_max: usize) -> Vec<String> {
    let largura_max = largura_max.max(1);
    let mut out = Vec::new();
    for paragrafo in texto.split('\n') {
        let mut linha = String::new();
        let mut usado = 0usize;
        for palavra in paragrafo.split_whitespace() {
            let w = largura(palavra);
            if usado > 0 && usado + 1 + w > largura_max {
                out.push(std::mem::take(&mut linha));
                usado = 0;
            }
            if usado > 0 {
                linha.push(' ');
                usado += 1;
            }
            linha.push_str(palavra);
            usado += w;
        }
        out.push(linha);
    }
    out
}

/// Aplica rolagem vertical simples baseada na linha que deve ficar visível.
fn rolar(linhas: Vec<Line>, linha_alvo: usize, altura: u16) -> Vec<Line> {
    let visiveis = altura.saturating_sub(2) as usize;
    if visiveis == 0 {
        return Vec::new();
    }
    let offset = linha_alvo.saturating_sub(visiveis.saturating_sub(1));
    linhas.into_iter().skip(offset).take(visiveis).collect()
}

pub fn desenhar(f: &mut Frame, app: &App) {
    let area = f.area();
    let bg = Paragraph::new("").style(estilo_base());
    f.render_widget(bg, area);

    match app.tela {
        Tela::Home => desenhar_home(f, app, area),
        Tela::Detalhe => desenhar_detalhe(f, app, area),
        Tela::Progresso => desenhar_progresso(f, app, area),
        Tela::Resultado => desenhar_resultado(f, app, area),
        Tela::Autostart => desenhar_autostart(f, app, area),
        Tela::Desinstalar => desenhar_desinstalador(f, app, area),
        Tela::Estatisticas => desenhar_estatisticas(f, app, area),
        Tela::Config => desenhar_config(f, app, area),
        Tela::Caminhos => desenhar_caminhos(f, app, area),
        Tela::Ajuda => {
            desenhar_sob_confirmar(f, app, area);
            desenhar_ajuda(f, area);
        }
        Tela::Confirmar => {
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
    let p = Paragraph::new(Span::styled(
        truncar_texto(texto, area.width.saturating_sub(2) as usize),
        Style::default().fg(DIM),
    ))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(BORDA))
            .style(Style::default().bg(PANEL)),
    );
    f.render_widget(p, area);
}

fn bloco(titulo: &str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDA))
        .title(Span::styled(
            format!(" {titulo} "),
            Style::default().fg(DIM),
        ))
        .style(estilo_base())
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

    let lista_area = corpo[0];
    let mut linhas: Vec<Line> = Vec::new();
    let mut linha_do_item: Vec<usize> = Vec::with_capacity(app.itens.len());
    let mut secao_atual = "";
    for (idx, item) in app.itens.iter().enumerate() {
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
        let tam_txt = if let Some(id) = &item.cat_id {
            if tamanho > 0 {
                format!("  {}", fsutil::humano(tamanho))
            } else if app.escaneando {
                String::new()
            } else if app.categoria(id).map(|c| c.requer_admin()).unwrap_or(false) {
                "  admin".to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let estilo = if selecionado {
            Style::default().fg(Color::White).bg(PANEL2)
        } else {
            Style::default().fg(TEXTO)
        };
        linha_do_item.push(linhas.len());
        linhas.push(Line::from(vec![
            Span::styled(prefixo, estilo),
            Span::styled(item.rotulo.clone(), estilo),
            Span::styled(
                tam_txt,
                Style::default().fg(if selecionado { VERDE } else { DIM }),
            ),
        ]));
    }
    let linha_alvo = linha_do_item.get(app.sel).copied().unwrap_or(0);
    let p = Paragraph::new(rolar(linhas, linha_alvo, lista_area.height)).block(
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
        let mut linhas: Vec<Line> = vec![
            Line::from(Span::styled(
                item.secao.to_uppercase(),
                Style::default().fg(ROXO).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                item.rotulo.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        if let Some(id) = &item.cat_id {
            if let Some(cat) = app.categoria(id) {
                let largura_info = (info.width as usize).saturating_sub(4);
                for l in wrap_text(&cat.descricao, largura_info) {
                    linhas.push(Line::from(Span::styled(l, Style::default().fg(DIM))));
                }
                linhas.push(Line::from(""));
                let tam = app.tamanhos.get(id).copied().unwrap_or(0);
                if app.escaneando && tam == 0 {
                    linhas.push(Line::from(vec![
                        Span::styled(spinner(app.frame), Style::default().fg(ACCENT)),
                        Span::styled(" analisando...", Style::default().fg(DIM)),
                    ]));
                } else if cat.requer_admin() && tam == 0 {
                    linhas.push(Line::from(Span::styled(
                        "Requer administrador — o tamanho não foi medido.",
                        Style::default().fg(AMARELO),
                    )));
                } else if cat.informativo {
                    linhas.push(Line::from(vec![
                        Span::styled("Tamanho: ", Style::default().fg(DIM)),
                        Span::styled(
                            fsutil::humano(tam),
                            Style::default().fg(VERDE).add_modifier(Modifier::BOLD),
                        ),
                    ]));
                    linhas.push(Line::from(Span::styled(
                        "Somente análise — a limpeza deve ser manual.",
                        Style::default().fg(AMARELO),
                    )));
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
            }
        } else {
            let desc = match item.ferramenta {
                Some(Ferramenta::LimparTudo) => {
                    "Executa a limpeza de todas as categorias básicas de uma só vez, \
                     com confirmação e relatório final."
                }
                Some(Ferramenta::Autostart) => {
                    "Gerencia programas que iniciam automaticamente com o sistema."
                }
                Some(Ferramenta::Desinstalar) => {
                    "Lista e remove programas instalados (RPM, Flatpak, Snap). \
                     Pacotes essenciais são protegidos."
                }
                Some(Ferramenta::Estatisticas) => {
                    "Histórico de espaço liberado e movido para a lixeira."
                }
                Some(Ferramenta::Configuracoes) => {
                    "Preferências do aplicativo, retenção e caminhos personalizados."
                }
                Some(Ferramenta::Ajuda) => "Atalhos de teclado e notas de segurança.",
                None => "",
            };
            for l in wrap_text(desc, (info.width as usize).saturating_sub(4)) {
                linhas.push(Line::from(Span::styled(l, Style::default().fg(DIM))));
            }
        }

        let p = Paragraph::new(linhas).block(bloco("Detalhes"));
        f.render_widget(p, info);
    }

    rodape(
        f,
        vertical[2],
        "↑↓ navegar  •  Enter abrir  •  L limpar tudo  •  R reanalisar  •  ? ajuda  •  q sair",
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
    desenhar_cabecalho(f, app, vertical[0], &format!("Categoria: {nome}"));

    let corpo = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(vertical[1]);

    let cat = app.cat_atual.as_ref().and_then(|id| app.categoria(id));
    let mut linhas: Vec<Line> = Vec::new();
    if let Some(cat) = cat {
        for l in wrap_text(&cat.descricao, (corpo[0].width as usize).saturating_sub(4)) {
            linhas.push(Line::from(Span::styled(l, Style::default().fg(TEXTO))));
        }
        linhas.push(Line::from(""));

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
                linhas.push(Line::from(Span::styled(
                    "Categoria somente informativa — nada será removido.",
                    Style::default().fg(AMARELO),
                )));
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
                        "Parte da limpeza requer autenticação de administrador (pkexec).",
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
                        format!("  • {l}"),
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
            for aviso in &scan.avisos {
                linhas.push(Line::from(Span::styled(
                    format!("  ! {aviso}"),
                    Style::default().fg(AMARELO),
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
                "Modo lixeira ativo: os itens serão movidos, não apagados de imediato.",
                Style::default().fg(ROXO),
            )));
        }
    }

    let p = Paragraph::new(linhas).block(bloco("Descrição"));
    f.render_widget(p, corpo[0]);

    let acoes = ["Reanalisar", "Limpar agora", "Voltar"];
    let mut spans = Vec::new();
    for (i, a) in acoes.iter().enumerate() {
        let sel = i == app.acao_idx;
        spans.push(Span::styled(
            format!(" {a} "),
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
        "↑↓ escolher ação  •  Enter executar  •  L limpar  •  R reanalisar  •  Esc voltar",
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
            Constraint::Length(3),
            Constraint::Length(3),
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
        if app.pediu_cancelamento {
            Span::styled("  (cancelando...)", Style::default().fg(AMARELO))
        } else {
            Span::raw("")
        },
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ACCENT_DIM))
            .style(Style::default().bg(PANEL)),
    );
    f.render_widget(titulo, centro[0]);

    if app.job_total > 0 {
        let pct = ((app.prog_bytes as f64 / app.job_total as f64) * 100.0).min(100.0) as u16;
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(BORDA))
                    .title(Span::styled(" Progresso ", Style::default().fg(DIM))),
            )
            .gauge_style(
                Style::default()
                    .fg(VERDE)
                    .bg(PANEL2)
                    .add_modifier(Modifier::BOLD),
            )
            .percent(pct)
            .label(format!(
                " {} de {} ",
                fsutil::humano(app.prog_bytes),
                fsutil::humano(app.job_total)
            ));
        f.render_widget(gauge, centro[1]);
    } else {
        let p = Paragraph::new(Line::from(vec![
            Span::styled(spinner(app.frame), Style::default().fg(ACCENT)),
            Span::styled(
                format!("  processado: {}", fsutil::humano(app.prog_bytes)),
                Style::default().fg(TEXTO),
            ),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDA))
                .title(Span::styled(" Progresso ", Style::default().fg(DIM))),
        );
        f.render_widget(p, centro[1]);
    }

    let stats = Paragraph::new(Line::from(vec![
        Span::styled("Itens processados: ", Style::default().fg(DIM)),
        Span::styled(
            format!("{}", app.prog_itens),
            Style::default().fg(TEXTO).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "   •   Não feche o terminal durante a operação.",
            Style::default().fg(AMARELO),
        ),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(stats, centro[2]);

    let visiveis = centro[3].height.saturating_sub(2) as usize;
    let inicio = app.logs.len().saturating_sub(visiveis);
    let linhas: Vec<Line> = app.logs[inicio..]
        .iter()
        .map(|l| Line::from(Span::styled(l.clone(), Style::default().fg(DIM))))
        .collect();
    let log = Paragraph::new(linhas).block(bloco("Atividade"));
    f.render_widget(log, centro[3]);

    if app.cancelar.is_some() {
        rodape(
            f,
            vertical[2],
            "Esc cancelar a limpeza  •  Ctrl+C sair do programa",
        );
    } else {
        rodape(f, vertical[2], "Aguarde a conclusão...");
    }
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
        .constraints([Constraint::Length(13), Constraint::Min(3)])
        .split(vertical[1]);

    let mut linhas: Vec<Line> = Vec::new();
    match &app.resultado {
        Some(Ok(resumo)) => {
            let icone = if resumo.erros > 0 { "⚠" } else { "✓" };
            let cor_icone = if resumo.erros > 0 { AMARELO } else { VERDE };
            linhas.push(Line::from(vec![
                Span::styled(format!(" {icone} "), Style::default().fg(cor_icone)),
                Span::styled(
                    app.resultado_titulo.clone(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            linhas.push(Line::from(""));
            if resumo.cancelado {
                linhas.push(Line::from(Span::styled(
                    "Operação cancelada pelo usuário — parte dos itens não foi processada.",
                    Style::default().fg(AMARELO),
                )));
            }
            if resumo.lixeira {
                linhas.push(Line::from(vec![
                    Span::styled("Movidos para a lixeira: ", Style::default().fg(DIM)),
                    Span::styled(
                        format!("{} ({})", resumo.itens, fsutil::humano(resumo.movidos)),
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
            for l in resumo.linhas.iter().take(4) {
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

    let p = Paragraph::new(linhas).block(bloco("Concluído"));
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

    let mut linha_do_item: Vec<usize> = Vec::with_capacity(app.autostarts.len());
    for (i, item) in app.autostarts.iter().enumerate() {
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
        linha_do_item.push(linhas.len());
        linhas.push(Line::from(vec![
            Span::styled(if sel { "▸ " } else { "  " }, estilo),
            Span::styled(badge, Style::default().fg(cor)),
            Span::styled(item.nome.clone(), estilo.add_modifier(Modifier::BOLD)),
        ]));
        if sel && !item.exec.is_empty() {
            linhas.push(Line::from(Span::styled(
                format!("    {}", truncar_texto(&item.exec, 80)),
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
    if !app.aviso_tela.is_empty() {
        linhas.push(Line::from(""));
        linhas.push(Line::from(Span::styled(
            app.aviso_tela.clone(),
            Style::default().fg(AMARELO),
        )));
    }

    let linha_alvo = linha_do_item.get(app.autostart_sel).copied().unwrap_or(0);
    let p = Paragraph::new(rolar(linhas, linha_alvo, vertical[1].height)).block(bloco("Programas"));
    f.render_widget(p, vertical[1]);

    rodape(
        f,
        vertical[2],
        "↑↓ navegar  •  a/Enter ativar-desativar  •  x excluir  •  r atualizar  •  Esc voltar",
    );
}

// ===================== DESINSTALADOR =====================

fn desenhar_desinstalador(f: &mut Frame, app: &App, area: Rect) {
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

    if app.carregando_pacotes {
        let p = Paragraph::new(vec![
            Line::from(vec![
                Span::styled(spinner(app.frame), Style::default().fg(ACCENT)),
                Span::styled(
                    "  carregando programas instalados (RPM, Flatpak, Snap)...",
                    Style::default().fg(TEXTO),
                ),
            ]),
            Line::from(Span::styled(
                "Isso pode levar alguns segundos; a interface continua responsiva.",
                Style::default().fg(DIM),
            )),
        ])
        .block(bloco("Programas instalados"));
        f.render_widget(p, vertical[1]);
        let vazio = Paragraph::new("");
        f.render_widget(vazio, vertical[2]);
        rodape(f, vertical[3], "Esc — voltar");
        return;
    }

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
    .block(bloco(""));
    f.render_widget(filtro_p, vertical[1]);

    let mut linhas: Vec<Line> = Vec::new();
    let mut linha_do_item: Vec<usize> = Vec::with_capacity(filtrados.len());
    for (pos, &indice) in filtrados.iter().enumerate() {
        let pacote = &app.pacotes[indice];
        let sel = pos == app.pacote_sel;
        let estilo = if sel {
            Style::default().fg(Color::White).bg(PANEL2)
        } else {
            Style::default().fg(TEXTO)
        };
        let cor_origem = match pacote.origem {
            Origem::Rpm => ACCENT,
            Origem::Flatpak => ROXO,
            Origem::Snap => VERDE,
        };
        let protegido = crate::tools::pacote_critico(&pacote.nome);
        linha_do_item.push(linhas.len());
        let mut spans = vec![
            Span::styled(if sel { "▸ " } else { "  " }, estilo),
            Span::styled(
                format!("[{:8}] ", pacote.origem.rotulo()),
                Style::default().fg(cor_origem),
            ),
            Span::styled(pacote.nome.clone(), estilo.add_modifier(Modifier::BOLD)),
            Span::styled(format!("  {}", pacote.detalhe), Style::default().fg(DIM)),
        ];
        if protegido {
            spans.push(Span::styled("  (essencial)", Style::default().fg(VERMELHO)));
        }
        linhas.push(Line::from(spans));
    }
    if filtrados.is_empty() {
        linhas.push(Line::from(Span::styled(
            "Nenhum programa corresponde ao filtro.",
            Style::default().fg(DIM),
        )));
    }
    if !app.aviso_tela.is_empty() {
        linhas.push(Line::from(""));
        linhas.push(Line::from(Span::styled(
            app.aviso_tela.clone(),
            Style::default().fg(AMARELO),
        )));
    }

    let linha_alvo = linha_do_item.get(app.pacote_sel).copied().unwrap_or(0);
    let p = Paragraph::new(rolar(linhas, linha_alvo, vertical[2].height))
        .block(bloco("Programas instalados"));
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

    let total_liberado: u64 = app.registros.iter().map(|r| r.bytes).sum();
    let total_movido: u64 = app.registros.iter().map(|r| r.movidos).sum();

    let mut por_cat: Vec<(String, u64, u64)> = Vec::new();
    for r in &app.registros {
        if let Some(entrada) = por_cat.iter_mut().find(|(id, _, _)| *id == r.categoria) {
            entrada.1 += r.bytes;
            entrada.2 += r.movidos;
        } else {
            por_cat.push((r.categoria.clone(), r.bytes, r.movidos));
        }
    }
    por_cat.retain(|(_, bytes, movidos)| *bytes > 0 || *movidos > 0);
    por_cat.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.2.cmp(&a.2)));
    let max_cat = por_cat
        .iter()
        .map(|(_, b, m)| *b + *m)
        .max()
        .unwrap_or(1)
        .max(1);

    let mut linhas: Vec<Line> = Vec::new();
    linhas.push(Line::from(vec![
        Span::styled("Total liberado historicamente: ", Style::default().fg(DIM)),
        Span::styled(
            fsutil::humano(total_liberado),
            Style::default()
                .fg(VERDE)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
        ),
    ]));
    linhas.push(Line::from(vec![
        Span::styled("Total movido para a lixeira: ", Style::default().fg(DIM)),
        Span::styled(fsutil::humano(total_movido), Style::default().fg(ROXO)),
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
            "Nenhuma limpeza registrada ainda. Execute uma limpeza para começar.",
            Style::default().fg(DIM),
        )));
    } else {
        linhas.push(Line::from(Span::styled(
            "Por categoria:",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )));
        for (id, bytes, movidos) in por_cat.iter().take(10) {
            let nome = app.categoria(id).map(|c| c.nome).unwrap_or(id.as_str());
            let total = bytes + movidos;
            let barras = (((total * 20) / max_cat).max(1)) as usize;
            let barra = "█".repeat(barras);
            linhas.push(Line::from(vec![
                Span::styled(
                    format!("  {:<22}", truncar_texto(nome, 22)),
                    Style::default().fg(TEXTO),
                ),
                Span::styled(barra, Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}", fsutil::humano(total)),
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
            let nome = app
                .categoria(&r.categoria)
                .map(|c| c.nome)
                .unwrap_or(r.categoria.as_str());
            let ts = r.ts.replace('T', " ");
            let ts = &ts[..ts.len().min(19)];
            let mut spans = vec![
                Span::styled(format!("  {ts}  "), Style::default().fg(DIM)),
                Span::styled(truncar_texto(nome, 26), Style::default().fg(TEXTO)),
                Span::styled(
                    format!("  {}", fsutil::humano(r.bytes)),
                    Style::default().fg(VERDE),
                ),
            ];
            if r.movidos > 0 {
                spans.push(Span::styled(
                    format!("  {} na lixeira", fsutil::humano(r.movidos)),
                    Style::default().fg(ROXO),
                ));
            }
            if r.erros > 0 {
                spans.push(Span::styled(
                    format!("  ({} erro)", r.erros),
                    Style::default().fg(VERMELHO),
                ));
            }
            linhas.push(Line::from(spans));
        }
    }

    let p = Paragraph::new(linhas)
        .wrap(Wrap { trim: false })
        .block(bloco("Relatório"));
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

    let itens: [(String, String, String); 5] = [
        (
            "Mover itens para a Lixeira".into(),
            if app.config.usar_lixeira {
                "[x]".into()
            } else {
                "[ ]".into()
            },
            "Em vez de apagar, move para ~/.local/share/Trash (freedesktop).".into(),
        ),
        (
            "Dias de retenção em /tmp".into(),
            format!("{} dias  ←→", app.config.dias_tmp),
            "Arquivos de /tmp e /var/tmp mais antigos que isso podem ser removidos.".into(),
        ),
        (
            "Dias de retenção de logs".into(),
            format!("{} dias  ←→", app.config.dias_logs),
            "Logs rotacionados do sistema com mais idade que isso são removidos.".into(),
        ),
        (
            "Apagar dados ao desinstalar Flatpak".into(),
            if app.config.apagar_dados_flatpak {
                "[x]".into()
            } else {
                "[ ]".into()
            },
            "Passa --delete-data ao desinstalar: apaga também dados do aplicativo.".into(),
        ),
        (
            "Caminhos personalizados".into(),
            format!("{} caminho(s)", app.config.caminhos.len()),
            "Adiciona diretórios/arquivos extras para limpeza (validados por segurança).".into(),
        ),
    ];

    let mut linhas: Vec<Line> = Vec::new();
    for (i, (nome, valor, desc)) in itens.iter().enumerate() {
        let sel = i == app.cfg_sel;
        let estilo = if sel {
            Style::default().fg(Color::White).bg(PANEL2)
        } else {
            Style::default().fg(TEXTO)
        };
        linhas.push(Line::from(vec![
            Span::styled(if sel { "▸ " } else { "  " }, estilo),
            Span::styled(
                format!("{valor:<14}"),
                Style::default().fg(if valor.starts_with('[') {
                    if valor.starts_with("[x]") { VERDE } else { DIM }
                } else {
                    ACCENT
                }),
            ),
            Span::styled(nome.clone(), estilo.add_modifier(Modifier::BOLD)),
        ]));
        linhas.push(Line::from(Span::styled(
            format!("      {desc}"),
            Style::default().fg(DIM),
        )));
        linhas.push(Line::from(""));
    }

    let p = Paragraph::new(linhas).block(bloco("Opções"));
    f.render_widget(p, vertical[1]);

    rodape(
        f,
        vertical[2],
        "↑↓ navegar  •  ←→ ajustar dias  •  Enter alternar/abrir  •  Esc voltar",
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
                Span::styled(truncar_texto(caminho, 70), estilo),
            ]));
        }
    }
    linhas.push(Line::from(""));
    linhas.push(Line::from(Span::styled(
        "Diretórios têm o conteúdo esvaziado; arquivos são removidos.",
        Style::default().fg(DIM),
    )));
    linhas.push(Line::from(Span::styled(
        "Raízes do sistema, a home e caminhos rasos demais são rejeitados.",
        Style::default().fg(DIM),
    )));
    if !app.aviso_tela.is_empty() {
        linhas.push(Line::from(""));
        linhas.push(Line::from(Span::styled(
            app.aviso_tela.clone(),
            Style::default().fg(AMARELO),
        )));
    }

    let p = Paragraph::new(linhas).block(bloco("Caminhos"));
    f.render_widget(p, vertical[1]);

    let rodape_txt = if app.digitando {
        format!(" Digite o caminho e pressione Enter: {}█", app.input_buf)
    } else {
        "a adicionar  •  Enter/x remover selecionado  •  Esc voltar".to_string()
    };
    rodape(f, vertical[2], &rodape_txt);
}

// ===================== AJUDA =====================

fn desenhar_ajuda(f: &mut Frame, area: Rect) {
    let largura_modal = area.width.saturating_sub(4).min(86);
    let altura_modal = area.height.saturating_sub(4).min(28);
    let x = area.x + (area.width.saturating_sub(largura_modal)) / 2;
    let y = area.y + (area.height.saturating_sub(altura_modal)) / 2;
    let modal = Rect::new(x, y, largura_modal, altura_modal);
    f.render_widget(Clear, modal);

    let mut linhas: Vec<Line<'static>> = Vec::new();
    push_secao(&mut linhas, "Navegação");
    for (tecla, acao) in [
        ("↑ ↓ / PgUp PgDn", "mover seleção"),
        ("Enter", "abrir / executar"),
        ("Esc", "voltar / cancelar limpeza em andamento"),
        ("R", "reanalisar"),
        ("L", "limpar (categoria ou tudo, no painel)"),
        ("?", "esta ajuda"),
        ("q / Ctrl+C", "sair"),
    ] {
        linhas.push(Line::from(vec![
            Span::styled(format!("  {tecla:<16}"), Style::default().fg(VERDE)),
            Span::styled(acao, Style::default().fg(TEXTO)),
        ]));
    }
    push_secao(&mut linhas, "\nSegurança");
    for nota in [
        "Caminhos personalizados são validados: raízes do sistema e a home são rejeitadas.",
        "Modo lixeira segue a especificação freedesktop; falha de move vira erro, nunca remoção silenciosa.",
        "Em /tmp removemos apenas arquivos antigos do próprio usuário; sockets (.X11-unix, ssh-*) são preservados.",
        "Ações administrativas usam pkexec/polkit e exigem sessão gráfica.",
        "Pacotes essenciais (kernel, systemd, snapd, dnf, flatpak...) são protegidos no desinstalador.",
    ] {
        for l in wrap_text(nota, largura_modal.saturating_sub(6) as usize) {
            linhas.push(Line::from(Span::styled(
                format!("  • {l}"),
                Style::default().fg(DIM),
            )));
        }
    }

    let p = Paragraph::new(linhas)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ACCENT))
                .title(Span::styled(
                    " Ajuda e atalhos ",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(PANEL)),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(p, modal);
}

fn push_secao(linhas: &mut Vec<Line<'static>>, titulo: &str) {
    linhas.push(Line::from(Span::styled(
        titulo.to_string(),
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
    )));
}

// ===================== MODAL =====================

fn desenhar_sob_confirmar(f: &mut Frame, app: &App, area: Rect) {
    match app.tela_anterior {
        Tela::Home => desenhar_home(f, app, area),
        Tela::Detalhe => desenhar_detalhe(f, app, area),
        Tela::Autostart => desenhar_autostart(f, app, area),
        Tela::Desinstalar => desenhar_desinstalador(f, app, area),
        Tela::Caminhos => desenhar_caminhos(f, app, area),
        Tela::Config => desenhar_config(f, app, area),
        _ => desenhar_home(f, app, area),
    }
}

fn desenhar_modal_confirmar(f: &mut Frame, app: &App, area: Rect) {
    let w = area.width.saturating_sub(4).min(70);
    let h = area.height.saturating_sub(2).min(18);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let modal = Rect::new(x, y, w, h);

    f.render_widget(Clear, modal);

    let (titulo, detalhes) = match &app.confirmar {
        Some(AcaoConfirm::Limpar(id)) => {
            let cat = app.categoria(id);
            let tam = app
                .tamanhos
                .get(id.as_str())
                .copied()
                .or_else(|| {
                    app.scan_result
                        .as_ref()
                        .filter(|_| app.cat_atual.as_deref() == Some(id.as_str()))
                        .map(|s| s.bytes)
                })
                .unwrap_or(0);
            let mut detalhes = vec![
                format!("Categoria: {}", cat.map(|c| c.nome).unwrap_or("?")),
                format!("Espaço a liberar: {}", fsutil::humano(tam)),
            ];
            if cat.map(|c| c.requer_admin()).unwrap_or(false) {
                detalhes
                    .push("Parte da limpeza pedirá senha de administrador (pkexec).".to_string());
            }
            if app.config.usar_lixeira {
                detalhes.push("Modo lixeira: os itens serão movidos, não apagados.".to_string());
            } else {
                detalhes.push("Esta ação remove arquivos de cache e temporários.".to_string());
                detalhes.push("Senhas e documentos pessoais não são afetados.".to_string());
            }
            ("Confirmar limpeza".to_string(), detalhes)
        }
        Some(AcaoConfirm::LimparTudo) => {
            let basicas: Vec<&crate::model::Categoria> = app
                .categorias
                .iter()
                .filter(|c| c.nivel == crate::model::Nivel::Basico && !c.informativo)
                .collect();
            let total: u64 = basicas
                .iter()
                .map(|c| app.tamanhos.get(c.id).copied().unwrap_or(0))
                .sum();
            (
                "Limpar todas as categorias básicas?".to_string(),
                vec![
                    format!("Categorias: {}", basicas.len()),
                    format!("Espaço estimado: {}", fsutil::humano(total)),
                    if app.config.usar_lixeira {
                        "Modo lixeira: os itens serão movidos, não apagados.".to_string()
                    } else {
                        "Esta ação remove caches e temporários de todas as categorias básicas."
                            .to_string()
                    },
                    "Categorias que exigem administrador não fazem parte desta limpeza."
                        .to_string(),
                ],
            )
        }
        Some(AcaoConfirm::ExcluirAutostart(_)) => (
            "Excluir inicialização?".to_string(),
            vec![
                "O programa não iniciará mais com o sistema.".to_string(),
                "Esta ação remove o arquivo .desktop do usuário.".to_string(),
            ],
        ),
        Some(AcaoConfirm::Desinstalar(indice)) => {
            let pacote = app.pacotes.get(*indice);
            let mut detalhes = vec![
                format!(
                    "Programa: {}",
                    pacote.map(|p| p.nome.clone()).unwrap_or_default()
                ),
                format!(
                    "Origem: {}",
                    pacote.map(|p| p.origem.rotulo()).unwrap_or("?")
                ),
                "Esta ação não pode ser desfeita facilmente.".to_string(),
            ];
            if app.config.apagar_dados_flatpak
                && pacote.map(|p| p.origem == Origem::Flatpak).unwrap_or(false)
            {
                detalhes.push(
                    "Modo configurado: os DADOS do aplicativo também serão apagados.".to_string(),
                );
            }
            ("Desinstalar programa?".to_string(), detalhes)
        }
        Some(AcaoConfirm::RemoverCaminho(indice)) => (
            "Remover caminho?".to_string(),
            vec![
                format!(
                    "Caminho: {}",
                    app.config
                        .caminhos
                        .get(*indice)
                        .cloned()
                        .unwrap_or_default()
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
            format!(" {b} "),
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

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn wrap_respeita_largura_com_acentos() {
        let linhas = wrap_text("Área de Trabalho muito longa para caber na tela", 12);
        for linha in &linhas {
            assert!(largura(linha) <= 12, "linha larga demais: {linha:?}");
        }
        assert!(linhas.iter().any(|l| l.contains("Área")));
    }

    #[test]
    fn truncar_mantem_limite() {
        let texto = "configuração muito longa";
        let t = truncar_texto(texto, 10);
        assert!(largura(&t) <= 10);
        assert!(t.ends_with('…'));
    }

    #[test]
    fn rolagem_mantem_alvo_visivel() {
        let linhas: Vec<Line> = (0..50).map(|i| Line::from(format!("{i}"))).collect();
        let visiveis = rolar(linhas, 42, 10);
        assert!(visiveis.len() <= 8);
    }
}
