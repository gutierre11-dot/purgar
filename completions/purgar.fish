# Completions de fish para purgar.

# Sem completions de arquivo por padrão.
complete -c purgar -f

# Subcomandos.
complete -c purgar -n '__fish_use_subcommand' -a scan -d 'Analisa o espaço recuperável por categoria'
complete -c purgar -n '__fish_use_subcommand' -a clean -d 'Limpa uma ou mais categorias'
complete -c purgar -n '__fish_use_subcommand' -a lista -d 'Lista as categorias disponíveis'
complete -c purgar -n '__fish_use_subcommand' -a config -d 'Mostra caminhos e valores da configuração'
complete -c purgar -n '__fish_use_subcommand' -a help -d 'Mostra a ajuda'

# Opções globais.
complete -c purgar -s h -l help -d 'Mostra a ajuda'
complete -c purgar -s V -l version -d 'Mostra a versão'

# purgar scan.
complete -c purgar -n '__fish_seen_subcommand_from scan' -l json -d 'Emite a saída em JSON'
complete -c purgar -n '__fish_seen_subcommand_from scan' -l detalhes -d 'Lista o detalhamento por alvo'
complete -c purgar -n '__fish_seen_subcommand_from scan' -l categoria -x -a 'cache_apps cache_navegadores lixeira miniaturas cache_dev logs_usuario downloads logs_sistema journal_sistema cache_sistema tmp_antigos dnf_cache apt_cache flatpak_cache flatpak_unused coredumps docker_cache vscode_cache jetbrains_cache custom' -d 'Filtra por categoria'

# purgar clean.
complete -c purgar -n '__fish_seen_subcommand_from clean' -l todos-basicos -d 'Limpa todas as categorias básicas'
complete -c purgar -n '__fish_seen_subcommand_from clean' -l sim -d 'Confirma sem perguntar'
complete -c purgar -n '__fish_seen_subcommand_from clean' -l yes -d 'Confirma sem perguntar'
complete -c purgar -n '__fish_seen_subcommand_from clean' -l dry-run -d 'Analisa sem apagar'
complete -c purgar -n '__fish_seen_subcommand_from clean' -l json -d 'Emite o resultado em JSON'
complete -c purgar -n '__fish_seen_subcommand_from clean' -l lixeira -d 'Força o uso da lixeira'
complete -c purgar -n '__fish_seen_subcommand_from clean' -l sem-lixeira -d 'Força a remoção definitiva'
complete -c purgar -n '__fish_seen_subcommand_from clean' -a 'cache_apps cache_navegadores lixeira miniaturas cache_dev logs_usuario downloads logs_sistema journal_sistema cache_sistema tmp_antigos dnf_cache apt_cache flatpak_cache flatpak_unused coredumps docker_cache vscode_cache jetbrains_cache custom' -d 'Categoria'

# purgar lista e purgar config.
complete -c purgar -n '__fish_seen_subcommand_from lista config' -l json -d 'Emite a saída em JSON'
