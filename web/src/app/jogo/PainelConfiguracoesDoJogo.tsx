import { useEffect, useMemo, useRef, useState } from "react";
import type { ErrorCode } from "../../bridge/generated/ErrorCode";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import {
  ConfiguracoesDoJogo,
  type ChaveConfiguracaoJogo,
  type ConfiguracoesDoJogoRotulos,
  type ValorConfiguracaoJogo,
  type ValoresConfiguracoesDoJogo,
} from "../../components/ConfiguracoesDoJogo/ConfiguracoesDoJogo";
import { Dialogo } from "../../components/Dialogo/Dialogo";
import { gameSettingsStrings as s } from "../strings";
import { modeLabel } from "../settings/model";
import { panelModel, toChange, type PanelLabels } from "./configuracoesModel";
import { useGameSettings } from "./useGameSettings";

const v = s.values;

const PANEL_LABELS: PanelLabels = {
  noScreen: v.noScreen,
  keepCurrent: v.keepCurrent,
  keepAudio: v.keepAudio,
  on: v.on,
  off: v.off,
  disconnected: v.disconnected,
  mode: modeLabel,
  screenNote: s.notes.useScreen,
  whichScreenNote: s.notes.screen,
  launchNote: s.notes.launch,
  environmentNote: s.notes.environment,
  commandsNote: s.notes.commands,
  commandsBefore: s.commandsBefore,
  commandsAfter: s.commandsAfter,
  vrrUnsupported: s.notes.vrrUnsupported,
  audioLocked: s.notes.audioLocked,
};

const ROTULOS: ConfiguracoesDoJogoRotulos = {
  titulo: s.title,
  fechar: s.close,
  grupoTela: s.groups.screen,
  grupoSom: s.groups.sound,
  grupoAbrirOJogo: s.groups.launch,
  usarTela: s.rows.useScreen,
  monitor: s.rows.screen,
  resolucaoETaxa: s.rows.mode,
  hdr: s.rows.hdr,
  vrr: s.rows.vrr,
  saidaDeAudio: s.rows.audio,
  argumentos: s.rows.arguments,
  pastaDeTrabalho: s.rows.folder,
  variaveisDeAmbiente: s.rows.environment,
  comandosAntesEDepois: s.rows.commands,
  usarTudoDoTako: s.reset.action,
  descricaoUsarTudoDoTako: s.reset.description,
  padraoPrefixo: v.default,
  nenhum: v.none,
  nenhuma: v.noneFeminine,
  ligado: v.on,
  desligado: v.off,
  singularMudanca: s.changes.one,
  pluralMudancas: s.changes.other,
  nenhumaMudanca: s.changes.none,
};

const TEXTO_DO_VAZIO = {
  argumentos: v.none,
  pastaDeTrabalho: v.executableFolder,
  variaveisDeAmbiente: v.noneFeminine,
  comandosAntesEDepois: v.none,
} as const;

/** O texto do aviso para uma gravação que a ponte recusou ou que falhou. */
function failureText(key: ChaveConfiguracaoJogo, value: ValorConfiguracaoJogo, code: ErrorCode): string {
  if (key === "pastaDeTrabalho" && code === "not_found") return s.folderMissing(String(value).trim());
  if (key === "variaveisDeAmbiente" && code === "bad_request") return s.invalidEnvironment;
  if ((key === "argumentos" || key === "pastaDeTrabalho") && code === "bad_request") return s.invalid;
  return s.saveError;
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
  const { view, loadFailed, save } = useGameSettings(gameId, open);
  const [pending, setPending] = useState<{ values: ValoresConfiguracoesDoJogo; tokens: Partial<Record<ChaveConfiguracaoJogo, number>> }>({
    values: {},
    tokens: {},
  });
  const [confirming, setConfirming] = useState(false);
  const counter = useRef(0);
  const model = useMemo(() => (view ? panelModel(view, PANEL_LABELS) : null), [view]);

  useEffect(() => {
    if (!open || !loadFailed || view) return;
    onError(s.loadError);
    onClose();
  }, [open, loadFailed, view, onError, onClose]);

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
    if (!result.ok) onError(failureText(key, value, result.code));
  };

  const reset = async () => {
    setConfirming(false);
    const result = await save({ field: "reset_all" });
    if (!result.ok) onError(s.saveError);
  };

  return (
    <>
      <ConfiguracoesDoJogo
        open={open}
        onClose={onClose}
        tituloDoJogo={title}
        {...(coverUrl ? { capaUrl: coverUrl } : {})}
        valores={{ ...model.valores, ...pending.values }}
        padroes={model.padroes}
        opcoes={model.opcoes}
        descricoes={model.descricoes}
        rotulos={ROTULOS}
        mudancasCount={model.mudancas}
        somenteLeitura={model.somenteLeitura}
        textoDoVazio={TEXTO_DO_VAZIO}
        onChange={(key, value) => void change(key, value)}
        onResetarAoPadrao={() => {
          if (model.mudancas > 0) setConfirming(true);
        }}
      />
      <Dialogo
        open={confirming}
        onClose={() => {
          setConfirming(false);
        }}
        title={s.reset.title}
        closeLabel={s.close}
        size="s"
        actions={
          <>
            <BotaoDesktop
              variant="outlined"
              onClick={() => {
                setConfirming(false);
              }}
            >
              {s.reset.cancel}
            </BotaoDesktop>
            <BotaoDesktop variant="primary" onClick={() => void reset()}>
              {s.reset.action}
            </BotaoDesktop>
          </>
        }
      >
        <p>{s.reset.text}</p>
      </Dialogo>
    </>
  );
}
