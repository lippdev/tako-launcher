# Pacotes do Linux: AUR e Flatpak (PLAT-002)

Arquivos em `packaging/aur/` e `packaging/flatpak/`. O CI (`.github/workflows/packaging.yml`) valida tudo e **não publica nada**: AUR e Flathub exigem a conta do mantenedor.

## AUR (Arch, CachyOS, Manjaro...)

| Pacote | O que faz |
|---|---|
| `tako-launcher-bin` | Baixa `Tako-Linux-x64.tar.gz` (ou `-arm64`) da release do GitHub, confere o sha256, instala `/usr/bin/tako`, o atalho `tako.desktop` (com `x-scheme-handler/tako`, EXT-005), o ícone, a licença e a regra udev do cursor pelo controle. |
| `tako-launcher` | Compila do código-fonte da tag (`cargo`, `cmake`; o SDL3 é compilado e embutido). Os dois se excluem (`conflicts`). |

Decisões:

- `pkgver` usa `_` no lugar do hífen (`0.2.0_alpha.14`); a URL volta ao hífen (`${pkgver//_/-}`).
- Dependências de execução tiradas do binário da release: a tabela de bibliotecas do ELF (DT_NEEDED) mostra só `libfontconfig`, `libgcc_s`, `libm` e `libc`; o SDL3 vai estático e o resto (GL/EGL, X11, Wayland, xkbcommon, Pulse/ALSA, D-Bus, udev) é aberto em tempo de execução. Por isso `depends` traz fontconfig, xkbcommon, libglvnd, wayland e as libs X11; áudio, `xdg-utils`, `kscreen` e `wlr-randr` ficam em `optdepends` (ADR-0024: o Tako só perde a função quando a ferramenta falta).
- A pasta `Tako_Data` do `.tar.gz` liga o modo portátil, então o pacote **não** a instala: os dados ficam em `~/.local/share/tako`.
- O `/usr/bin/tako` faz o atualizador interno só avisar (`Package::LinuxDeb`): quem atualiza é o `pacman`/helper do AUR.
- O `tako-launcher` não tem as credenciais do ScreenScraper (ADR-0022): os metadados dele aparecem como indisponíveis.

### Publicar no AUR (mantenedor)

1. Conta em aur.archlinux.org com a chave SSH cadastrada.
2. Para cada pacote: `git clone ssh://aur@aur.archlinux.org/tako-launcher-bin.git` (idem `tako-launcher`); o primeiro `clone` de um nome livre cria o repositório.
3. Copie o `PKGBUILD` de `packaging/aur/<pacote>/` para o clone.
4. Em Arch: `updpkgsums && makepkg --printsrcinfo > .SRCINFO && makepkg -f` (testa o pacote). `namcap PKGBUILD *.pkg.tar.zst` deve sair sem erro.
5. `git add PKGBUILD .SRCINFO && git commit -m "tako-launcher-bin 0.2.0_alpha.14" && git push`.

A cada release: troque `pkgver`, ponha `pkgrel=1`, rode `updpkgsums` (no `-bin`, o `LICENSE`/ícone da tag também entram) e refaça o `.SRCINFO`. No repositório, mantenha `packaging/aur/` igual ao AUR.

## Flatpak

ID `io.github.lippdev.Tako` (o usado no AUR/Windows/macOS é `dev.lipp.tako`, que no Flathub exigiria provar o domínio `lipp.dev`; o `io.github.<usuário>` é verificado pelo GitHub). **Decisão do mantenedor:** confirme o ID antes de submeter ao Flathub, porque depois não troca.

Compilação local:

```bash
pip install aiohttp tomlkit
curl -fsSLO https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py
python3 flatpak-cargo-generator.py Cargo.lock -o packaging/flatpak/cargo-sources.json   # não versionado
flatpak install flathub org.freedesktop.Platform//25.08 org.freedesktop.Sdk//25.08 org.freedesktop.Sdk.Extension.rust-stable//25.08
flatpak-builder --user --install --force-clean build-dir packaging/flatpak/io.github.lippdev.Tako.yml
flatpak run io.github.lippdev.Tako
```

O `Cargo.toml` pede Rust 1.97; o `rust-stable` do SDK acompanha o estável, então confira a versão (`flatpak run --command=rustc org.freedesktop.Sdk//25.08 --version` com a extensão) antes de submeter.

### Permissões (`finish-args`) e por quê

| Permissão | Motivo |
|---|---|
| `--share=ipc`, `--socket=wayland`, `--socket=fallback-x11` | Janela (Slint/winit). X11 só quando não há Wayland. |
| `--device=all` | O SDL3 lê controles por evdev/hidraw e o cursor pelo controle usa o `/dev/uinput` (ADR-0024). `--device=input` não cobre o `uinput`; se abrir mão dessa função, troque por `--device=dri --device=input`. |
| `--share=network` | Metadados, capas e verificação de atualização. |
| `--socket=pulseaudio` | Saída de áudio e volume (`pactl`, funciona também no PipeWire). |
| `--talk-name=org.freedesktop.secrets` | Chaves de API no cofre do sistema. |
| `--talk-name=org.gnome.Mutter.DisplayConfig` | Trocar a tela no GNOME Wayland. |
| `--filesystem=...:ro` | Só leitura e só as pastas das lojas: Steam (`xdg-data/Steam`, `~/.steam`, e a Steam Flatpak em `~/.var/app/com.valvesoftware.Steam/...`), Heroic e Lutris (nativos e Flatpak), `/mnt`, `/media`, `/run/media` (bibliotecas em outros discos) e `~/Games`. Sem `home` inteira. |

