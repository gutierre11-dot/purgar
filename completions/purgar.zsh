#compdef purgar

# Completions de zsh para purgar.

_purgar() {
    local -a subcomandos categorias
    subcomandos=(
        'scan:analisa o espaço recuperável por categoria'
        'clean:limpa uma ou mais categorias'
        'lista:lista as categorias disponíveis'
        'config:mostra caminhos e valores da configuração'
        'help:mostra a ajuda'
    )
    categorias=(
        cache_apps
        cache_navegadores
        lixeira
        miniaturas
        cache_dev
        logs_usuario
        downloads
        logs_sistema
        journal_sistema
        cache_sistema
        tmp_antigos
        dnf_cache
        apt_cache
        flatpak_cache
        flatpak_unused
        coredumps
        docker_cache
        vscode_cache
        jetbrains_cache
        custom
    )

    _arguments -C \
        '(-h --help)'{-h,--help}'[mostra a ajuda]' \
        '(-V --version)'{-V,--version}'[mostra a versão]' \
        '1:subcomando:->subcomando' \
        '*::argumento:->argumento'

    case $state in
        subcomando)
            _describe -t subcomandos 'subcomando' subcomandos
            ;;
        argumento)
            case $words[1] in
                scan)
                    _arguments \
                        '--json[emite a saída em JSON]' \
                        '--detalhes[lista o detalhamento por alvo]' \
                        '--categoria[filtra por categoria]:categoria:->categoria'
                    ;;
                clean)
                    _arguments \
                        '--todos-basicos[limpa todas as categorias básicas]' \
                        '--sim[confirma sem perguntar]' \
                        '--yes[confirma sem perguntar]' \
                        '--dry-run[analisa sem apagar]' \
                        '--json[emite o resultado em JSON]' \
                        '--lixeira[força o uso da lixeira]' \
                        '--sem-lixeira[força a remoção definitiva]' \
                        '*:categoria:->categoria'
                    ;;
                lista|config)
                    _arguments '--json[emite a saída em JSON]'
                    ;;
            esac
            ;;
    esac

    if [[ $state == categoria ]]; then
        _values 'categoria' $categorias
    fi
}

_purgar "$@"
