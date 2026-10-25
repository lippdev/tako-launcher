import { useState } from "react";
import { call } from "../../bridge";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { Dialogo } from "../../components/Dialogo/Dialogo";
import { useT } from "../../i18n";

interface DialogoRemoverJogoProps {
  gameId: string;
  title: string;
  open: boolean;
  onClose: () => void;
  /** O núcleo tirou o jogo da biblioteca; a página dele deixa de existir e quem a abriu volta à Visão geral. */
  onRemoved: () => void;
  onError: (text: string) => void;
}

/** "Remover {jogo}?" (port do `confirm_remove` do Slint): o jogo sai do Tako; os arquivos no disco ficam onde estão. */
export function DialogoRemoverJogo({ gameId, title, open, onClose, onRemoved, onError }: DialogoRemoverJogoProps) {
  const t = useT();
  const [busy, setBusy] = useState(false);

  const remove = async () => {
    if (busy) return;
    setBusy(true);
    try {
      await call("game.remove", { gameId });
      onRemoved();
    } catch (failure) {
      console.error("game.remove", failure);
      onError(t("edit_game.remove_error"));
      onClose();
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialogo
      open={open}
      onClose={onClose}
      title={t("options.remove_title", { game: title })}
      closeLabel={t("common.close")}
      size="s"
      actions={
        <>
          <BotaoDesktop variant="outlined" onClick={onClose}>
            {t("common.cancel")}
          </BotaoDesktop>
          <BotaoDesktop disabled={busy} onClick={() => void remove()}>
            {t("options.remove")}
          </BotaoDesktop>
        </>
      }
    >
      <p>{t("options.remove_text")}</p>
    </Dialogo>
  );
}
