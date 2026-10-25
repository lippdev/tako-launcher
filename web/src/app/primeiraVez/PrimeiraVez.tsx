import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { call, on } from "../../bridge";
import type { EmulatorsView } from "../../bridge/generated/EmulatorsView";
import type { LibraryPage } from "../../bridge/generated/LibraryPage";
import type { LibrarySources } from "../../bridge/generated/LibrarySources";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { CampoDeTexto } from "../../components/CampoDeTexto/CampoDeTexto";
import { Chave } from "../../components/Chave/Chave";
import { Icon } from "../../components/Icon/Icon";
import { LojaEncontrada, LojaEncontradaGroup } from "../../components/LojaEncontrada/LojaEncontrada";
import { OnboardingCapas } from "../../components/OnboardingCapas/OnboardingCapas";
import { OnboardingEmuladores } from "../../components/OnboardingEmuladores/OnboardingEmuladores";
import { OnboardingModo } from "../../components/OnboardingModo/OnboardingModo";
import { OnboardingPronto } from "../../components/OnboardingPronto/OnboardingPronto";
import { PassosDoOnboarding, PassosDoOnboardingMovimento, PassosDoOnboardingStaggerItem } from "../../components/PassosDoOnboarding/PassosDoOnboarding";
import { TelaOnboarding } from "../../components/TelaOnboarding/TelaOnboarding";
import { useT } from "../../i18n";
import { useDevArt } from "../busca/useDevArt";
import { isStoreDisabled } from "../settings/model";
import { onboardingStrings as s } from "../strings";
import { RevisarCapas } from "../revisarCapas";
import { coverChoice, finishChoice, IGDB_KEY, initialMode, modeChoice, nextStep, previousStep, storeChoice, type OpenMode, type Step, type Settings } from "./model";
import common from "../../components/OnboardingEmuladores/OnboardingEmuladores.module.css";
import styles from "./PrimeiraVez.module.css";

