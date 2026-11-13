# Política de segurança do purgar

O purgar apaga arquivos e, em algumas categorias, executa comandos com
privilégios de administrador. Por isso, problemas de segurança são tratados com
prioridade, especialmente qualquer caso que possa causar perda de dados além do
que o usuário selecionou.

## Versões suportadas

| Versão | Suporte |
|---|---|
| 0.2.x | Sim |
| < 0.2 | Não |

Apenas a série 0.2 recebe correções de segurança. Como o projeto ainda está em
fase inicial e não publicou uma 1.0, não há promessa de compatibilidade entre
séries.

## Como reportar uma vulnerabilidade

**Não abra uma issue pública para relatar vulnerabilidades.**

Use o canal privado de
[GitHub Security Advisories](https://github.com/gutierre11-dot/purgar/security/advisories/new)
do repositório. O relato fica visível apenas para os mantenedores até que uma
correção seja publicada e a divulgação seja coordenada.

Inclua, se possível:

- versão do purgar (`purgar --version`) e como ele foi instalado;
- distribuição e versão do Linux, gerenciador de pacotes e versão do Rust;
- passos para reproduzir o problema;
- impacto esperado (por exemplo: remoção de arquivos fora do escopo,
  escalonamento de privilégio, terminal corrompido, execução de comando
  inesperado);
- saída do programa, logs e trechos relevantes do arquivo de configuração;
- se o problema depende de `pkexec`/polkit, lixeira, caminhos personalizados ou
  de uma categoria específica.

Não inclua dados pessoais nem execute limpezas destrutivas reais durante a
investigação; use `--dry-run`, um diretório de teste ou uma máquina virtual.

O projeto não oferece programa de recompensas. Relatos responsáveis são
creditados no changelog e no aviso de segurança, se o autor desejar.

## Modelo de segurança

### O que o purgar remove

- Conteúdo de caches e de arquivos temporários do usuário, dentro das categorias
  documentadas no README e na man page.
- Lixeira, quando a categoria `lixeira` é selecionada.
- Itens de categorias administrativas, somente após autenticação via
  `pkexec`/polkit.
- Caminhos personalizados da categoria `custom`, somente após validação.
- Nada relacionado a `downloads`, que é uma categoria apenas de análise.

### Privilégios

Ações administrativas usam `pkexec`/polkit. O purgar não pede root para
categorias de usuário e não mantém um processo privilegiado. Sem uma sessão
gráfica com agente polkit, a operação falha com mensagem clara; o programa não
contorna a autenticação.

### Lixeira

O modo lixeira segue a
[freedesktop.org Trash spec](https://specifications.freedesktop.org/trash-spec/latest/):
o `Path=` recebe URL-encoding e o arquivo `.trashinfo` é criado antes do move.
Quando o move não é possível (por exemplo, `rename` entre sistemas de arquivos
diferentes), o programa tenta cópia seguida de remoção verificada. Se nada der
certo, a limpeza falha e o usuário é avisado — o purgar não apaga
silenciosamente no modo lixeira.

O modo lixeira não substitui backup: itens podem ser removidos da lixeira por
outros programas, e a lixeira de outros usuários ou de sistemas de arquivos
removíveis pode não estar acessível.

### Symlinks e caminhos

- A varredura interna de diretórios não segue symlinks.
- Diretórios-raiz simbólicos são resolvidos para validação, garantindo que a
  categoria só seja limpa depois de validar o caminho real.
- Caminhos personalizados rejeitam diretórios de sistema, a própria home e
  caminhos rasos demais, e precisam existir.
- A validação de caminhos é aplicada no fluxo de limpeza, não apenas na
  interface.

### Navegadores e confirmação

- A limpeza de cache de navegadores é bloqueada enquanto houver navegador
  aberto, tanto na TUI quanto no CLI.
- O CLI exige confirmação interativa (ou `--sim` explícito em ambiente não
  interativo); a TUI pede confirmação em um modal.
- A TUI restaura o terminal mesmo em caso de erro ou panic, e a tela de
  progresso pode ser cancelada com `Esc`.

### Fora do escopo

O purgar **não**:

- é um sandbox ou um mecanismo de isolamento;
- remove pacotes do sistema fora do desinstalador;
- garante que itens antigos em `/tmp` e `/var/log` pertençam a um serviço
  inativo — por isso essas categorias exigem atenção;
- substitui backup, snapshots do sistema de arquivos ou a lixeira;
- acessa a rede para verificar atualizações ou enviar telemetria.

## Recomendações de uso

- Rode `purgar scan` ou `purgar scan --detalhes` antes de limpar.
- Use `--dry-run` para revisar o que seria removido.
- Mantenha o modo lixeira ativado (`usar_lixeira: true`) durante os primeiros
  usos, ou use `--lixeira` em execuções pontuais.
- Revise a lista de caminhos personalizados antes de salvar a configuração.
- Evite rodar o binário diretamente como root; prefira as categorias que já
  usam `pkexec`/polkit.
- Não execute o purgar com `sudo` sem necessidade; o programa foi feito para
  rodar como usuário e elevar apenas o necessário.
- Mantenha o sistema e o toolchain do Rust atualizados.
- Em ambientes automatizados, prefira `purgar clean ... --sim` apenas depois de
  validar os ids e o comportamento com `--dry-run`.
