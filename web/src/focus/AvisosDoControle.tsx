import { useEffect, useState } from "react";
import { Aviso } from "../components/Aviso/Aviso";
import type { InputNotice } from "../bridge/generated/InputNotice";
import { useT, type TFunction } from "../i18n";
import { onControllerNotice } from "./controller";

function text(t: TFunction, notice: InputNotice): string {
  switch (notice.kind) {
    case "connected":
      return t("status.pad_connected");
    case "disconnected":
      return t("status.pad_disconnected");
    case "battery_low":
      return t("status.battery_low", { pad: notice.pad ?? "", percent: String(notice.percent ?? "") });
  }
}

/**
 * Os avisos do controle, como no app atual (INPUT-003, INPUT-004): desconectado, conectado de volta e bateria fraca. O
 * núcleo decide quando avisar (nada durante o jogo); aqui só o texto do catálogo e o `Aviso`. Uma vez, no `AppShell`.
 */
export function AvisosDoControle() {
  const t = useT();
  const [notice, setNotice] = useState<{ notice: InputNotice; key: number } | null>(null);
  const [open, setOpen] = useState(false);

  useEffect(
    () =>
      onControllerNotice((next) => {
        setNotice((before) => ({ notice: next, key: (before?.key ?? 0) + 1 }));
        setOpen(true);
      }),
    [],
  );

  return (
    <Aviso
      open={open}
      tone="info"
      icon="gamepad"
      noticeKey={notice?.key ?? 0}
      onClose={() => {
        setOpen(false);
      }}
    >
      {notice ? text(t, notice.notice) : ""}
    </Aviso>
  );
}
