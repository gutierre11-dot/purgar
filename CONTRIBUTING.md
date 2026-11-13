# Contribuindo com o purgar

Obrigado pelo interesse em contribuir. O purgar é um utilitário de limpeza para
Linux: uma contribuição de qualidade precisa ser segura, testada e documentada,
porque o programa apaga arquivos de verdade.

Este documento descreve o fluxo de trabalho, os comandos de verificação, o
estilo do código e como adicionar categorias, flags e configurações.

## Antes de começar

- Seja respeitoso e objetivo nas discussões.
- Para mudanças grandes, abra uma issue antes de implementar, descrevendo o
  problema e a proposta.
- Vulnerabilidades **não** devem ser reportadas em issues públicas. Siga o
  [SECURITY.md](SECURITY.md).
- Mudanças que afetam o que é apagado precisam de cuidado extra e de testes.

## Pré-requisitos

- Linux (Fedora/RHEL, Debian/Ubuntu, Arch ou derivados).
- Rust 1.95 ou superior (`rustup` recomendado).
- `git`.
- Opcional: `pkexec`/polkit para testar categorias administrativas e `docker`,
  `flatpak`, `snap`, `code` e IDEs JetBrains para testar as categorias
  condicionais.

## Configuração do ambiente

```bash
git clone https://github.com/gutierre11-dot/purgar.git
cd purgar
cargo build
```

Para rodar a TUI:

```bash
cargo run
```

Para rodar o CLI sem abrir a TUI:

```bash
cargo run -- scan
cargo run -- lista
cargo run -- config
```

## Comandos de verificação

Antes de abrir um pull request, rode:

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release
```

O CI executa exatamente esses passos em cada push e pull request.

Durante o desenvolvimento, `cargo fmt --all` e `cargo clippy --fix` ajudam a
resolver a maior parte dos avisos. Não envie código com warnings do clippy.

## Estilo

- Use `rustfmt` com as configurações padrão do projeto.
- `cargo clippy --all-targets -- -D warnings` precisa passar.
- Mantenha o padrão do código existente: identificadores e mensagens em
  português do Brasil, nomes de categorias e ids em snake_case.
- Não introduza `unsafe` sem uma justificativa muito forte.
- Prefira `Result` e mensagens de erro claras a `unwrap()`/`expect()` em código
  que pode falhar em produção.
- Escreva testes para qualquer rotina que manipule filesystem, configuração,
  caminhos, lixeira ou comandos externos.
- Mensagens de usuário devem ser curtas, em pt-BR e sem emojis.

## Commits

Use mensagens no formato [Conventional Commits](https://www.conventionalcommits.org/):

```text
feat: adiciona categoria de cache do Docker
fix: evita remoção de diretório com conteúdo em /tmp
docs: atualiza a tabela de categorias do README
test: cobre a validação de caminhos personalizados
refactor: separa a limpeza do cálculo de tamanho
ci: adiciona cache do Cargo no job de testes
chore: atualiza dependências
```

Exemplos de escopos úteis: `feat(cli):`, `fix(tui):`, `docs(readme):`.
Mantenha um assunto por commit e referencie issues quando existirem
(`... (#123)`).

## Pull requests

- Descreva o problema, a solução e como testou.
- Inclua testes para mudanças de comportamento.
- Atualize a documentação afetada:
  - `README.md` para comportamento visível;
  - `packaging/purgar.1` para CLI, flags e categorias;
  - `completions/` ao adicionar subcomandos, flags ou ids;
  - `CHANGELOG.md` para mudanças notáveis;
  - `SECURITY.md` se a mudança alterar garantias de segurança.
- Não comite `target/`, arquivos de editor ou artefatos de build.
- Não altere `Cargo.lock` manualmente; rode `cargo update` apenas quando
  necessário e explique o motivo.

## Estrutura de módulos

| Arquivo | Responsabilidade |
|---|---|
| `src/lib.rs` | Biblioteca e módulos públicos usados pelo binário. |
| `src/main.rs` | Ponto de entrada, parsing simples da CLI e despacho dos subcomandos. |
| `src/categories.rs` | Definição das categorias, níveis, alvos e necessidade de administrador. |
| `src/cleaner.rs` | Análise (scan) e limpeza de cada categoria. |
| `src/fsutil.rs` | Medição, remoção segura, lixeira e formatação de tamanhos. |
| `src/seguranca.rs` | Validação de caminhos personalizados e barreiras de segurança. |
| `src/config.rs` | Leitura/gravação da configuração e histórico. |
| `src/model.rs` | Tipos compartilhados e mensagens entre as camadas. |
| `src/tools.rs` | Gerenciador de inicialização e desinstalador de programas. |
| `src/tui/mod.rs` | Estado, eventos e threads de trabalho da TUI. |
| `src/tui/ui.rs` | Desenho das telas, modais e barras de progresso. |

## Como adicionar uma categoria

1. Defina a categoria em `src/categories.rs` com um `id` estável em snake_case,
   um nome em pt-BR, o nível (básico ou avançado) e a indicação de que exige
   administrador.
2. Declare os alvos da categoria (diretórios a esvaziar, arquivos a remover ou
   comandos externos) usando os tipos existentes em `src/model.rs`.
3. Se a categoria for condicional, verifique a existência da ferramenta ou do
   caminho antes de adicioná-la à lista.
4. Implemente a análise e a limpeza em `src/cleaner.rs`, reutilizando as rotinas
   de `src/fsutil.rs` e o mesmo caminho de confirmação e histórico das demais
   categorias.
5. Aplique as regras de segurança: não seguir symlinks na varredura interna,
   respeitar o modo lixeira, validar caminhos personalizados e preservar os
   arquivos protegidos de `/tmp`.
6. Adicione testes para a análise, a limpeza e os casos de recusa.
7. Atualize:
   - a tabela de categorias do `README.md`;
   - a seção `CATEGORIES` da man page `packaging/purgar.1`;
   - a lista de ids das completions (`completions/purgar.bash`,
     `completions/purgar.zsh` e `completions/purgar.fish`);
   - o `CHANGELOG.md`.
8. Se a categoria exige administrador, confirme que o fluxo de `pkexec`/polkit
   continua com mensagem clara em ambiente sem sessão gráfica.

## Como adicionar uma flag ou subcomando

1. Implemente o parsing em `src/main.rs` e a ajuda em pt-BR.
2. Atualize os códigos de saída quando a flag alterar o resultado.
3. Adicione a flag ao `README.md` e à man page.
4. Atualize as três completions.
5. Registre a mudança no `CHANGELOG.md`.

## Como adicionar uma chave de configuração

1. Adicione o campo em `src/config.rs`, com valor padrão e serialização via
   `serde`.
2. Garanta compatibilidade com arquivos existentes: chaves ausentes usam o valor
   padrão.
3. Atualize a documentação (`README.md` e man page) com tipo e padrão.
4. Adicione testes de leitura, gravação e valores padrão.
5. Registre no `CHANGELOG.md`.

## Licença

Ao contribuir, você concorda que sua contribuição será distribuída sob a
licença MIT do projeto.
