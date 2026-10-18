//! Assistente da primeira execução (UX-003): idioma, lojas e provedores de metadados (com o driver opcional do
//! INPUT-005 no Windows), em três diálogos.
//! Aparece uma vez numa instalação nova; tudo dá para mudar depois em Configurações.

use super::ctx::keys;
use super::layer::{Flow, MenuItem, MenuLayer};
use super::settings::{library, metadata};
use super::{Ctx, Shell};
use crate::model::PanelRow;
use tako_core::i18n::{self, Lang};
use tako_core::library::Game;
use tako_sources::emulators::emulator::Emulator;
use tako_sources::emulators::{Installed, systems};

const STEPS: usize = 3;
/// No desktop o onboarding é em tela inteira e tem mais dois passos: Modo (abrir em janela ou em tela cheia) e Pronto.
const DESKTOP_STEPS: usize = 5;

/// Instalação nova: nunca concluiu o assistente, nada gravado nas configurações e biblioteca vazia. Quem atualiza
/// de uma alpha não vê o assistente; marca como concluído para ele não aparecer depois.
fn needed(ctx: &mut Ctx) -> bool {
    if ctx.settings.bool(keys::FIRST_RUN_DONE, false) {
        return false;
    }
    if !ctx.settings.is_empty() || !ctx.library.games().is_empty() {
        ctx.settings.set_bool(keys::FIRST_RUN_DONE, true);
        return false;
    }
    true
}

/// Abre o assistente se for a primeira vez (fora do `--simulate`, do modo seguro e da volta de um jogo).
pub fn open_if_needed(shell: &mut Shell) {
    if shell.ctx.simulated() || shell.ctx.safe_mode || shell.ctx.running.is_some() {
        return;
    }
    if needed(&mut shell.ctx) {
        let layer = welcome(&shell.ctx);
        shell.push_layer(Box::new(layer));
    }
}

fn finish(ctx: &mut Ctx) -> Flow {
    ctx.settings.set_bool(keys::FIRST_RUN_DONE, true);
    ctx.toast(ctx.t("first_run.done_toast"));
    Flow::Close
}

fn step(
    ctx: &Ctx,
    n: usize,
    title: &str,
    text: &str,
    build: impl Fn(&Ctx) -> Vec<MenuItem> + 'static,
) -> MenuLayer {
    if ctx.desktop {
        // TelaOnboarding: "n de 5", título grande e a nota de rodapé de cada passo.
        let total = DESKTOP_STEPS.to_string();
        // "Pular configuração" em todo passo (fica no topo da tela).
        let build = move |ctx: &Ctx| {
            let mut v: Vec<MenuItem> = build(ctx)
                .into_iter()
                .filter(|i| i.row.key != "skip")
                .collect();
            v.push(
                MenuItem::new(PanelRow::button("skip", ctx.t("first_run.full.skip"))).on(finish),
            );
            v
        };
        return MenuLayer::dialog(ctx.t(&format!("first_run.full.{title}")), build)
            .overline(ctx.tf(
                "first_run.count",
                &[("n", &n.to_string()), ("total", &total)],
            ))
            .text(ctx.t(&format!("first_run.full.{text}")))
            .steps((1..=DESKTOP_STEPS).map(|i| i.to_string()).collect(), n - 1)
            .full(ctx.t(&format!("first_run.full.{title}_note")));
    }
    MenuLayer::dialog(ctx.t(&format!("first_run.{title}")), build)
        .overline(ctx.tf(
            "first_run.step",
            &[("n", &n.to_string()), ("total", &STEPS.to_string())],
        ))
        .text(ctx.t(&format!("first_run.{text}")))
        .steps(
            vec![
                ctx.t("first_run.welcome_short"),
                ctx.t("first_run.stores"),
                ctx.t("first_run.metadata"),
            ],
            n - 1,
        )
        .wide()
}

const WALL: usize = 15;

/// Jogo da comparação do passo das capas: o primeiro da parede que já tem capa (posição na parede).
fn compare_game(ctx: &Ctx) -> Option<usize> {
    ctx.library
        .games()
        .iter()
        .take(WALL)
        .position(|g| g.shown_cover().is_some())
}

/// Capas da parede ao fundo do onboarding: os jogos que já estão na biblioteca (ambiente, sem foco nem clique).
pub fn wall(ctx: &Ctx) -> Vec<crate::model::Tile> {
    ctx.library
        .games()
        .iter()
        .take(WALL)
        .map(|g| crate::model::Tile {
            id: g.id.clone(),
            title: g.title.clone(),
            cover: super::tiles::image(g.shown_cover(), super::tiles::COVER),
            accent: crate::values::parse_color(&g.accent),
            installed: g.is_installed,
            ..Default::default()
        })
        .collect()
}

