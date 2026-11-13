%global crate purgar

Name:           purgar
Version:        0.2.0
Release:        1%{?dist}
Summary:        Limpador de cache e arquivos temporários para Linux com TUI e CLI

License:        MIT
URL:            https://github.com/gutierre11-dot/purgar
Source0:        %{url}/archive/v%{version}/%{name}-%{version}.tar.gz

BuildRequires:  rust-packaging

Requires:       polkit
Recommends:     systemd
Recommends:     dnf
Suggests:       flatpak

%description
purgar é um limpador de cache e arquivos temporários para Linux, escrito em
Rust, com interface de terminal (TUI) e interface de linha de comando (CLI),
totalmente em português do Brasil.

Ele analisa e remove cache de aplicativos e navegadores, lixeira, miniaturas,
caches de desenvolvimento (npm, pip, Cargo, Go, Gradle, Maven e outros), logs
do usuário e do sistema, journal do systemd, temporários antigos, coredumps e
caches de contêineres, além de integrar um desinstalador de programas com
proteção de pacotes essenciais.

As ações administrativas usam pkexec/polkit e o modo lixeira opcional segue a
especificação freedesktop.org Trash. A detecção do gerenciador de pacotes
(dnf, apt, pacman, zypper) e de ferramentas opcionais (flatpak, docker, VS
Code, IDEs JetBrains) habilita apenas as categorias disponíveis no sistema.

%generate_buildrequires
%cargo_generate_buildrequires

%prep
%cargo_prep

%build
%cargo_build

%install
%cargo_install
install -Dpm0644 packaging/%{name}.1 %{buildroot}%{_mandir}/man1/%{name}.1
install -Dpm0644 completions/%{name}.bash %{buildroot}%{_datadir}/bash-completion/completions/%{name}
install -Dpm0644 completions/%{name}.zsh %{buildroot}%{_datadir}/zsh/site-functions/_%{name}
install -Dpm0644 completions/%{name}.fish %{buildroot}%{_datadir}/fish/vendor_completions.d/%{name}.fish

%check
%cargo_test

%files
%license LICENSE
%doc README.md CHANGELOG.md
%{_bindir}/%{crate}
%{_mandir}/man1/%{name}.1*
%{_datadir}/bash-completion/completions/%{name}
%{_datadir}/zsh/site-functions/_%{name}
%{_datadir}/fish/vendor_completions.d/%{name}.fish

%changelog
* Fri Nov 13 2026 Gutierre <gutierre11-dot@users.noreply.github.com> - 0.2.0-1
- Versão inicial empacotada para Fedora.
