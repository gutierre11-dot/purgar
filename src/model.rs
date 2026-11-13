use std::path::PathBuf;

/// Nível de risco/experiência de uma categoria.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// Remove arquivos mais antigos que `dias` dias.
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
    /// Apenas mede o caminho (categoria informativa), nunca remove.
    Medir(PathBuf),
}

/// Uma unidade de limpeza exibida ao usuário.
#[derive(Clone, Debug)]
pub struct Categoria {
    pub id: &'static str,
    pub nome: &'static str,
    pub descricao: String,
    pub nivel: Nivel,
    pub alvos: Vec<Alvo>,
    /// Nomes de processos: se algum estiver rodando, a TUI exibe aviso.
    pub avisos_proc: &'static [&'static str],
    /// Nomes de processos: se algum estiver rodando, a limpeza fica bloqueada.
    pub bloqueio_proc: &'static [&'static str],
    /// Categoria apenas informativa (ex.: Downloads) — não limpa.
    pub informativo: bool,
}

impl Categoria {
    /// `true` se a categoria possui pelo menos uma ação que exige root.
    pub fn requer_admin(&self) -> bool {
        self.alvos
            .iter()
            .any(|a| matches!(a, Alvo::ComandoRoot(_) | Alvo::Antigos { sudo: true, .. }))
    }

    /// `true` se a categoria não remove nada (apenas mede).
    pub fn apenas_informativa(&self) -> bool {
        self.informativo
    }
}

/// Resultado da análise de uma categoria.
#[derive(Clone, Debug, Default)]
pub struct ScanResult {
    pub bytes: u64,
    pub itens: u64,
    /// Linhas de detalhamento por alvo ("npm — 230 MB").
    pub detalhes: Vec<String>,
    /// true se algum alvo exige root para medir/limpar com precisão.
    pub requer_root: bool,
    /// Avisos de medição (diretórios inacessíveis, etc.).
    pub avisos: Vec<String>,
}

/// Resumo de uma operação de limpeza.
#[derive(Clone, Debug, Default)]
pub struct Resumo {
    /// Bytes realmente liberados no disco.
    pub bytes: u64,
    /// Bytes movidos para a lixeira (não liberam espaço até esvaziá-la).
    pub movidos: u64,
    pub itens: u64,
    pub erros: u64,
    pub lixeira: bool,
    pub cancelado: bool,
    pub linhas: Vec<String>,
}