/// Passo 1: idioma. O diálogo é refeito ao trocar, para o próprio texto mudar de idioma. No desktop o onboarding em
/// tela inteira começa pelas lojas (uma pergunta por tela; o idioma segue o sistema e fica em Configurações › Geral).
pub fn welcome(ctx: &Ctx) -> MenuLayer {
    if ctx.desktop {
        return stores(ctx);
    }
    step(ctx, 1, "welcome", "welcome_text", |ctx| {
        let mut v: Vec<MenuItem> = [Lang::PtBr, Lang::EnUs]
            .into_iter()
            .enumerate()
            .map(|(i, lang)| {
                MenuItem::new(PanelRow::radio(
                    lang.code(),
                    super::settings::lang_name(lang),
                    ctx.lang() == lang,
                ))
                .on(move |ctx| {
                    ctx.settings.set(keys::LANGUAGE, lang.code());
                    i18n::set_lang(lang);
                    ctx.dirty = true;
                    Flow::Replace(Box::new(welcome(ctx).focus_at(i)))
                })
            })
            .collect();
        v.push(
            MenuItem::new(PanelRow::button("next", ctx.t("first_run.next")))
                .on(|ctx| Flow::Replace(Box::new(stores(ctx)))),
        );
        v.push(MenuItem::new(PanelRow::button("skip", ctx.t("first_run.skip"))).on(finish));
        v
    })
    .focus_at(2)
    .on_back(finish)
}

/// Passo 2 (no desktop, 1): lojas que este sistema lê, com o que a leitura achou em cada uma.
fn stores(ctx: &Ctx) -> MenuLayer {
    if ctx.desktop {
        return desktop_stores(ctx);
    }
    let n = if ctx.desktop { 1 } else { 2 };
    let layer = step(ctx, n, "stores", "stores_text", |ctx| {
        let mut v = library::store_rows(ctx);
        v.push(
            MenuItem::new(PanelRow::button("next", ctx.t("first_run.next"))).on(|ctx| {
                // Desktop: emuladores só quando há algum; senão direto para as capas.
                if ctx.desktop && super::settings::emulators::anything_found(ctx) {
                    Flow::Replace(Box::new(emulators(ctx)))
                } else {
                    Flow::Replace(Box::new(providers(ctx)))
                }
            }),
        );
        v
    })
    // Foco em "Continuar": A seguido não desliga a primeira loja sem querer.
    .focus_at(library::store_rows(ctx).len());
    if ctx.desktop {
        layer.on_back(finish)
    } else {
        layer.on_back(|ctx| Flow::Replace(Box::new(welcome(ctx))))
    }
}

/// Passo 1 do desktop (TelaOnboarding, LojaEncontrada): as lojas com o que o Tako achou nelas (ponto verde quando
/// achou, "Não instalado" apagado), **Procurar jogos** (procura e segue) e **Escolher pastas…** (liga e desliga lojas).
fn desktop_stores(ctx: &Ctx) -> MenuLayer {
    step(ctx, 1, "stores", "stores_text", |ctx| {
        let mut v: Vec<MenuItem> = library::stores()
            .iter()
            .map(|&store| {
                let report = ctx.sources.iter().find(|r| r.store == store && r.enabled);
                let found = report.is_some_and(|r| r.client_installed);
                let desc = match report {
                    Some(r) if r.client_installed => r.summary(),
                    Some(_) => ctx.t("first_run.full.stores_missing"),
                    None => ctx.t("first_run.full.stores_unchecked"),
                };
                let mut row = PanelRow::info(
                    format!("store:{}", store.key()),
                    tako_core::presentation::game_text::store_name(ctx.lang(), store),
                    "",
                )
                .desc(desc)
                .icon(super::sidebar::store_icon(store))
                .enabled(report.is_none_or(|r| r.client_installed))
                .emphasized(true);
                row.on = found;
                MenuItem::new(row)
            })
            .collect();
        v.push(
            MenuItem::new(PanelRow {
                style: "primary".into(),
                ..PanelRow::button("search", ctx.t("first_run.full.stores_search"))
            })
            .on(|ctx| {
                ctx.effect(super::Effect::Rescan);
                if super::settings::emulators::anything_found(ctx) {
                    Flow::Replace(Box::new(emulators(ctx)))
                } else {
                    Flow::Replace(Box::new(providers(ctx)))
                }
            }),
        );
        v.push(
            MenuItem::new(PanelRow::button(
                "choose",
                ctx.t("first_run.full.stores_choose"),
            ))
            .on(|ctx| {
                let layer = MenuLayer::dialog(ctx.t("settings.library.stores_header"), |ctx| {
                    let mut v = library::store_rows(ctx);
                    v.push(
                        MenuItem::new(PanelRow::button("done", ctx.t("common.done")))
                            .on(|_| Flow::Close),
                    );
                    v
                });
                Flow::Push(Box::new(layer))
            }),
        );
        v
    })
    .focus_at(library::stores().len())
    .on_back(finish)
}

/// Jogos de cada emulador achado: vale o executável que a leitura gravou no jogo (o que de fato abre a ROM).
fn by_emulator<'a>(games: &'a [Game], found: &Installed) -> Vec<(Emulator, Vec<&'a Game>)> {
    found
        .programs
        .iter()
        .map(|(&e, p)| {
            let target = p.target.to_string_lossy();
            let mine = games
                .iter()
                .filter(|g| g.is_rom())
                .filter(|g| match &p.flatpak {
                    // Flatpak: o executável é o mesmo para todos; o id vai nos argumentos.
                    Some(id) => g
                        .launch_arguments
                        .as_deref()
                        .is_some_and(|a| a.contains(id)),
                    None => g.launch_target.as_deref() == Some(&*target),
                })
                .collect();
            (e, mine)
        })
        .collect()
}

