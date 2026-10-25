// A parte que toca o DOM: junta os candidatos, escolhe com `spatial.ts`, move o foco e guarda o último foco de cada
// região. O contrato com os componentes está em `README.md` desta pasta.

import { closeTopLayer, hasOpenLayer } from "../lib/escapeLayers";
import { FOCUSABLE } from "../lib/focusTrap";
import { keyForAction, type NavAction } from "./keymap";
import { choose, entry, type Candidate, type Direction, type Rect } from "./spatial";

/** Quem entregou a última ação: o foco só aparece para teclado e controle, nunca depois do mouse. */
export type InputSource = "keyboard" | "controller" | "mouse";

const REGION = "data-focus-region";
const GROUP = "data-focus-group";
/** Camadas que prendem a navegação: diálogos, painéis (`aria-modal`) e menus. */
const LAYER = '[role="dialog"][aria-modal="true"], [role="menu"]';
/** Quem muda de seção com Q e E: `tablist` e o que se marcar com `data-focus-sections`. */
const SECTIONS = '[role="tablist"], [data-focus-sections]';

/** Evento dos comandos do controle sem tecla (X, Menu): `detail.command` é `favorite` ou `menu`. */
export const COMMAND_EVENT = "tako:command";

type Focusable = HTMLElement;

/** Último foco de cada região. */
const memory = new Map<string, Focusable>();
let lastRegion = "";
let lastFocused: Focusable | null = null;
let source: InputSource = "mouse";
let backHandler: (() => boolean) | null = null;

export function inputSource(): InputSource {
  return source;
}

/** Guarda a origem da entrada em `data-input` na raiz (o CSS e quem depura leem dali). */
export function setInputSource(next: InputSource): void {
  source = next;
  document.documentElement.dataset["input"] = next;
}

/**
 * Quem sabe o que "voltar" significa na tela atual (fechar a página do jogo, sair das Configurações). Roda quando
 * nenhuma camada, campo ou componente tratou o "voltar"; devolve `true` se fez algo. Devolve a função que desfaz.
 */
export function setBackHandler(handler: () => boolean): () => void {
  backHandler = handler;
  return () => {
    if (backHandler === handler) backHandler = null;
  };
}

function regionOf(el: Element): string {
  return el.closest(`[${REGION}]`)?.getAttribute(REGION) ?? "";
}

function visible(el: HTMLElement): boolean {
  if (!el.isConnected || el.closest("[inert]")) return false;
  if (typeof el.checkVisibility === "function" && !el.checkVisibility({ visibilityProperty: true })) return false;
  return el.getClientRects().length > 0;
}

function toRect(r: DOMRect): Rect {
  return { left: r.left, top: r.top, width: r.width, height: r.height };
}

/** Campo em que setas, letras e Backspace são do texto. */
export function isTextEntry(el: Element | null): boolean {
  if (el instanceof HTMLTextAreaElement) return true;
  if (el instanceof HTMLInputElement) return ["text", "search", "email", "url", "tel", "password", "number"].includes(el.type);
  return el instanceof HTMLElement && el.isContentEditable;
}

/** A camada que prende o foco agora, ou `null` se a tela está livre. */
function currentLayer(): HTMLElement | null {
  if (!hasOpenLayer()) return null;
  const active = document.activeElement;
  const around = active instanceof Element ? active.closest<HTMLElement>(LAYER) : null;
  if (around) return around;
  // Foco perdido com uma camada aberta: a última que está à vista.
  const layers = [...document.querySelectorAll<HTMLElement>(LAYER)].filter(visible);
  return layers[layers.length - 1] ?? null;
}

/** Tudo o que recebe foco por Tab dentro de `root` e está à vista. Quem tem `tabindex="-1"` (roving) fica de fora. */
function collect(root: ParentNode): Candidate<Focusable>[] {
  const found: Candidate<Focusable>[] = [];
  const groups = new Map<Element, Rect>();
  for (const el of root.querySelectorAll<Focusable>(FOCUSABLE)) {
    if (el.tabIndex < 0 || el.closest("[data-focus-skip]") || !visible(el)) continue;
    const rect = toRect(el.getBoundingClientRect());
    if (rect.width <= 0 || rect.height <= 0) continue;
    const group = el.closest(`[${GROUP}]`);
    let clipped = false;
    if (group) {
      let box = groups.get(group);
      if (!box) groups.set(group, (box = toRect(group.getBoundingClientRect())));
      clipped = rect.left + rect.width <= box.left + 1 || rect.left >= box.left + box.width - 1;
    }
    found.push({ item: el, rect, region: regionOf(el), ...(group ? { group, clipped } : {}) });
  }
  return found;
}

