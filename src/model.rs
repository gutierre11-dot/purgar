use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Nivel {
    Basico,
    Avancado,
}

impl Nivel {
    pub fn rotulo(self) -> &'static str {
        match self {
            Nivel::Basico => "Básico",
            Nivel::Avancado => "Avançado",
        }
    }
}

/// Uma unidade de limpeza dentro de uma categoria.
#[derive(Debug)]
pub enum Alvo {
    /// Apaga o conteúdo do diretório, mantendo o diretório.
    /// `excluir` lista nomes de filhos (nível 1) que não devem ser tocados.
    Esvaziar {
        dir: PathBuf,
        excluir: &'static [&'static str],
    },
    /// Apaga o caminho inteiro (arquivo ou diretório).
    Remover(PathBuf),
    /// Varre `raiz` até `prof` níveis e remove diretórios cujo nome esteja em `nomes`.
    /// Útil para caches de navegadores em perfis variados.
    Nomes {
        raiz: PathBuf,
        nomes: &'static [&'static str],
        prof: usize,
    },
    /// Remove arquivos/diretórios mais antigos que `dias` dias.
    /// Se `sudo` for true, executa via pkexec; senão, remove como usuário.
    Antigos {
        dirs: Vec<PathBuf>,
        dias: u64,
        sudo: bool,
    },
    /// Comando executado como o usuário atual.
    ComandoUser(Vec<String>),
    /// Comando executado com pkexec (raiz).
    ComandoRoot(Vec<String>),
}

pub struct Categoria {
    pub id: &'static str,
    pub nome: &'static str,
    pub descricao: &'static str,
    pub nivel: Nivel,
    pub alvos: Vec<Alvo>,
    /// Nomes de processos: se algum estiver rodando, a TUI exibe aviso.
    pub avisos_proc: &'static [&'static str],
    /// Nomes de processos: se algum estiver rodando, a limpeza fica bloqueada.
    pub bloqueio_proc: &'static [&'static str],
    /// Categoria apenas informativa (ex.: Downloads) — não limpa.
    pub informativo: bool,
}

/// Resultado da análise de uma categoria.
#[derive(Default)]
pub struct ScanResult {
    pub bytes: u64,
    pub itens: u64,
    /// Linhas de detalhamento por alvo ("npm — 230 MB").
    pub detalhes: Vec<String>,
    /// true se algum alvo exige root para medir/limpar com precisão.
    pub requer_root: bool,
}

/// Resumo de uma operação de limpeza.
pub struct Resumo {
    pub bytes: u64,
    pub itens: u64,
    pub erros: u64,
    pub lixeira: bool,
    pub linhas: Vec<String>,
}