/// Nome do sistema da ROM ("Super Nintendo"), pela chave gravada no jogo.
fn system_name(g: &Game) -> Option<&'static str> {
    let p = g.platform.as_deref()?;
    systems::SYSTEMS
        .iter()
        .find(|s| systems::platform_key(s.id) == p)
        .map(|s| s.name)
}

/// Logo oficial do emulador (os que o tema tem) ou as iniciais num selo.
fn emulator_icon(e: Emulator) -> String {
    format!("emu-{}", e.key())
}

/// Passo 2 (desktop, OnboardingEmuladores): os emuladores deste PC com os sistemas e quantos jogos cada um abre, a
/// pasta de ROMs, **Continuar**, **Adicionar emulador…** e "Não uso emuladores".
pub(crate) fn emulators(ctx: &Ctx) -> MenuLayer {
    use super::settings::emulators as emu;
    let cache = emu::Cache::default();
    step(ctx, 2, "emulators", "emulators_text", move |ctx| {
        let lang = ctx.lang();
        let found = emu::installed(&cache, ctx);
        let games = ctx.library.games();
        let mut v: Vec<MenuItem> = by_emulator(games, &found)
            .into_iter()
            .map(|(e, mine)| {
                let mut names: Vec<&str> = mine.iter().filter_map(|g| system_name(g)).collect();
                names.sort_unstable();
                names.dedup();
                let desc = if mine.is_empty() {
                    ctx.t("first_run.full.emu_no_games")
                } else {
                    let count = tako_core::presentation::library_text::game_count(lang, mine.len());
                    if names.is_empty() {
                        count
                    } else {
                        format!("{} · {count}", names.join(", "))
                    }
                };
                let mut row = PanelRow::info(format!("emu:{}", e.key()), e.name(), "")
                    .desc(desc)
                    .icon(&emulator_icon(e))
                    .emphasized(true);
                row.on = true;
                MenuItem::new(row)
            })
            .collect();
        // Os não instalados completam até quatro linhas, apagados: mostram o que o Tako sabe abrir.
        let missing = Emulator::ALL
            .into_iter()
            .filter(|e| !found.programs.contains_key(e))
            .take(4usize.saturating_sub(v.len()));
        v.extend(missing.map(|e| {
            MenuItem::new(
                PanelRow::info(format!("emu:{}", e.key()), e.name(), "")
                    .desc(ctx.t("first_run.full.stores_missing"))
                    .icon(&emulator_icon(e))
                    .enabled(false)
                    .emphasized(true),
            )
        }));
        // Pasta de ROMs em linha própria, com quantos jogos e sistemas achou.
        let folders = emu::config(ctx).rom_folders;
        for (i, f) in folders.iter().enumerate() {
            let inside: Vec<&Game> = games
                .iter()
                .filter(|g| {
                    g.rom_path
                        .as_deref()
                        .is_some_and(|p| std::path::Path::new(p).starts_with(&f.path))
                })
                .collect();
            let mut names: Vec<&str> = inside.iter().filter_map(|g| system_name(g)).collect();
            names.sort_unstable();
            names.dedup();
            let desc = format!(
                "{} · {}",
                f.path.to_string_lossy(),
                ctx.tf(
                    "first_run.full.roms_found",
                    &[
                        (
                            "games",
                            &tako_core::presentation::library_text::game_count(lang, inside.len()),
                        ),
                        (
                            "systems",
                            &i18n::tn("first_run.full.systems", names.len() as i64, &[]),
                        ),
                    ],
                ),
            );
            v.push(
                MenuItem::new(
                    PanelRow {
                        style: "card".into(),
                        ..PanelRow::action(format!("roms:{i}"), ctx.t("first_run.full.roms"))
                    }
                    .desc(desc)
                    .icon("folder")
                    .value(ctx.t("first_run.full.change")),
                )
                .on(move |ctx| {
                    emu::replace_folder(ctx, i);
                    Flow::Stay
                }),
            );
        }
        if folders.is_empty() {
            v.push(
                MenuItem::new(
                    PanelRow {
                        style: "card".into(),
                        ..PanelRow::action("roms", ctx.t("first_run.full.roms"))
                    }
                    .desc(ctx.t("first_run.full.roms_none"))
                    .icon("folder")
                    .value(ctx.t("first_run.full.roms_choose")),
                )
                .on(|ctx| {
                    emu::add_folder(ctx);
                    Flow::Stay
                }),
            );
        }
        v.push(
            MenuItem::new(PanelRow::button("next", ctx.t("first_run.next")))
                .on(|ctx| Flow::Replace(Box::new(providers(ctx)))),
        );
        v.push(
            MenuItem::new(PanelRow::button(
                "add-emulator",
                ctx.t("first_run.full.add_emulator"),
            ))
            .on(|ctx| Flow::Push(Box::new(emu::panel(ctx)))),
        );
        v.push(
            MenuItem::new(PanelRow {
                style: "link".into(),
                ..PanelRow::button("no-emulators", ctx.t("first_run.full.no_emulators"))
            })
            .on(|ctx| Flow::Replace(Box::new(providers(ctx)))),
        );
        v
    })
    .on_back(|ctx| Flow::Replace(Box::new(stores(ctx))))
}