/** A caixa de cada região à vista (as de mesmo nome se juntam). */
function regionBoxes(): Map<string, Rect> {
  const boxes = new Map<string, Rect>();
  for (const el of document.querySelectorAll<HTMLElement>(`[${REGION}]`)) {
    if (!visible(el)) continue;
    const name = el.getAttribute(REGION) ?? "";
    const r = toRect(el.getBoundingClientRect());
    const before = boxes.get(name);
    if (!before) {
      boxes.set(name, r);
      continue;
    }
    const left = Math.min(before.left, r.left);
    const top = Math.min(before.top, r.top);
    boxes.set(name, {
      left,
      top,
      width: Math.max(before.left + before.width, r.left + r.width) - left,
      height: Math.max(before.top + before.height, r.top + r.height) - top,
    });
  }
  return boxes;
}

function reducedMotion(): boolean {
  return document.documentElement.dataset["reduceMotion"] === "true" || matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/** Leva o foco ao elemento e o põe na tela, com rolagem suave (instantânea com "Reduzir animações"). */
export function focusElement(el: Focusable): void {
  // `focusVisible` pede o anel mesmo sem tecla real (controle); o Chromium que não conhece a opção a ignora.
  el.focus({ preventScroll: true, focusVisible: true });
  el.scrollIntoView({ block: "nearest", inline: "nearest", behavior: reducedMotion() ? "auto" : "smooth" });
}

/** O primeiro foco quando nada está focado: a região do último foco, senão a marcada com `data-focus-initial`. */
function enter(layer: HTMLElement | null, anyElement = true): boolean {
  const all = collect(layer ?? document.body);
  const initial = document.querySelector(`[${REGION}][data-focus-initial]`)?.getAttribute(REGION);
  const regions = layer ? [""] : [lastRegion, initial ?? ""];
  for (const region of regions) {
    const target = entry(all, region, layer ? undefined : memory);
    if (target) {
      focusElement(target.item);
      return true;
    }
  }
  const first = anyElement ? all[0] : undefined;
  if (!first) return false;
  focusElement(first.item);
  return true;
}

/** Move o foco na direção. `false` se não há para onde ir (a borda). */
export function navigate(direction: Direction): boolean {
  const layer = currentLayer();
  const active = document.activeElement;
  const current = active instanceof HTMLElement && active !== document.body && (layer ?? document.body).contains(active) ? active : null;
  if (!current) return enter(layer);
  const group = current.closest(`[${GROUP}]`) ?? undefined;
  const choice = choose(
    toRect(current.getBoundingClientRect()),
    regionOf(current),
    group,
    collect(layer ?? document.body),
    direction,
    { current, memory: layer ? undefined : memory, regions: layer ? undefined : regionBoxes() },
  );
  if (!choice) return false;
  focusElement(choice.candidate.item);
  return true;
}

const NATIVE = "button, a[href], input, select, textarea, summary";
const CLICKABLE_ROLES = new Set(["button", "tab", "menuitem", "menuitemradio", "menuitemcheckbox", "switch", "option", "link", "checkbox", "radio"]);

/** Enter e Espaço já ativam sozinhos o que o navegador conhece; o resto (`div role="button"` sem teclado) precisa do clique. */
export function needsClick(el: Element): boolean {
  return !el.matches(NATIVE) && CLICKABLE_ROLES.has(el.getAttribute("role") ?? "");
}

/**
 * Entrega a tecla ao componente focado, como se ela tivesse sido apertada (o controle não gera tecla). `true` se ele
 * tratou (`preventDefault`): lista, abas, deslizante e menu continuam donos das próprias setas.
 */
function offer(key: string, init: KeyboardEventInit = {}): boolean {
  const target = document.activeElement ?? document.body;
  const event = new KeyboardEvent("keydown", { ...init, key, bubbles: true, cancelable: true });
  target.dispatchEvent(event);
  return event.defaultPrevented;
}

/** Aperta o elemento focado: o `Enter` primeiro (lista e capa tratam sozinhas), o clique se ninguém tratou. */
export function activate(): boolean {
  const el = document.activeElement;
  if (!(el instanceof HTMLElement) || el === document.body || isTextEntry(el)) return false;
  if (offer("Enter")) return true;
  el.click();
  return true;
}

/** Sai do campo de texto para o vizinho (de baixo, senão de cima); sem vizinho, só tira o foco. */
function leaveField(field: HTMLElement): void {
  if (!navigate("down") && !navigate("up")) field.blur();
}

/**
 * Volta um passo: fecha a camada de cima; senão sai do campo de texto; senão deixa a tela decidir
 * (`setBackHandler`). `false` se não havia nada a voltar.
 */
export function back(): boolean {
  if (closeTopLayer()) return true;
  const active = document.activeElement;
  if (active instanceof HTMLElement && isTextEntry(active)) {
    leaveField(active);
    return true;
  }
  return backHandler?.() ?? false;
}

/** As regiões extras que o conjunto de seções atende (`data-focus-sections="content"`): as categorias valem para as linhas ao lado. */
function servedRegions(container: HTMLElement): string[] {
  return (container.getAttribute("data-focus-sections") ?? "").split(/\s+/).filter(Boolean);
}

/** Troca de seção (Q e E, LB e RB): a aba ou categoria vizinha do conjunto mais próximo do foco. `false` na borda. */
export function switchSection(delta: -1 | 1): boolean {
  const layer = currentLayer();
  const root = layer ?? document.body;
  const containers = [...root.querySelectorAll<HTMLElement>(SECTIONS)].filter(visible);
  const active = document.activeElement;
  const container =
    (active instanceof Element ? containers.find((c) => c.contains(active)) : undefined) ??
    (lastRegion === "" ? undefined : containers.find((c) => regionOf(c) === lastRegion || servedRegions(c).includes(lastRegion)));
  if (!container) return false;
  const items = [...container.querySelectorAll<HTMLElement>(container.matches('[role="tablist"]') ? '[role="tab"]' : "button")].filter(visible);
  const index = items.findIndex((el) => el.getAttribute("aria-selected") === "true" || ["page", "true"].includes(el.getAttribute("aria-current") ?? ""));
  const target = items[Math.min(items.length - 1, Math.max(0, index + delta))];
  if (!target || (index >= 0 && target === items[index])) return false;
  target.click();
  // Voltar às categorias devolve a que ficou aberta, não a que foi visitada antes.
  const region = regionOf(container);
  if (region !== "") memory.set(region, target);
  if (active instanceof Element && container.contains(active)) focusElement(target);
  return true;
}

/** Y do controle: a busca geral, pelo mesmo Ctrl+K do teclado (o atalho é do `AppShell`). */
function openSearch(): boolean {
  return offer("k", { ctrlKey: true });
}

/**
 * X e Menu do controle: `tako:command` no elemento focado, subindo pela árvore. O componente que tem favorito ou menu
 * escuta e chama `preventDefault`; `false` se ninguém tratou.
 */
function sendCommand(command: "favorite" | "menu"): boolean {
  const target = document.activeElement ?? document.body;
  const event = new CustomEvent(COMMAND_EVENT, { detail: { command }, bubbles: true, cancelable: true });
  target.dispatchEvent(event);
  return event.defaultPrevented;
}

/**
 * O ponto único das ações. O teclado chega pelo `useSpatialNavigation`; o controle chega aqui, vindo da ponte
 * (`source: "controller"`, `controller.ts`), e passa antes pelo componente focado, como uma tecla. `repeat`: direção
 * mantida no controle; vai na tecla oferecida, para a lista e o deslizante não pularem a borda segurando.
 */
export function dispatchNavAction(action: NavAction, from: InputSource = "controller", repeat = false): boolean {
  setInputSource(from);
  if (from === "controller") {
    const key = keyForAction(action);
    if (key !== null && offer(key, { repeat })) return true;
  }
  switch (action.kind) {
    case "navigate":
      return navigate(action.direction);
    case "activate":
      return from === "controller" ? activate() : false;
    case "back":
      return back();
    case "section":
      return switchSection(action.delta);
    case "search":
      return openSearch();
    case "command":
      return sendCommand(action.command);
  }
}

/** Região e último foco: chamado a cada `focusin`. Camadas (região vazia) não entram na memória. */
export function remember(el: Focusable): void {
  lastFocused = el;
  const region = regionOf(el);
  if (region === "" || el.closest(LAYER)) return;
  lastRegion = region;
  memory.set(region, el);
}

/**
 * O elemento focado saiu da tela (a página do jogo fechou, a categoria trocou) e o foco caiu no `body`: leva-o de
 * volta ao último foco da região, ou ao primeiro dela. Só para teclado e controle; depois do mouse o foco solto é normal.
 */
export function recoverFocus(): void {
  if (source === "mouse" || lastFocused === null) return;
  const active = document.activeElement;
  if (active && active !== document.body) return;
  // Sem nada na região ainda (a tela nova está chegando), espera: a próxima mudança do DOM tenta de novo.
  enter(currentLayer(), false);
}
