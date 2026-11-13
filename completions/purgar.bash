# Completions de bash para purgar.

_purgar() {
    local cur prev words cword
    _init_completion || return

    local subcomandos="scan clean lista config help"
    local opcoes_globais="--help -h --version -V"
    local opcoes_scan="--json --detalhes --categoria"
    local opcoes_clean="--todos-basicos --sim --yes --dry-run --json --lixeira --sem-lixeira"
    local categorias="cache_apps cache_navegadores lixeira miniaturas cache_dev logs_usuario downloads logs_sistema journal_sistema cache_sistema tmp_antigos dnf_cache apt_cache flatpak_cache flatpak_unused coredumps docker_cache vscode_cache jetbrains_cache custom"

    if (( cword == 1 )); then
        COMPREPLY=( $(compgen -W "$subcomandos $opcoes_globais" -- "$cur") )
        return
    fi

    local subcomando="${words[1]}"

    case "$subcomando" in
        scan)
            if [[ $prev == "--categoria" ]]; then
                COMPREPLY=( $(compgen -W "$categorias" -- "$cur") )
            else
                COMPREPLY=( $(compgen -W "$opcoes_scan" -- "$cur") )
            fi
            ;;
        clean)
            if [[ $cur == -* ]]; then
                COMPREPLY=( $(compgen -W "$opcoes_clean" -- "$cur") )
            else
                COMPREPLY=( $(compgen -W "$categorias" -- "$cur") )
            fi
            ;;
        lista|config)
            COMPREPLY=( $(compgen -W "--json $opcoes_globais" -- "$cur") )
            ;;
        *)
            COMPREPLY=( $(compgen -W "$subcomandos $opcoes_globais" -- "$cur") )
            ;;
    esac
}

complete -F _purgar purgar
