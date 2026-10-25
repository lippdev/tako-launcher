// Página de desenvolvimento (`#/dev`): mostra a base do design system para conferir no navegador.
// Não faz parte do produto. Os textos daqui ficam fora dos catálogos de `locales/` de propósito.

import { useEffect, useState, type ReactNode } from "react";
import { setAppearance, THEMES, UI_SCALES, useAppearance, type Theme, type UiScale } from "../appearance/appearance";
import { call } from "../bridge";
import type { AppInfo } from "../bridge/generated/AppInfo";
import { ICON_NAMES, Icon } from "../components/Icon/Icon";
import { Marca } from "../components/Marca/Marca";
import { cx } from "../lib/cx";
import { COLOR_VARS } from "../styles/colorVars.generated";
import styles from "./DevPage.module.css";

const THEME_LABELS: Record<Theme, string> = {
  tako: "TAKO",
  escuro: "Escuro",
  "meia-noite": "Meia-noite",
  oled: "Preto OLED",
  vidro: "Liquid Glass",
};

/** Escala tipográfica: as classes do `tokens.css` (README do design system, "Tipografia"). */
const TYPE_SCALE: { cls: string; spec: string; sample: string }[] = [
  { cls: "headline", spec: "28/700 Poppins", sample: "Configurações" },
  { cls: "page-title", spec: "26/800 Poppins", sample: "God of War Ragnarök" },
  { cls: "title", spec: "19/700 Poppins", sample: "Continuar jogando" },
  { cls: "value", spec: "22/700 Poppins", sample: "1.234 h" },
  { cls: "tab", spec: "12/400 Poppins, 0,32 em", sample: "LOJA BIBLIOTECA" },
  { cls: "label-lg", spec: "14/600 Poppins", sample: "Jogar agora" },
  { cls: "body", spec: "13,5/400 Plus Jakarta Sans", sample: "Kratos e Atreus atravessam os nove reinos em busca de respostas." },
  { cls: "label", spec: "14/400 Plus Jakarta Sans", sample: "Biblioteca de jogos" },
  { cls: "label-sm", spec: "12,5/500 Plus Jakarta Sans", sample: "Última sessão há 2 dias" },
  { cls: "overline", spec: "11/700 Plus Jakarta Sans", sample: "TEMPO DE JOGO" },
  { cls: "keycap", spec: "10,5/500 Plus Jakarta Sans", sample: "Ctrl K" },
];

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className={styles["section"]}>
      <h2 className="title">{title}</h2>
      {children}
    </section>
  );
}

function Controls() {
  const a = useAppearance();
  return (
    <div className={styles["controls"]}>
      <label className={styles["field"]}>
        <span className="label-sm">Tema</span>
        <select
          className={styles["select"]}
          value={a.theme}
          onChange={(e) => {
            setAppearance({ theme: e.target.value as Theme });
          }}
        >
          {THEMES.map((t) => (
            <option key={t} value={t}>
              {THEME_LABELS[t]}
            </option>
          ))}
        </select>
      </label>
      <label className={styles["field"]}>
        <span className="label-sm">Tamanho da interface</span>
        <select
          className={styles["select"]}
          value={a.uiScale}
          onChange={(e) => {
            setAppearance({ uiScale: Number(e.target.value) as UiScale });
          }}
        >
          {UI_SCALES.map((s) => (
            <option key={s} value={s}>
              {s}%
            </option>
          ))}
        </select>
      </label>
      <label className={styles["check"]}>
        <input
          type="checkbox"
          checked={a.reduceMotion}
          onChange={(e) => {
            setAppearance({ reduceMotion: e.target.checked });
          }}
        />
        <span className="label">Reduzir animações</span>
      </label>
      <label className={styles["check"]}>
        <input
          type="checkbox"
          checked={a.glassEffects}
          onChange={(e) => {
            setAppearance({ glassEffects: e.target.checked });
          }}
        />
        <span className="label">Efeitos de vidro</span>
      </label>
      <p className={cx("label-sm", styles["muted"])}>data-theme=&quot;{document.documentElement.dataset["theme"]}&quot;</p>
    </div>
  );
}

