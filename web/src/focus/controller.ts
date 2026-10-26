// Controle pelo núcleo (WEB-05b): o SDL3 roda no processo do núcleo, que já aplica o perfil de botões, a zona morta e
// a repetição do app atual e manda só o resultado pela ponte. Aqui os comandos entram no mesmo ponto do teclado
// (`dispatchNavAction`), e o estado do controle (família dos botões, bateria) e os avisos ficam para a tela.

import { useSyncExternalStore } from "react";
import { call, on } from "../bridge";
import type { ControllerInfo } from "../bridge/generated/ControllerInfo";
import type { InputNotice } from "../bridge/generated/InputNotice";
import { shouldDropEcho } from "./echo";
import { dispatchNavAction } from "./engine";
import { actionForInput } from "./keymap";

const NONE: ControllerInfo = { connected: false, count: 0, style: "xbox", name: "", battery: null };

let info: ControllerInfo = NONE;
/** Muda a cada `input.controller`: a resposta da consulta que chega depois de um evento já está velha. */
let version = 0;
const subscribers = new Set<() => void>();
const noticeListeners = new Set<(notice: InputNotice) => void>();

function setInfo(next: ControllerInfo): void {
  info = next;
  // `data-pad` na raiz: o CSS escolhe os glifos (xbox, playstation, nintendo); sem controle, some.
  if (next.connected) document.documentElement.dataset["pad"] = next.style;
  else delete document.documentElement.dataset["pad"];
  subscribers.forEach((fn) => {
    fn();
  });
}

/**
 * Liga o controle: comandos para o navegador de foco, estado e avisos. Pergunta o estado ao abrir (a interface
 * reabre depois do jogo e o evento só chega quando algo muda). Devolve a função que desliga.
 */
export function startController(): () => void {
  const offs = [
    on("input.nav", (nav) => {
      const action = actionForInput(nav.action);
      if (shouldDropEcho(action, "controller")) return;
      dispatchNavAction(action, "controller", nav.repeat);
    }),
    on("input.controller", (next) => {
      version += 1;
      setInfo(next);
    }),
    on("input.notice", (notice) => {
      noticeListeners.forEach((fn) => {
        fn(notice);
      });
    }),
  ];
  const asked = version;
  call("input.controller").then(
    (next) => {
      if (version === asked) setInfo(next);
    },
    () => {
      // Hospedeiro sem controle (falso antigo, teste): fica "sem controle".
    },
  );
  return () => {
    offs.forEach((off) => {
      off();
    });
  };
}

function subscribe(listener: () => void): () => void {
  subscribers.add(listener);
  return () => subscribers.delete(listener);
}

/** O controle em uso: conectado, família dos botões (para ícones e dicas), nome e bateria. */
export function useController(): ControllerInfo {
  return useSyncExternalStore(subscribe, () => info);
}

/** Escuta os avisos do controle (desconectado, de volta, bateria fraca). Devolve a função que para de escutar. */
export function onControllerNotice(listener: (notice: InputNotice) => void): () => void {
  noticeListeners.add(listener);
  return () => noticeListeners.delete(listener);
}
