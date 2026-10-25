import { useState } from "react";
import { Aviso } from "../../components/Aviso/Aviso";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { Dialogo } from "../../components/Dialogo/Dialogo";
import { ErroDoJogo } from "../../components/ErroDoJogo/ErroDoJogo";
import { useT } from "../../i18n";
import { imageUrl } from "../jogo/jogoModel";
import { useDevImages } from "../jogo/useDevImages";
import { useGameDetails } from "../jogo/useGameDetails";
import { errorTexts, welcomeText, type EndNotice } from "./sessionModel";
import { useGameSession, type PlayFeedback } from "./useGameSession";
import styles from "./CamadasDaSessao.module.css";

/** Quanto o aviso de abertura (ferramenta que não abriu, comando que falhou) fica na tela. */
const WARNING_MS = 6000;

type WelcomeNotice = Extract<EndNotice, { kind: "welcome" }>;
type ErrorNotice = Extract<EndNotice, { kind: "error" }>;

/**
 * As camadas da sessão de jogo, por cima da interface inteira (WEB-21): "Fechando o jogo…", o "bem-vindo de volta" e o
 * diálogo de erro do jogo (SESSION-003), mais os avisos da abertura. O estado está em `useGameSession`; aqui só se
 * desenha. Cada camada guarda o último conteúdo enquanto sai, para o texto não sumir antes da animação acabar.
 */
export function CamadasDaSessao() {
  const t = useT();
  const session = useGameSession();
  const { running, state } = session;

  // O diálogo "Fechando…" sai animado depois que a sessão some: guarda o jogo para o título não esvaziar.
  const [closingGame, setClosingGame] = useState("");
  if (running && running.title !== closingGame) setClosingGame(running.title);

  const [welcome, setWelcome] = useState<WelcomeNotice | null>(null);
  const [failure, setFailure] = useState<ErrorNotice | null>(null);
  const notice = state.notice;
  if (notice?.kind === "welcome" && welcome?.id !== notice.id) setWelcome(notice);
  if (notice?.kind === "error" && failure?.id !== notice.id) setFailure(notice);

  // "Tentar de novo" do diálogo de erro: o aviso do pedido de abrir (abrindo, outro jogo aberto, falha).
  const [retry, setRetry] = useState<{ feedback: PlayFeedback; key: number } | null>(null);
  const [retryOpen, setRetryOpen] = useState(false);

  const [warning, setWarning] = useState<{ text: string; key: number } | null>(null);
  if (state.warnings.length > 0 && warning?.key !== state.counter) {
    setWarning({ text: state.warnings.join(" "), key: state.counter });
  }

  return (
    <>
      <Dialogo
        open={session.closingDialogOpen}
        onClose={session.dismissClosing}
        title={t("session.closing", { game: closingGame })}
        closeLabel={t("common.close")}
        actions={
          <>
            {session.closingStuck && (
              <BotaoDesktop
                variant="outlined"
                onClick={() => {
                  void session.end(true);
                }}
              >
                {t("session.force_close")}
              </BotaoDesktop>
            )}
            <BotaoDesktop
              icon="gamepad"
              size="sm"
              onClick={() => {
                session.dismissClosing();
                void session.resume();
              }}
            >
              {t("cc.back_to_game")}
            </BotaoDesktop>
          </>
        }
      >
        <p className={styles["texto"]} role="status">
          {t(session.closingStuck ? "session.closing_stuck" : "session.closing_wait")}
        </p>
      </Dialogo>

      <Aviso
        open={notice?.kind === "welcome"}
        tone="tako"
        noticeKey={welcome?.id ?? 0}
        onClose={session.dismissNotice}
      >
        {welcome ? welcomeText(t, welcome) : ""}
      </Aviso>

      <Aviso
        open={state.warnings.length > 0}
        tone="error"
        duration={WARNING_MS}
        noticeKey={warning?.key ?? 0}
        onClose={session.dismissWarnings}
      >
        {warning?.text ?? ""}
      </Aviso>

      <Aviso
        open={retryOpen}
        tone={retry?.feedback.tone ?? "info"}
        noticeKey={retry?.key ?? 0}
        onClose={() => {
          setRetryOpen(false);
        }}
      >
        {retry?.feedback.text ?? ""}
      </Aviso>

      {failure && (
        <AvisoDeErro
          notice={failure}
          open={notice?.kind === "error"}
          onClose={session.dismissNotice}
          onRetry={() => {
            session.dismissNotice();
            void session.play({ id: failure.gameId, title: failure.title }).then((feedback) => {
              setRetry({ feedback, key: Date.now() });
              setRetryOpen(true);
            });
          }}
        />
      )}
    </>
  );
}

/** O diálogo de erro do jogo: a capa vem dos detalhes, o código de saída do que o núcleo explicou (SESSION-003). */
function AvisoDeErro({
  notice,
  open,
  onClose,
  onRetry,
}: {
  notice: ErrorNotice;
  open: boolean;
  onClose: () => void;
  onRetry: () => void;
}) {
  const t = useT();
  const devImage = useDevImages();
  const { details } = useGameDetails(notice.gameId);
  const texts = errorTexts(t, notice);
  const cover = details?.game.hasCover
    ? (devImage?.("cover", notice.gameId, notice.title) ??
      imageUrl("cover", notice.gameId, 80 * window.devicePixelRatio, 120 * window.devicePixelRatio))
    : undefined;
  return (
    <ErroDoJogo
      open={open}
      gameTitle={texts.title}
      {...(texts.subtitle ? { subtitle: texts.subtitle } : {})}
      {...(cover ? { coverUrl: cover } : {})}
      exitCode={texts.exitCode}
      message={texts.message}
      labels={{
        close: t("common.close"),
        technicalDetails: t("game_page.technical_details"),
        saveReport: t("session.save_report"),
        repair: t("session.repair"),
        retry: t("session.try_again"),
      }}
      onRetry={onRetry}
      onClose={onClose}
    />
  );
}