/// Passo 3: provedores de capas e descrições (os mesmos painéis de Configurações) e, no Windows, o driver opcional
/// que esconde o controle do jogo com a Central aberta (INPUT-005).
fn providers_rows(ctx: &Ctx) -> Vec<MenuItem> {
    let mut v = metadata::provider_rows(ctx);
    v.extend(super::settings::controls::hide_pads_row(ctx));
    v
}

fn providers(ctx: &Ctx) -> MenuLayer {
    if ctx.desktop {
        return covers(ctx);
    }
    step(ctx, 3, "metadata", "metadata_text", |ctx| {
        let mut v = providers_rows(ctx);
        v.push(MenuItem::new(PanelRow::button("done", ctx.t("first_run.finish"))).on(finish));
        v
    })
    .focus_at(providers_rows(ctx).len())
    .on_back(|ctx| Flow::Replace(Box::new(stores(ctx))))
}

/// Voltar das capas: aos emuladores quando o passo existe; senão às lojas.
fn before_covers(ctx: &mut Ctx) -> Flow {
    if super::settings::emulators::anything_found(ctx) {
        Flow::Replace(Box::new(emulators(ctx)))
    } else {
        Flow::Replace(Box::new(stores(ctx)))
    }
}

/// Internet para a IGDB: 0 = ainda não sabe, 1 = com conexão, 2 = sem.
static ONLINE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// Confere uma vez, numa thread, se a IGDB responde (OnboardingCapas avisa quando não).
fn check_online() {
    use std::sync::atomic::Ordering;
    if ONLINE
        .compare_exchange(0, 3, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    let _ = std::thread::Builder::new()
        .name("tako-online".into())
        .spawn(|| {
            use std::net::ToSocketAddrs;
            let ok = ("api.igdb.com", 443)
                .to_socket_addrs()
                .ok()
                .and_then(|mut a| a.next())
                .is_some_and(|a| {
                    std::net::TcpStream::connect_timeout(&a, std::time::Duration::from_secs(4))
                        .is_ok()
                });
            ONLINE.store(if ok { 1 } else { 2 }, Ordering::SeqCst);
            let _ = slint::invoke_from_event_loop(|| {
                if let Some(shell) = crate::tasks::shell() {
                    shell.borrow_mut().ctx.dirty = true;
                }
            });
        });
}

/// Liga ou desliga a IGDB (a escolha do passo das capas).
fn choose_igdb(ctx: &mut Ctx, on: bool) -> Flow {
    metadata::update(ctx, |p| p.igdb.enabled = on);
    // Sem a chave da Twitch a IGDB não responde: abre o painel para colar.
    if on && !metadata::providers(ctx).igdb.has_key() {
        return Flow::Push(Box::new(metadata::igdb_panel(ctx)));
    }
    Flow::Stay
}

/// Passo 3 (desktop, OnboardingCapas): a capa de um jogo de verdade sem arte e com arte, e as duas opções
/// (EscolhaDoOnboarding em linha): buscar na IGDB ou usar só o que as lojas têm. ←→ trocam.
pub(crate) fn covers(ctx: &Ctx) -> MenuLayer {
    check_online();
    step(ctx, 3, "metadata", "metadata_text", |ctx| {
        let igdb = metadata::providers(ctx).igdb;
        let mut v = Vec::new();
        // Comparação: o primeiro jogo da parede que já tem capa (índice na parede, para o tema achar a cor e a capa).
        if let Some(i) = compare_game(ctx) {
            v.push(MenuItem::new(
                PanelRow::new("compare", "compare", ctx.t("first_run.full.cover_none"))
                    .desc(ctx.t("first_run.full.cover_with"))
                    .value(i.to_string()),
            ));
        }
        let games = ctx.library.games().len();
        let mut igdb_desc = if igdb.has_key() {
            ctx.tf(
                "first_run.full.igdb_desc",
                &[(
                    "games",
                    &tako_core::presentation::library_text::game_count(ctx.lang(), games),
                )],
            )
        } else {
            ctx.t("first_run.full.igdb_key_desc")
        };
        if igdb.enabled && ONLINE.load(std::sync::atomic::Ordering::SeqCst) == 2 {
            igdb_desc = ctx.t("first_run.full.igdb_offline");
        }
        let toggle = |ctx: &mut Ctx, _: i32| {
            let on = !metadata::providers(ctx).igdb.enabled;
            choose_igdb(ctx, on)
        };
        v.push(
            MenuItem::new(
                PanelRow::radio("igdb", ctx.t("first_run.full.igdb"), igdb.enabled).desc(igdb_desc),
            )
            .on(|ctx| choose_igdb(ctx, true))
            .on_adjust(toggle),
        );
        v.push(
            MenuItem::new(
                PanelRow::radio(
                    "stores-only",
                    ctx.t("first_run.full.stores_only"),
                    !igdb.enabled,
                )
                .desc(ctx.t("first_run.full.stores_only_desc")),
            )
            .on(|ctx| choose_igdb(ctx, false))
            .on_adjust(toggle),
        );
        v.push(
            MenuItem::new(PanelRow::button("next", ctx.t("first_run.next")))
                .on(|ctx| Flow::Replace(Box::new(mode(ctx))))
                .on_adjust(toggle),
        );
        v.push(
            MenuItem::new(PanelRow::button("back", ctx.t("first_run.full.back"))).on(before_covers),
        );
        v
    })
    // Foco em "Continuar" (depois da comparação, se houver, e das duas opções).
    .focus_at(usize::from(compare_game(ctx).is_some()) + 2)
    .on_back(before_covers)
}

/// Passo 4 (desktop): abrir o Tako em janela ou em tela cheia. Já vem marcado pelo que está em uso: com um controle
/// conectado, tela cheia.
pub(crate) fn mode(ctx: &Ctx) -> MenuLayer {
    use crate::mode::{OPEN_IN, OPEN_IN_BIG_PICTURE, OPEN_IN_DESKTOP};
    step(ctx, 4, "mode", "mode_text", |ctx| {
        let current = ctx
            .settings
            .get(OPEN_IN)
            .map(str::to_owned)
            .unwrap_or_else(|| {
                if ctx.pads_connected > 0 {
                    OPEN_IN_BIG_PICTURE.into()
                } else {
                    OPEN_IN_DESKTOP.into()
                }
            });
        // ←→ trocam a opção (também com o foco em Continuar); Enter continua.
        let shown = current.clone();
        let swap = move |ctx: &mut Ctx, _: i32| {
            let now = ctx
                .settings
                .get(OPEN_IN)
                .map_or_else(|| shown.clone(), str::to_owned);
            let other = if now == OPEN_IN_DESKTOP {
                OPEN_IN_BIG_PICTURE
            } else {
                OPEN_IN_DESKTOP
            };
            ctx.settings.set(OPEN_IN, other);
            Flow::Stay
        };
        let mut v: Vec<MenuItem> = [(OPEN_IN_DESKTOP, "window"), (OPEN_IN_BIG_PICTURE, "full")]
            .into_iter()
            .map(|(value, key)| {
                MenuItem::new(
                    PanelRow::radio(
                        value,
                        ctx.t(&format!("first_run.full.mode_{key}")),
                        current == value,
                    )
                    .desc(ctx.t(&format!("first_run.full.mode_{key}_desc")))
                    .value(ctx.t(&format!("first_run.full.mode_{key}_hint")))
                    .icon(if key == "window" { "monitor" } else { "tv" }),
                )
                .on(move |ctx| {
                    ctx.settings.set(OPEN_IN, value);
                    Flow::Stay
                })
                .on_adjust(swap.clone())
            })
            .collect();
        v.push(
            MenuItem::new(PanelRow::button("next", ctx.t("first_run.next")))
                .on(move |ctx| {
                    if ctx.settings.get(OPEN_IN).is_none() {
                        let value = current.clone();
                        ctx.settings.set(OPEN_IN, &value);
                    }
                    Flow::Replace(Box::new(ready(ctx)))
                })
                .on_adjust(swap),
        );
        v.push(
            MenuItem::new(PanelRow::button("back", ctx.t("first_run.full.back")))
                .on(|ctx| Flow::Replace(Box::new(providers(ctx)))),
        );
        v
    })
    .focus_at(2)
    .on_back(|ctx| Flow::Replace(Box::new(providers(ctx))))
}

/// Linhas do resumo (LojaEncontrada, variante resumo): cada loja e cada emulador com quantos jogos trouxe.
fn summary_rows(ctx: &Ctx, found: &Installed) -> Vec<MenuItem> {
    use tako_core::library::GameStore;
    use tako_core::presentation::{game_text, library_text};
    let lang = ctx.lang();
    let games = ctx.library.games();
    let row = |key: String, title: String, icon: &str, n: usize| {
        MenuItem::new(
            PanelRow::info(key, title, library_text::game_count(lang, n))
                .icon(icon)
                // O número de verdade em destaque.
                .emphasized(true),
        )
    };
    let mut v = Vec::new();
    for store in GameStore::ALL {
        let n = games.iter().filter(|g| g.store() == store).count();
        if n == 0 {
            continue;
        }
        if store != GameStore::Emulators {
            let name = game_text::store_name(lang, store);
            v.push(row(
                format!("store:{}", store.key()),
                name,
                super::sidebar::store_icon(store),
                n,
            ));
            continue;
        }
        // Emuladores: um por um, com o logo; o que nenhum emulador achado reconhece fica numa linha só.
        let mut counted = 0;
        for (e, mine) in by_emulator(games, found) {
            if !mine.is_empty() {
                counted += mine.len();
                v.push(row(
                    format!("emu:{}", e.key()),
                    e.name().into(),
                    &emulator_icon(e),
                    mine.len(),
                ));
            }
        }
        if n > counted {
            v.push(row(
                "store:emulators".into(),
                game_text::store_name(lang, store),
                super::sidebar::store_icon(store),
                n - counted,
            ));
        }
    }
    v
}

/// Passo 5 (desktop, OnboardingPronto): o número de verdade, lojas e emuladores com a quantidade de cada um, a
/// escolha do passo do modo ("Abre em janela" · Trocar) e o caminho para a biblioteca.
pub(crate) fn ready(ctx: &Ctx) -> MenuLayer {
    let total = ctx.library.games().len();
    let title = if total == 0 {
        ctx.t("first_run.full.ready_none")
    } else {
        ctx.tf("first_run.full.ready", &[("n", &total.to_string())])
    };
    let cache = super::settings::emulators::Cache::default();
    let found = super::settings::emulators::installed(&cache, ctx);
    let summary = summary_rows(ctx, &found).len();
    MenuLayer::dialog(title, move |ctx| {
        let found = super::settings::emulators::installed(&cache, ctx);
        let mut v = summary_rows(ctx, &found);
        let big = ctx.settings.get(crate::mode::OPEN_IN) == Some(crate::mode::OPEN_IN_BIG_PICTURE);
        // Linha da escolha: "Abre em **janela**" e o link "Trocar" (volta ao passo do modo).
        v.push(
            MenuItem::new(
                PanelRow {
                    style: "line".into(),
                    ..PanelRow::button("change-mode", ctx.t("first_run.full.opens_in"))
                }
                .value(ctx.t(if big {
                    "first_run.full.opens_full"
                } else {
                    "first_run.full.opens_window"
                }))
                .desc(ctx.t("first_run.full.change"))
                .icon(if big { "tv" } else { "pointer" }),
            )
            .on(|ctx| Flow::Replace(Box::new(mode(ctx)))),
        );
        if ctx.library.games().is_empty() {
            // Nenhum jogo: adicionar à mão é a ação principal; "Procurar de novo" refaz a busca nas lojas.
            v.push(
                MenuItem::new(PanelRow {
                    style: "primary".into(),
                    ..PanelRow::button("add", ctx.t("first_run.full.add_game"))
                })
                .on(|ctx| {
                    let flow = finish(ctx);
                    ctx.effect(super::Effect::AddGame);
                    flow
                }),
            );
            v.push(
                MenuItem::new(PanelRow::button(
                    "rescan",
                    ctx.t("first_run.full.search_again"),
                ))
                .on(|ctx| {
                    ctx.effect(super::Effect::Rescan);
                    Flow::Stay
                }),
            );
            v.push(
                MenuItem::new(PanelRow {
                    style: "link".into(),
                    ..PanelRow::button("open", ctx.t("first_run.full.open_library"))
                })
                .on(finish),
            );
        } else {
            v.push(
                MenuItem::new(PanelRow::button(
                    "done",
                    ctx.t("first_run.full.open_library"),
                ))
                .on(finish),
            );
            // "Revisar capas": termina o onboarding e abre a FilaDeCapas.
            v.push(
                MenuItem::new(PanelRow {
                    style: "link".into(),
                    ..PanelRow::button("review", ctx.t("first_run.full.review_covers"))
                })
                .on(|ctx| {
                    let _ = finish(ctx);
                    Flow::Replace(Box::new(super::art_review::panel(ctx)))
                }),
            );
        }
        v
    })
    .overline(ctx.tf(
        "first_run.count",
        &[
            ("n", &DESKTOP_STEPS.to_string()),
            ("total", &DESKTOP_STEPS.to_string()),
        ],
    ))
    .text(ctx.t(if total == 0 {
        "first_run.full.ready_none_text"
    } else {
        "first_run.full.ready_text"
    }))
    .steps(
        (1..=DESKTOP_STEPS).map(|i| i.to_string()).collect(),
        DESKTOP_STEPS - 1,
    )
    .full(ctx.t("first_run.full.ready_note"))
    // Foco na ação principal: depois do resumo e da linha do modo.
    .focus_at(summary + 1)
    .on_back(|ctx| Flow::Replace(Box::new(mode(ctx))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::{Cmd, Layer};
    use tako_core::paths::DataPaths;
    use tako_platform::Platform;

    fn fresh() -> (tempfile::TempDir, Ctx) {
        let dir = tempfile::tempdir().unwrap();
        let ctx = crate::app::build_ctx(DataPaths::at(dir.path()), Platform::simulated(), true);
        i18n::set_lang(Lang::PtBr);
        (dir, ctx)
    }

    fn keys_of(layer: &dyn Layer, ctx: &Ctx) -> Vec<String> {
        layer
            .render(ctx)
            .rows
            .iter()
            .map(|r| r.key.to_string())
            .collect()
    }

    #[test]
    fn each_emulator_counts_the_roms_it_really_opens() {
        use tako_sources::emulators::emulator::Program;
        let mut found = Installed::default();
        found
            .programs
            .insert(Emulator::Dolphin, Program::exe("/emu/dolphin".into()));
        let mut flat = Program::exe("/usr/bin/flatpak".into());
        flat.flatpak = Some("org.DolphinEmu.dolphin-emu".into());
        let rom = |target: &str, args: &str, platform: &str| Game {
            rom_path: Some("/roms/x".into()),
            launch_target: Some(target.into()),
            launch_arguments: Some(args.into()),
            platform: Some(platform.into()),
            ..Default::default()
        };
        let games = vec![
            rom("/emu/dolphin", "-e x", "gamecube"),
            rom("/emu/dolphin", "-e y", "wii"),
            rom("/emu/pcsx2", "x", "ps2"),
            // Jogo de PC com o mesmo executável não conta: só ROMs.
            Game {
                launch_target: Some("/emu/dolphin".into()),
                ..Default::default()
            },
        ];
        let counts = by_emulator(&games, &found);
        assert_eq!(counts.len(), 1);
        assert_eq!(counts[0].0, Emulator::Dolphin);
        assert_eq!(counts[0].1.len(), 2);
        assert_eq!(system_name(counts[0].1[0]), Some("GameCube"));
        // Flatpak: o id nos argumentos decide.
        found.programs.insert(Emulator::Dolphin, flat);
        let games = vec![rom(
            "/usr/bin/flatpak",
            "run org.DolphinEmu.dolphin-emu -e x",
            "gamecube",
        )];
        assert_eq!(by_emulator(&games, &found)[0].1.len(), 1);
    }

    #[test]
    fn covers_and_mode_choices_switch_with_the_arrows_and_continue_with_enter() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, mut ctx) = fresh();
        ctx.desktop = true;
        let on = |layer: &MenuLayer, ctx: &Ctx, key: &str| {
            layer.render(ctx).rows.iter().any(|r| r.key == key && r.on)
        };
        // Capas: sem a IGDB ligada, "Usar só o que as lojas têm" vem marcada; o foco começa em Continuar.
        let mut c = covers(&ctx);
        assert!(on(&c, &ctx, "stores-only"));
        let sel = c.render(&ctx).sel as usize;
        assert_eq!(keys_of(&c, &ctx)[sel], "next");
        // → liga a IGDB; sem a chave da Twitch, abre o painel dela.
        assert!(matches!(c.handle(Cmd::Right, &mut ctx), Flow::Push(_)));
        assert!(on(&c, &ctx, "igdb"));
        // Sem conexão, a opção avisa.
        ONLINE.store(2, std::sync::atomic::Ordering::SeqCst);
        let desc = |c: &MenuLayer, ctx: &Ctx| {
            c.render(ctx)
                .rows
                .iter()
                .find(|r| r.key == "igdb")
                .map(|r| r.description.to_string())
        };
        assert_eq!(desc(&c, &ctx), Some(ctx.t("first_run.full.igdb_offline")));
        assert!(matches!(c.handle(Cmd::Left, &mut ctx), Flow::Stay));
        assert!(on(&c, &ctx, "stores-only"));
        ONLINE.store(0, std::sync::atomic::Ordering::SeqCst);

        // Modo: ←→ trocam a opção com o foco em Continuar; Enter segue para o Pronto.
        let mut m = mode(&ctx);
        assert!(on(&m, &ctx, crate::mode::OPEN_IN_DESKTOP));
        m.handle(Cmd::Right, &mut ctx);
        assert!(on(&m, &ctx, crate::mode::OPEN_IN_BIG_PICTURE));
        m.handle(Cmd::Left, &mut ctx);
        assert!(on(&m, &ctx, crate::mode::OPEN_IN_DESKTOP));
        let Flow::Replace(r) = m.handle(Cmd::Accept, &mut ctx) else {
            panic!("Enter continua")
        };
        assert_eq!(r.render(&ctx).tab, 4);
    }

    #[test]
    fn emulators_step_offers_the_rom_folder_and_a_way_out() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, mut ctx) = fresh();
        ctx.desktop = true;
        let e = emulators(&ctx);
        let keys = keys_of(&e, &ctx);
        // Sem pasta: a linha da pasta de ROMs oferece escolher uma.
        assert!(keys.contains(&"roms".to_owned()));
        assert!(keys.contains(&"add-emulator".to_owned()));
        // Linhas de emulador, no máximo quatro quando nenhum está instalado.
        assert!(keys.iter().filter(|k| k.starts_with("emu:")).count() <= Emulator::ALL.len());
        // "Não uso emuladores" pula para as capas.
        let item = (e.build)(&ctx)
            .into_iter()
            .find(|i| i.row.key == "no-emulators")
            .unwrap();
        let Flow::Replace(next) = (item.accept.unwrap())(&mut ctx) else {
            panic!("pular")
        };
        assert_eq!(next.render(&ctx).tab, 2);
    }

    #[test]
    fn the_desktop_onboarding_has_five_steps_and_saves_how_to_open() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, mut ctx) = fresh();
        ctx.desktop = true;
        let accept = |layer: &MenuLayer, ctx: &mut Ctx, key: &str| {
            let item = (layer.build)(ctx)
                .into_iter()
                .find(|i| i.row.key == key)
                .unwrap();
            (item.accept.unwrap())(ctx)
        };
        let next = |flow: Flow| match flow {
            Flow::Replace(l) => l,
            _ => panic!("o passo não avançou"),
        };
        // No desktop o primeiro passo já são as lojas; com jogos de emuladores, o segundo são os emuladores.
        let first = welcome(&ctx);
        let o = first.render(&ctx);
        assert_eq!((o.tab2, o.tabs.len(), o.tab), (1, 5, 0));
        assert_eq!(o.title, ctx.t("first_run.full.stores"));
        assert!(keys_of(&first, &ctx).contains(&"skip".to_owned()));
        // Emuladores só aparecem quando há algum neste PC; senão o passo é pulado e vai direto para as capas.
        // "Procurar jogos" procura e segue.
        let second = next(accept(&first, &mut ctx, "search")).render(&ctx);
        if super::super::settings::emulators::anything_found(&ctx) {
            assert_eq!(
                (second.tab, second.title),
                (1, ctx.t("first_run.full.emulators"))
            );
        } else {
            assert_eq!(
                (second.tab, second.title),
                (2, ctx.t("first_run.full.metadata"))
            );
        }
        assert_eq!(emulators(&ctx).render(&ctx).tab, 1);
        // Capas → Modo (no desktop o terceiro passo não termina).
        let providers = providers(&ctx);
        let mode_layer = next(accept(&providers, &mut ctx, "next"));
        assert_eq!(mode_layer.render(&ctx).tab, 3);
        // Sem controle, "Em janela" já vem marcado; escolher tela cheia grava a preferência.
        let mode = mode(&ctx);
        assert!(
            mode.render(&ctx)
                .rows
                .iter()
                .any(|r| r.key == crate::mode::OPEN_IN_DESKTOP && r.on)
        );
        let _ = accept(&mode, &mut ctx, crate::mode::OPEN_IN_BIG_PICTURE);
        assert_eq!(
            ctx.settings.get(crate::mode::OPEN_IN),
            Some(crate::mode::OPEN_IN_BIG_PICTURE)
        );
        let ready = next(accept(&mode, &mut ctx, "next"));
        let o = ready.render(&ctx);
        assert_eq!(o.tab, 4);
        assert!(o.rows.iter().any(|r| r.key == "done"));
        // Na tela cheia continua o assistente de três passos.
        ctx.desktop = false;
        assert_eq!(super::welcome(&ctx).render(&ctx).tabs.len(), 3);
    }

    #[test]
    fn shows_once_on_a_new_install_and_not_after_an_upgrade() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, mut ctx) = fresh();
        // `--simulate` acha os jogos de demonstração; sem eles, é uma instalação nova.
        ctx.found.lock().unwrap().clear();
        ctx.library.refresh();
        assert!(needed(&mut ctx));
        let _ = finish(&mut ctx);
        assert!(!needed(&mut ctx));

        // Com jogos: como quem atualiza de uma alpha.
        let (_d, mut ctx) = fresh();
        assert!(!needed(&mut ctx));
        assert!(ctx.settings.bool(keys::FIRST_RUN_DONE, false));
    }

    #[test]
    fn walks_language_stores_and_providers() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, mut ctx) = fresh();
        let mut layer: Box<dyn Layer> = Box::new(welcome(&ctx));
        // Foco começa em "Continuar"; sobe para o inglês e escolhe.
        layer.handle(Cmd::Up, &mut ctx);
        let Flow::Replace(l) = layer.handle(Cmd::Accept, &mut ctx) else {
            panic!("idioma")
        };
        layer = l;
        assert_eq!(i18n::lang(), Lang::EnUs);
        assert_eq!(ctx.settings.string(keys::LANGUAGE), "en-US");
        assert_eq!(layer.render(&ctx).title, i18n::t("first_run.welcome"));
        i18n::set_lang(Lang::PtBr);

        // Continuar → lojas.
        layer.handle(Cmd::Down, &mut ctx);
        let Flow::Replace(l) = layer.handle(Cmd::Accept, &mut ctx) else {
            panic!("lojas")
        };
        layer = l;
        let rows = keys_of(layer.as_ref(), &ctx);
        assert!(rows.iter().any(|k| k.starts_with("store:")));
        assert_eq!(rows.last().map(String::as_str), Some("next"));

        // B volta um passo; Continuar de novo e chega aos provedores.
        let Flow::Replace(back) = layer.handle(Cmd::Back, &mut ctx) else {
            panic!("voltar")
        };
        assert_eq!(
            keys_of(back.as_ref(), &ctx).last().map(String::as_str),
            Some("skip")
        );
        // O foco das lojas já começa em "Continuar".
        let mut s = stores(&ctx);
        let n = keys_of(&s, &ctx).len();
        assert_eq!(s.render(&ctx).sel, n as i32 - 1);
        let Flow::Replace(l) = s.handle(Cmd::Accept, &mut ctx) else {
            panic!("provedores")
        };
        layer = l;
        let rows = keys_of(layer.as_ref(), &ctx);
        assert!(rows.contains(&"settings.metadata.igdb".to_string()));
        assert_eq!(rows.last().map(String::as_str), Some("done"));
        assert!(!ctx.settings.bool(keys::FIRST_RUN_DONE, false));

        // B no primeiro passo pula e também conta como concluído.
        let mut w = welcome(&ctx);
        assert!(matches!(w.handle(Cmd::Back, &mut ctx), Flow::Close));
        assert!(ctx.settings.bool(keys::FIRST_RUN_DONE, false));
    }

    #[test]
    fn offers_the_optional_controller_driver_where_it_exists() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, mut ctx) = fresh();
        assert!(
            !keys_of(&providers(&ctx), &ctx).contains(&"settings.controls.hidhide".to_string())
        );
        ctx.platform.controller_hiding =
            std::sync::Arc::new(tako_core::devices::fake::FakeHiding::new(false));
        let p = providers(&ctx);
        let rows = keys_of(&p, &ctx);
        let at = rows.iter().position(|k| k == "settings.controls.hidhide");
        assert_eq!(at, Some(rows.len() - 2));
        // O foco continua em "Concluir".
        assert_eq!(p.render(&ctx).sel, rows.len() as i32 - 1);
    }
}