export function PrimeiraVez({ initialSettings, onFinish }: { initialSettings: Settings; onFinish: () => void }) {
  const t = useT();
  const [step, setStep] = useState<Step>(1);
  const [values, setValues] = useState(initialSettings);
  const [sources, setSources] = useState<LibrarySources | null>(null);
  const [emulators, setEmulators] = useState<EmulatorsView | null>(null);
  const [page, setPage] = useState<LibraryPage | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [reload, setReload] = useState(0);
  const [folderOpen, setFolderOpen] = useState(false);
  const [reviewOpen, setReviewOpen] = useState(false);
  const [path, setPath] = useState("");
  // TODO(WEB-05b): o contrato desta base ainda não informa os controles conectados; por ora o padrão é janela.
  const [mode, setMode] = useState<OpenMode>(() => initialMode(initialSettings, false));
  const lock = useRef(false);
  const focusAfterSave = useRef<HTMLElement | null>(null);
  const root = useRef<HTMLDivElement>(null);
  const devArt = useDevArt();
  const ready = !!page && !!emulators && !!sources;

  useEffect(() => {
    let cancelled = false;
    const refreshLibrary = () => {
      void call("library.list", { limit: 15 }).then((data) => {
        if (!cancelled) setPage(data);
      }).catch(() => { if (!cancelled) setError(s.loadError); });
    };
    const offScan = on("library.scan.progress", (data) => {
      if (!cancelled) setSources(data);
    });
    const offLibrary = on("library.changed", refreshLibrary);
    void Promise.all([call("library.sources"), call("emulators.list"), call("library.list", { limit: 15 }), call("settings.get", { keys: [IGDB_KEY] })])
      .then(([foundSources, foundEmulators, games, igdb]) => {
        if (cancelled) return;
        setSources(foundSources);
        setEmulators(foundEmulators);
        setPage(games);
        setValues((current) => ({ ...current, ...igdb.values }));
      })
      .catch(() => { if (!cancelled) setError(s.loadError); });
    return () => { cancelled = true; offScan(); offLibrary(); };
  }, [reload]);

  useEffect(() => {
    if (busy || !ready || folderOpen) return;
    const saved = focusAfterSave.current;
    const target = saved?.isConnected && saved !== document.body ? saved : root.current?.querySelector<HTMLButtonElement>('button[data-continue], main button[class*="primary"]');
    target?.focus({ preventScroll: true });
    focusAfterSave.current = null;
  }, [step, ready, busy, folderOpen]);

  useEffect(() => { if (root.current) root.current.scrollTop = 0; }, [step]);

  function run(action: () => Promise<void>) {
    if (lock.current) return;
    focusAfterSave.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    lock.current = true;
    setBusy(true);
    setError("");
    void action().catch(() => { setError(s.saveError); }).finally(() => {
      lock.current = false;
      setBusy(false);
    });
  }

  async function save(setting: { key: string; value: string }) {
    await call("settings.set", setting);
    setValues((current) => ({ ...current, [setting.key]: setting.value }));
  }

  function finish() {
    run(async () => { await call("settings.set", finishChoice()); onFinish(); });
  }

  const hasEmulators = (emulators?.emulators.some((item) => item.found) ?? false) || (emulators?.romFolders.length ?? 0) > 0;
  function proceed() {
    if (!page || !emulators || !sources) return;
    if (step === 5) { finish(); return; }
    run(async () => {
      if (step === 1) await call("library.scan");
      if (step === 4) await save(modeChoice(mode));
      setFolderOpen(false);
      setStep(nextStep(step, hasEmulators));
    });
  }

  function back() {
    if (lock.current) return;
    if (folderOpen) { setFolderOpen(false); return; }
    const previous = previousStep(step, hasEmulators);
    if (previous === null) finish();
    else setStep(previous);
  }

  function keyboard(event: KeyboardEvent) {
    if (event.repeat || event.altKey || event.ctrlKey || event.metaKey || event.nativeEvent.isComposing) return;
    if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); back(); return; }
    if (folderOpen || lock.current || event.target instanceof HTMLInputElement) return;
    if ((event.key === "ArrowLeft" || event.key === "ArrowRight") && !(event.target instanceof Element && event.target.closest('[role="radio"]'))) {
      if (step === 3) {
        event.preventDefault();
        run(() => save(coverChoice(values[IGDB_KEY] !== "true")));
      }
      if (step === 4) {
        event.preventDefault();
        const value = mode === "desktop" ? "big-picture" : "desktop";
        run(async () => { await save(modeChoice(value)); setMode(value); });
      }
    }
    // Enter no foco inicial continua; botões e rádios mantêm a própria ação quando o usuário navega por Tab.
    if (event.key === "Enter" && !(event.target instanceof Element && event.target.closest("button"))) {
      event.preventDefault(); proceed();
    }
  }

  const count = (n: number) => s.gameCount(n);
  const stepLabel = t("first_run.count", { n: step, total: 5 });
  const next = t("first_run.next");
  const goBack = { backLabel: t("first_run.full.back"), onBack: back };
  const cover = (game: NonNullable<LibraryPage["rows"][number]>) => devArt ? devArt.cover(game.id, game.title) : `tako-img://cover?id=${encodeURIComponent(game.id)}&w=240&h=320`;
  const comparison = page?.rows.find((game) => game.hasCover);
  const stores = sources?.sources.filter((source) => source.store !== "emulators" && source.store !== "manual") ?? [];

  if (reviewOpen) {
    return (
      <div className={styles["viewport"]}>
        <RevisarCapas
          onConcluir={finish}
          onVoltar={() => setReviewOpen(false)}
        />
      </div>
    );
  }

  return <div
    ref={root}
    className={styles["viewport"]}
    onKeyDown={keyboard}
  >
    <fieldset
      disabled={busy}
      className={styles["controls"]}
      aria-busy={busy}
    >
      <TelaOnboarding
        className={styles["screen"] ?? ""}
        skipLabel={t("first_run.full.skip")}
        onSkip={finish}
        covers={page?.rows.filter((game) => game.hasCover).map(cover) ?? []}
      >
        {!ready ? <div className={styles["extra"]}>
          <p role="status">{error || s.loading}</p>
          {error && <BotaoDesktop onClick={() => { setError(""); setReload((n) => n + 1); }}>{s.retry}</BotaoDesktop>}
        </div> : <>
          {step === 1 && <PassosDoOnboardingMovimento stepKey={step} className={common["root"] ?? ""}>
            <PassosDoOnboardingStaggerItem index={0}><PassosDoOnboarding currentStep={step} label={stepLabel} /></PassosDoOnboardingStaggerItem>
            <PassosDoOnboardingStaggerItem index={1}><h1 className={common["title"]}>{t("first_run.full.stores")}</h1></PassosDoOnboardingStaggerItem>
            <PassosDoOnboardingStaggerItem index={2}><p className={common["description"]}>{t("first_run.full.stores_text")}</p></PassosDoOnboardingStaggerItem>
            <PassosDoOnboardingStaggerItem index={3}><LojaEncontradaGroup className={styles["stores"] ?? ""}>
              {stores.map((source) => {
                const enabled = !isStoreDisabled(values["DisabledStores"], source.store);
                return <LojaEncontrada
                  key={source.store}
                  icon={<Icon name="store" size="sm" />}
                  name={source.name}
                  status={!enabled ? "action" : source.clientInstalled ? "ready" : "not-installed"}
                  statusLabel={source.clientInstalled ? count(source.found) : t("first_run.full.stores_missing")}
                  control={<Chave
                    aria-label={s.toggleStore(source.name)}
                    checked={enabled}
                    onChange={(checked) => run(async () => { await save(storeChoice(values["DisabledStores"], source.store, checked)); await call("library.scan"); })}
                  />}
                />;
              })}
            </LojaEncontradaGroup></PassosDoOnboardingStaggerItem>
            <PassosDoOnboardingStaggerItem index={4}><div className={styles["actions"]}>
              <BotaoDesktop
                data-continue
                icon="chevron-right"
                onClick={proceed}
              >{t("first_run.full.stores_search")}</BotaoDesktop>
              <BotaoDesktop variant="outlined" onClick={() => { setStep(2); setFolderOpen(true); }}>{t("first_run.full.stores_choose")}</BotaoDesktop>
            </div></PassosDoOnboardingStaggerItem>
            <p className={common["footerNote"]}>{t("first_run.full.stores_note")}</p>
          </PassosDoOnboardingMovimento>}
          {step === 2 && <>
            <OnboardingEmuladores
              stepLabel={stepLabel}
              title={t("first_run.full.emulators")}
              description={t("first_run.full.emulators_text")}
              emulators={emulators.emulators.slice(0, 4).map((item) => ({ id: item.key, name: item.name, status: item.found ? "ready" : "not-installed", details: item.path ?? "", statusLabel: item.found ? s.found : t("first_run.full.stores_missing") }))}
              romsFolder={{ name: t("first_run.full.roms"), details: emulators.romFolders.map((folder) => folder.path).join(" · ") || t("first_run.full.roms_none"), actionLabel: t("first_run.full.roms_choose"), onAction: () => { setFolderOpen(true); } }}
              continueLabel={next}
              onContinue={proceed}
              skipEmulatorsLabel={t("first_run.full.no_emulators")}
              onSkipEmulators={proceed}
              footerNote={t("first_run.full.emulators_note")}
            />
            {folderOpen && <form className={styles["extra"]} onSubmit={(event) => {
              event.preventDefault();
              if (!path.trim()) return;
              run(async () => { const data = await call("emulators.folder.add", { path: path.trim() }); setEmulators(data); setPath(""); setFolderOpen(false); });
            }}>
              <CampoDeTexto
                label={t("first_run.full.roms")}
                value={path}
                onChange={setPath}
                autoFocus
                help={s.folderHelp}
              />
              <div className={styles["actions"]}><BotaoDesktop type="submit" disabled={!path.trim()}>{s.addFolder}</BotaoDesktop><BotaoDesktop variant="outlined" onClick={() => { setFolderOpen(false); }}>{s.cancel}</BotaoDesktop></div>
            </form>}
            <BotaoDesktop variant="outlined" onClick={back}>{goBack.backLabel}</BotaoDesktop>
          </>}
          {step === 3 && <OnboardingCapas
            stepLabel={stepLabel}
            title={t("first_run.full.metadata")}
            description={t("first_run.full.metadata_text")}
            gameTitle={comparison?.title ?? s.noGame}
            {...(comparison ? { gameCoverUrl: cover(comparison) } : {})}
            comparisonNote={t("first_run.full.cover_with")}
            optionsAriaLabel={t("first_run.metadata")}
            options={[{ value: "igdb", title: t("first_run.full.igdb"), description: s.igdbKeyNote }, { value: "stores", title: t("first_run.full.stores_only"), description: t("first_run.full.stores_only_desc") }]}
            value={values[IGDB_KEY] === "true" ? "igdb" : "stores"}
            onChange={(value) => run(() => save(coverChoice(value === "igdb")))}
            continueLabel={next}
            onContinue={proceed}
            {...goBack}
            footerNote={t("first_run.full.metadata_note")}
          />}
          {step === 4 && <OnboardingModo
            stepLabel={stepLabel}
            title={t("first_run.full.mode")}
            description={t("first_run.full.mode_text")}
            optionsAriaLabel={t("first_run.full.mode")}
            options={[{ value: "desktop", title: t("first_run.full.mode_window"), description: t("first_run.full.mode_window_desc"), hint: t("first_run.full.mode_window_hint") }, { value: "big-picture", title: t("first_run.full.mode_full"), description: t("first_run.full.mode_full_desc"), hint: t("first_run.full.mode_full_hint") }]}
            value={mode}
            onChange={(value) => run(async () => { await save(modeChoice(value)); setMode(value); })}
            continueLabel={next}
            onContinue={proceed}
            {...goBack}
            footerNote={s.fullscreenNote}
          />}
          {step === 5 && <OnboardingPronto
            stepLabel={stepLabel}
            title={t(page.total ? "first_run.full.ready" : "first_run.full.ready_none", { n: page.total })}
            description={t(page.total ? "first_run.full.ready_text" : "first_run.full.ready_none_text")}
            summaryItems={sources.sources.filter((source) => source.enabled && source.found > 0).map((source) => ({ id: source.store, name: source.name, countLabel: count(source.found) }))}
            selectedMode={{ title: t("first_run.full.opens_in"), description: t(mode === "big-picture" ? "first_run.full.opens_full" : "first_run.full.opens_window"), changeLabel: t("first_run.full.change"), onChangeMode: () => { setStep(4); } }}
            finishLabel={t("first_run.full.open_library")}
            onFinish={finish}
            {...(page.total ? { reviewCoversLabel: t("first_run.full.review_covers") } : {})}
            onReviewCovers={() => {
              setReviewOpen(true);
            }}
          />}
          {error && <p role="alert" className={styles["notice"]}>{error}</p>}
        </>}
      </TelaOnboarding>
    </fieldset>
  </div>;
}
