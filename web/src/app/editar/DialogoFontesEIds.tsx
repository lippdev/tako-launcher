import { useEffect, useMemo, useState } from "react";
import type { SourceIdChoice } from "../../bridge/generated/SourceIdChoice";
import type { SourceIdMatch } from "../../bridge/generated/SourceIdMatch";
import type { SourceIdState } from "../../bridge/generated/SourceIdState";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { CampoDeTexto } from "../../components/CampoDeTexto/CampoDeTexto";
import { Dialogo } from "../../components/Dialogo/Dialogo";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { useT } from "../../i18n";
import { describeSource, matchDetail, matchMarker, type SourceLabels } from "./editarModel";
import { useSourceIds } from "./useSourceIds";
import styles from "./editar.module.css";

interface DialogoFontesEIdsProps {
  gameId: string;
  open: boolean;
  onClose: () => void;
  /** Confirmações curtas (a fonte passou a usar tal id) e falhas. */
  onNotice: (tone: "success" | "error", text: string) => void;
}

/**
 * "Fontes e IDs" (port do `source_ids` do Slint, ADR-0023): para cada fonte de metadados ligada, o id casado com o
 * nome e o ano, os candidatos da busca pelo nome em todas as fontes ao mesmo tempo (✓ = o casado, ★ = o mais
 * provável) e as ações "Digitar ou colar o ID", "Nenhum" e "Automático". Um id digitado só é fixado depois de a
 * fonte dizer qual jogo é e o usuário confirmar.
 */
