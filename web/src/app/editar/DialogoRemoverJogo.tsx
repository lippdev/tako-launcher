import { useState } from "react";
import { call } from "../../bridge";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { Dialogo } from "../../components/Dialogo/Dialogo";
import { editGameStrings as s } from "../strings";

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
  const [busy, setBusy] = useState(false);

  const remove = async () => {
    if (busy) return;
    setBusy(true);
    try {
      await call("game.remove", { gameId });
      onRemoved();
    } catch (failure) {
      console.error("game.remove", failure);
      onError(s.removeError);
      onClose();
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialogo
      open={open}
      onClose={onClose}
      title={s.removeTitle(title)}
      closeLabel={s.close}
      size="s"
      actions={
        <>
          <BotaoDesktop variant="outlined" onClick={onClose}>
            {s.cancel}
          </BotaoDesktop>
          <BotaoDesktop disabled={busy} onClick={() => void remove()}>
            {s.removeAction}
          </BotaoDesktop>
        </>
      }
    >
      <p>{s.removeText}</p>
    </Dialogo>
  );
}