**Não** há `--talk-name=org.freedesktop.Flatpak` (`flatpak-spawn --host`): daria ao app o equivalente a sair da sandbox, e o Flathub só aceita com justificativa forte.

### Limites do Flatpak (consequência de não sair da sandbox)

- Jogos abertos por link da loja (`steam://`, `heroic://`, `lutris:`) funcionam: o `xdg-open` passa pelo portal e abre no host. Executáveis, scripts, `.desktop`, AppImages e emuladores iniciados pelo Tako rodam **dentro** da sandbox e quase sempre não abrem; o Tako também não acompanha o processo de um jogo aberto no host (a pasta `/proc` é da sandbox), só o `RunningAppID` da Steam, lido do `registry.vdf`.
- Tela: só XRandR (X11) e Mutter (GNOME). `kscreen-doctor` e `wlr-randr` não existem na sandbox: nesses ambientes as opções de tela somem (`Unsupported`).
- "Abrir com o sistema" grava no `~/.var/app/<id>/config/autostart`, que o sistema não lê; precisa do portal Background (não implementado).
- Atualizador interno desligado (`/app/bin/tako` conta como pacote do sistema); atualiza pelo Flatpak.
- Cada fonte lê `XDG_*_HOME` da sandbox e também a pasta padrão do usuário (`~/.config/heroic`, `~/.local/share/lutris`), então Heroic e Lutris nativos aparecem. Já a descoberta de emuladores nativos e "Adicionar jogo" por `.desktop` do usuário olham só o `XDG_*_HOME` da sandbox.
- Uma saída para tudo isso é um backend `flatpak-spawn --host` em `tako-platform`: decisão de produto do mantenedor (amplia a permissão); não está nesta entrega.

### O que o linter do Flathub aponta (resultado do `flatpak-builder-lint`)

- `appid-url-not-reachable`: para `io.github.lippdev.<X>` o Flathub espera um repositório `lippdev/<x>`; o repositório se chama `tako-launcher`. Opções: ID `io.github.lippdev.tako_launcher`, ou espelhar/renomear o repositório, ou um ID por domínio próprio. Decisão do mantenedor, antes de submeter.
- `finish-args-flatpak-appdata-folder-*` e `finish-args-unnecessary-xdg-*-ro-access`: o Flathub não aceita, por regra, ler as pastas de outros apps (Steam, Heroic e Lutris). É preciso pedir exceção na submissão, explicando que é um launcher que só lista a biblioteca (somente leitura).
- Runtime: o linter sugere o 26.08; trocar `runtime-version` (e o `rust-stable`) quando for compilar para o Flathub.

### Publicar no Flathub (mantenedor)

1. Conta no GitHub com acesso para abrir PR em `flathub/flathub` (branch `new-pr`), seguindo https://docs.flathub.org/docs/for-app-authors/submission.
2. O repositório do app no Flathub leva uma cópia do manifesto trocando a fonte local por git: no módulo `tako`, `- type: git`, `url: https://github.com/lippdev/tako-launcher.git`, `tag: vX.Y.Z-alpha.N`, `commit: <sha>`, mais o `cargo-sources.json` gerado para esse `Cargo.lock`. Os arquivos `.desktop`, `.metainfo.xml` e o ícone têm de existir na tag.
3. Falta o que o Flathub exige e a árvore não tem: **capturas de tela** hospedadas (`<screenshots>` no metainfo, URL estável), e a verificação do ID.
4. A cada release: nova `<release>` no metainfo, `TAKO_VERSION` no manifesto, nova `tag`/`commit` e regenerar o `cargo-sources.json`.

## CI

`packaging.yml` roda ao mexer em `packaging/aur`, `packaging/flatpak`, no ícone ou no próprio arquivo:

- `aur` (contêiner `archlinux:base-devel`, por pacote): `makepkg --printsrcinfo`, `--verifysource` (confere o sha256 contra o GitHub), `makepkg` do pacote, `namcap`, instala o `.pkg.tar.zst` e confere binário, `.desktop` com `x-scheme-handler/tako`, regra udev e `ldd`.
- `flatpak-lint`: `desktop-file-validate`, `appstreamcli validate` e `flatpak-builder --show-manifest` reprovam; o `flatpak-builder-lint` (regras do Flathub) só informa no resumo do job.
- `flatpak-build`: compila o Flatpak de verdade (demora; sem publicar nem guardar o pacote).

Quando a release mudar (`pkgver`), o `verifysource` quebra de propósito se o sha256 não foi atualizado.