export function DialogoFontesEIds({ gameId, open, onClose, onNotice }: DialogoFontesEIdsProps) {
  const t = useT();
  const ids = useSourceIds(gameId, open);
  const { view, loadFailed, results, term, searching, searchFailed, check, setCheck, search, verify, choose } = ids;
  /** O que o usuário digitou no campo do nome; enquanto não digita, vale o título (a busca de abertura saiu com ele). */
  const [termEdit, setTermEdit] = useState<string | null>(null);
  const [typing, setTyping] = useState<string | null>(null);
  const [typed, setTyped] = useState("");
  const termText = termEdit ?? view?.title ?? "";

  const labels: SourceLabels = useMemo(
    () => ({
      noneChosen: t("source_ids.none_chosen"),
      noMatch: t("source_ids.no_match"),
      chosen: t("source_ids.chosen"),
      automatic: t("source_ids.automatic_lower"),
    }),
    [t],
  );

  const [wasOpen, setWasOpen] = useState(open);
  if (open !== wasOpen) {
    setWasOpen(open);
    if (open) {
      setTermEdit(null);
      setTyping(null);
      setTyped("");
    }
  }

  useEffect(() => {
    if (open && loadFailed) {
      onNotice("error", t("source_ids.load_error"));
      onClose();
    }
  }, [open, loadFailed, onNotice, onClose, t]);

  if (!view) return null;

  const apply = async (source: string, choice: SourceIdChoice, name: string) => {
    const result = await choose(source, choice);
    if (!result.ok) {
      onNotice("error", t("source_ids.save_failed"));
      return;
    }
    setTyping(null);
    setTyped("");
    onNotice(
      "success",
      choice.kind === "auto"
        ? t("source_ids.auto_done", { source })
        : choice.kind === "none"
          ? t("source_ids.none_done", { game: view.title, source })
          : t("source_ids.pinned_done", { game: view.title, name, source }),
    );
  };

  const startTyping = (source: string) => {
    setTyping(source);
    setTyped("");
    setCheck(null);
  };

  const pick = (state: SourceIdState, match: SourceIdMatch) => {
    void apply(state.source, { kind: "id", value: match.id }, match.title);
  };

  const sourceRows = (state: SourceIdState) => {
    const found = results.find((r) => r.source === state.source);
    const matches = found?.matches ?? [];
    const best = found?.best ?? null;
    const isTyping = typing === state.source;
    const mine = check && check.source === state.source ? check : null;
    return (
      <GrupoDeConfiguracao key={state.source} title={state.source.toUpperCase()}>
        <LinhaDeConfiguracao title={describeSource(state, labels)} />
        <LinhaDeConfiguracao
          title={t("source_ids.type_id")}
          description={state.example}
          onClick={() => {
            if (isTyping) setTyping(null);
            else startTyping(state.source);
          }}
        />
        {isTyping && (
          <form
            className={styles["typing"]}
            onSubmit={(event) => {
              event.preventDefault();
              if (typed.trim() !== "") verify(state.source, typed);
            }}
          >
            <div className={styles["typingRow"]}>
              <CampoDeTexto
                label={t("source_ids.id_in", { source: state.source })}
                value={typed}
                placeholder={state.example}
                autoFocus
                maxLength={1024}
                error={mine?.state === "invalid" ? mine.message : mine?.state === "unknown" ? t("source_ids.unknown_id", { source: state.source }) : undefined}
                onChange={(value) => {
                  setTyped(value);
                  if (mine) setCheck(null);
                }}
              />
              <BotaoDesktop type="submit" variant="outlined" disabled={typed.trim() === "" || mine?.state === "checking"}>
                {t("source_ids.check")}
              </BotaoDesktop>
            </div>
            {mine?.state === "checking" && (
              <p className={styles["info"]} role="status">
                {t("source_ids.checking", { source: state.source })}
              </p>
            )}
            {mine?.state === "found" && (
              <div className={styles["confirm"]} role="status">
                <p className={styles["confirmTitle"]}>{t("source_ids.confirm_title", { name: mine.name })}</p>
                <p className={styles["info"]}>{t("source_ids.confirm_text", { source: state.source, id: mine.id })}</p>
                <div className={styles["confirmActions"]}>
                  <BotaoDesktop
                    size="sm"
                    onClick={() => {
                      void apply(state.source, { kind: "id", value: mine.id }, mine.name);
                    }}
                  >
                    {t("source_ids.use")}
                  </BotaoDesktop>
                  <BotaoDesktop
                    size="sm"
                    variant="outlined"
                    onClick={() => {
                      setCheck(null);
                    }}
                  >
                    {t("common.cancel")}
                  </BotaoDesktop>
                </div>
              </div>
            )}
          </form>
        )}
        <LinhaDeConfiguracao
          title={t("source_ids.none")}
          description={t("source_ids.none_text")}
          onClick={() => {
            void apply(state.source, { kind: "none" }, "");
          }}
        />
        {state.pinned && (
          <LinhaDeConfiguracao
            title={t("source_ids.automatic")}
            description={t("source_ids.automatic_text")}
            onClick={() => {
              void apply(state.source, { kind: "auto" }, "");
            }}
          />
        )}
        {searching && term !== null && <LinhaDeConfiguracao title={t("source_ids.searching", { term })} />}
        {!searching && term !== null && matches.length === 0 && <LinhaDeConfiguracao title={t("source_ids.nothing", { term })} />}
        {!searching &&
          matches.map((match) => {
            const marker = matchMarker(match, state, best);
            return (
              <LinhaDeConfiguracao
                key={match.id}
                title={`${marker === "current" ? "✓ " : marker === "best" ? "★ " : ""}${match.title}`}
                description={matchDetail(match)}
                onClick={() => {
                  pick(state, match);
                }}
              />
            );
          })}
      </GrupoDeConfiguracao>
    );
  };

  return (
    <Dialogo open={open} onClose={onClose} title={t("source_ids.title", { game: view.title })} closeLabel={t("common.close")} size="l">
      <div className={styles["list"]}>
        <form
          className={styles["search"]}
          onSubmit={(event) => {
            event.preventDefault();
            search(termText);
          }}
        >
          <CampoDeTexto
            label={t("source_ids.term")}
            value={termText}
            maxLength={200}
            onChange={setTermEdit}
            error={searchFailed ? t("source_ids.search_error") : undefined}
          />
          <BotaoDesktop type="submit" variant="outlined" icon="search" disabled={termText.trim() === "" || searching}>
            {t("source_ids.search")}
          </BotaoDesktop>
        </form>
        {view.sources.length === 0 ? <p className={styles["info"]}>{t("source_ids.no_sources")}</p> : view.sources.map(sourceRows)}
      </div>
    </Dialogo>
  );
}
