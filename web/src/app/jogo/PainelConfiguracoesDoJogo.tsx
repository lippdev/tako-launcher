import { useEffect, useMemo, useRef, useState } from "react";
import type { ErrorCode } from "../../bridge/generated/ErrorCode";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import {
  ConfiguracoesDoJogo,
  type ChaveConfiguracaoJogo,
  type ConfiguracoesDoJogoLabels,
  type ValorConfiguracaoJogo,
  type ValoresConfiguracoesDoJogo,
} from "../../components/ConfiguracoesDoJogo/ConfiguracoesDoJogo";
import { Dialogo } from "../../components/Dialogo/Dialogo";
import { useT, type TFunction } from "../../i18n";
import { modeLabel } from "../settings/model";
import { panelModel, toChange, type PanelLabels } from "./configuracoesModel";
import { useGameSettings } from "./useGameSettings";

/** O texto do aviso para uma gravação que a ponte recusou ou que falhou. */
function failureText(
  key: ChaveConfiguracaoJogo,
  value: ValorConfiguracaoJogo,
  code: ErrorCode,
  t: TFunction,
): string {
  if (key === "pastaDeTrabalho" && code === "not_found") return t("game_settings.folder_missing", { folder: String(value).trim() });
  if (key === "variaveisDeAmbiente" && code === "bad_request") return t("game_settings.invalid_environment");
  if ((key === "argumentos" || key === "pastaDeTrabalho") && code === "bad_request") return t("game_settings.invalid");
  return t("game_settings.save_failed");
}

/** O mesmo objeto sem a chave `key`. */
function without<T extends object>(object: T, key: keyof T): T {
  return Object.fromEntries(Object.entries(object).filter(([name]) => name !== key)) as T;
}

interface PainelConfiguracoesDoJogoProps {
  gameId: string;
  title: string;
  /** A capa do cabeçalho; sem ela o cabeçalho fica só com o título. */
  coverUrl: string | undefined;
  open: boolean;
  onClose: () => void;
  /** Aviso de erro (a gravação falhou e o valor voltou ao de antes). */
  onError: (text: string) => void;
}

/**
 * "Configurações deste jogo" ligado ao núcleo (`game.settings`): cada mudança grava na hora. O valor novo aparece
 * antes da resposta; se a ponte falhar, volta ao de antes e um aviso diz o que houve. "Usar tudo do Tako" pergunta
 * antes de apagar as escolhas do jogo.
 */