function Palette() {
  // Lido na renderização: `setAppearance` já escreveu o tema no `<html>` antes de avisar a tela.
  const computed = getComputedStyle(document.documentElement);
  return (
    <ul className={styles["swatches"]}>
      {COLOR_VARS.map((name) => (
        <li key={name} className={styles["swatch"]}>
          <span className={styles["chip"]}>
            <span className={styles["chipColor"]} style={{ background: `var(${name})` }} />
          </span>
          <span className="label-sm">{name}</span>
          <span className={cx("label-sm", styles["muted"])}>{computed.getPropertyValue(name).trim()}</span>
        </li>
      ))}
    </ul>
  );
}

type InfoState = { status: "loading" } | { status: "ok"; data: AppInfo } | { status: "error"; message: string };

function BridgeInfo() {
  const [info, setInfo] = useState<InfoState>({ status: "loading" });
  useEffect(() => {
    let alive = true;
    call("app.info").then(
      (data) => {
        if (alive) setInfo({ status: "ok", data });
      },
      (e: unknown) => {
        if (alive) setInfo({ status: "error", message: e instanceof Error ? e.message : String(e) });
      },
    );
    return () => {
      alive = false;
    };
  }, []);
  return (
    <pre className={cx("label-sm", styles["code"])}>
      {info.status === "loading" && "app.info …"}
      {info.status === "ok" && `app.info → ${JSON.stringify(info.data)}`}
      {info.status === "error" && `app.info falhou: ${info.message}`}
    </pre>
  );
}

export function DevPage() {
  return (
    <main className={styles["page"]}>
      <h1 className="page-title">Base do design system</h1>
      <Section title="Aparência e acessibilidade">
        <Controls />
        <div className={styles["glassDemo"]}>
          <div className={styles["glassPanel"]}>
            <span className="label-lg">Painel de vidro</span>
            <span className={cx("label-sm", styles["muted"])}>Com &quot;Efeitos de vidro&quot; desligado o desfoque some.</span>
          </div>
        </div>
        <div className={styles["motionDemo"]}>
          <span className={styles["track"]}>
            <span className={styles["mover"]} />
          </span>
          <span className={cx("label-sm", styles["muted"])}>Com &quot;Reduzir animações&quot; a bolinha para.</span>
        </div>
      </Section>
      <Section title="Ponte (app.info)">
        <BridgeInfo />
      </Section>
      <Section title="Marca">
        <div className={styles["brandRow"]}>
          <div className={styles["brandBar"]}>
            <Marca />
          </div>
          <Marca variant="horizontal" />
        </div>
      </Section>
      <Section title="Paleta do tema atual">
        <Palette />
      </Section>
      <Section title="Tipografia">
        <ul className={styles["types"]}>
          {TYPE_SCALE.map(({ cls, spec, sample }) => (
            <li key={cls} className={styles["typeRow"]}>
              <span className={cx("label-sm", styles["muted"])}>
                .{cls}
                <br />
                {spec}
              </span>
              <span className={cls}>{sample}</span>
            </li>
          ))}
        </ul>
      </Section>
      <Section title={`Ícones (${String(ICON_NAMES.length)})`}>
        <ul className={styles["icons"]}>
          {ICON_NAMES.map((name) => (
            <li key={name} className={styles["iconCell"]}>
              <Icon name={name} />
              <span className="label-sm">{name}</span>
            </li>
          ))}
        </ul>
        <div className={styles["iconSizes"]}>
          <Icon name="gear" /> <span className="label-sm">18 px</span>
          <Icon name="gear" size="sm" /> <span className="label-sm">15 px</span>
          <Icon name="play" filled /> <Icon name="star" filled /> <Icon name="heart" filled />
          <span className="label-sm">preenchidos (ligado)</span>
        </div>
      </Section>
    </main>
  );
}