export function PainelConfiguracoesDoJogo({ gameId, title, coverUrl, open, onClose, onError }: PainelConfiguracoesDoJogoProps) {
  const t = useT();
  const { view, loadFailed, save } = useGameSettings(gameId, open);
  const [pending, setPending] = useState<{ values: ValoresConfiguracoesDoJogo; tokens: Partial<Record<ChaveConfiguracaoJogo, number>> }>({
    values: {},
    tokens: {},
  });
  const [confirming, setConfirming] = useState(false);
  const counter = useRef(0);

  const panelLabels = useMemo<PanelLabels>(
    () => ({
      noScreen: t("game_settings.no_screen_cap"),
      keepCurrent: t("game_settings.keep_current"),
      keepAudio: t("game_settings.keep_audio"),
      on: t("game_settings.on"),
      off: t("game_settings.off"),
      disconnected: (name: string) => t("game_settings.disconnected", { screen: name }),
      mode: modeLabel,
      screenNote: t("game_settings.screen_note"),
      whichScreenNote: t("game_settings.which_screen_desc"),
      launchNote: t("game_settings.launch_note"),
      environmentNote: t("game_settings.environment_hint"),
      commandsNote: t("game_settings.commands_edit_disabled"),
      commandsBefore: t("game_settings.commands_before"),
      commandsAfter: t("game_settings.commands_after"),
      vrrUnsupported: t("vrr.unsupported"),
      audioLocked: t("game_settings.audio_locked"),
    }),
    [t],
  );

  const rotulos = useMemo<ConfiguracoesDoJogoLabels>(
    () => ({
      title: t("details.game_settings"),
      close: t("game_settings.close"),
      screenGroup: t("game_settings.group_screen"),
      soundGroup: t("game_settings.group_sound"),
      launchGroup: t("game_settings.group_launch"),
      useScreen: t("game_settings.use_screen"),
      monitor: t("game_settings.which_screen"),
      resolutionAndRate: t("game_settings.mode"),
      hdr: t("game_settings.hdr"),
      vrr: t("vrr.game"),
      audioOutput: t("game_settings.audio"),
      arguments: t("game_settings.arguments"),
      workingDirectory: t("game_settings.folder"),
      environmentVariables: t("game_settings.environment"),
      beforeAndAfterCommands: t("game_settings.commands"),
      useAllFromTako: t("game_settings.reset"),
      useAllFromTakoDescription: (changes: number) =>
        changes === 0 ? t("game_settings.reset_desc_none") : t("game_settings.reset_desc", { n: changes }),
      defaultPrefix: t("game_settings.default_prefix"),
      none: t("game_settings.none"),
      noneFem: t("game_settings.none_fem"),
      on: t("game_settings.on"),
      off: t("game_settings.off"),
      singularChange: t("game_settings.change_singular"),
      pluralChanges: t("game_settings.change_plural"),
      noChanges: t("game_settings.change_none"),
    }),
    [t],
  );

  const emptyText = useMemo(
    () =>
      ({
        argumentos: t("game_settings.none"),
        pastaDeTrabalho: t("game_settings.exe_folder"),
        variaveisDeAmbiente: t("game_settings.none_fem"),
        comandosAntesEDepois: t("game_settings.none"),
      }) as const,
    [t],
  );

  const model = useMemo(() => (view ? panelModel(view, panelLabels) : null), [view, panelLabels]);

  useEffect(() => {
    if (!open || !loadFailed || view) return;
    onError(t("game_settings.load_error"));
    onClose();
  }, [open, loadFailed, view, onError, onClose, t]);

  if (!view || !model) return null;

  const change = async (key: ChaveConfiguracaoJogo, value: ValorConfiguracaoJogo) => {
    const request = toChange(key, value, view);
    if (!request) return;
    const token = ++counter.current;
    setPending((p) => ({ values: { ...p.values, [key]: value }, tokens: { ...p.tokens, [key]: token } }));
    const result = await save(request);
    // Só a última escolha da linha limpa o valor provisório; a anterior já foi substituída.
    setPending((p) => {
      if (p.tokens[key] !== token) return p;
      return { values: without(p.values, key), tokens: without(p.tokens, key) };
    });
    if (!result.ok) onError(failureText(key, value, result.code, t));
  };

  const reset = async () => {
    setConfirming(false);
    const result = await save({ field: "reset_all" });
    if (!result.ok) onError(t("game_settings.save_failed"));
  };

  return (
    <>
      <ConfiguracoesDoJogo
        open={open}
        onClose={onClose}
        gameTitle={title}
        {...(coverUrl ? { coverUrl } : {})}
        values={{ ...model.valores, ...pending.values }}
        defaults={model.padroes}
        options={model.opcoes}
        descriptions={model.descricoes}
        labels={rotulos}
        changesCount={model.mudancas}
        readOnly={model.somenteLeitura}
        emptyText={emptyText}
        onChange={(key, value) => void change(key, value)}
        onResetToDefault={() => {
          if (model.mudancas > 0) setConfirming(true);
        }}
      />
      <Dialogo
        open={confirming}
        onClose={() => {
          setConfirming(false);
        }}
        title={t("game_settings.reset_title")}
        closeLabel={t("game_settings.close")}
        size="s"
        actions={
          <>
            <BotaoDesktop
              variant="outlined"
              onClick={() => {
                setConfirming(false);
              }}
            >
              {t("common.cancel")}
            </BotaoDesktop>
            <BotaoDesktop variant="primary" onClick={() => void reset()}>
              {t("game_settings.reset")}
            </BotaoDesktop>
          </>
        }
      >
        <p>{t("game_settings.reset_text")}</p>
      </Dialogo>
    </>
  );
}
