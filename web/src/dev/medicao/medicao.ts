// Medição de desempenho dentro da página (WEB-60b). Não faz parte do produto.
//
// Só roda quando o servidor de medição (`vite.medicao.config.mjs`) injeta este arquivo no `index.html` e a URL traz
// `?medicao=<cenário>`. O `npm run build` normal nunca o importa; mesmo que alguém importe, fora do `DEV` e do modo
// `medicao` o módulo não faz nada. Os resultados vão por `POST /__medicao` ao coletor do mesmo servidor, que grava
// uma linha JSON por evento; `scripts/measure-web.ps1` lê esse arquivo.
//
// Tempos absolutos são `performance.timeOrigin + performance.now()` (ms desde 1970, o mesmo relógio do
// `DateTimeOffset.UtcNow` do script), para somar com o que o script mede fora da página.

import { call, on } from "../../bridge";
import { defaultScreenState, serializeScreenState } from "../../app/uiState";
import { t } from "../../i18n";
import { coverChoice, finishChoice } from "../../app/primeiraVez/model";

const ATIVO = import.meta.env.DEV || import.meta.env.MODE === "medicao";
const consulta = new URLSearchParams(window.location.search);
const cenario = consulta.get("medicao");

/** Quadros por segundo exigidos pela meta (p95 do tempo de quadro ≤ 1000 / 58). */
const META_FPS = 58;
const LIMITE_QUADRO_MS = 1000 / META_FPS;

// O contador de pedidos de quadro tem de ser instalado antes do código do app: este módulo vai primeiro no `<head>`.
// As sondas daqui usam o original e não entram na conta.
const pedirQuadro = window.requestAnimationFrame.bind(window);
let pedidosDeQuadro = 0;

function agora(): number {
  return performance.timeOrigin + performance.now();
}

function enviar(evento: string, dados: Record<string, unknown> = {}): void {
  const corpo = JSON.stringify({ evento, em: agora(), ...dados });
  fetch("/__medicao", { method: "POST", body: corpo, keepalive: true }).catch(() => {
    // Sem coletor não há o que fazer; a medição só perde a linha.
  });
}

function esperar(ms: number): Promise<void> {
  return new Promise((resolve) => {
    window.setTimeout(resolve, ms);
  });
}

function percentil(ordenados: readonly number[], p: number): number {
  if (ordenados.length === 0) return 0;
  const i = Math.min(ordenados.length - 1, Math.max(0, Math.ceil((p / 100) * ordenados.length) - 1));
  return ordenados[i] ?? 0;
}

function arredondar(valor: number): number {
  return Math.round(valor * 100) / 100;
}

// --- A biblioteca interativa --------------------------------------------------------------------------------

function listaDeJogos(): HTMLElement | null {
  return document.querySelector<HTMLElement>('[data-focus-region="list"] [role="listbox"]');
}

/** Lista com itens, sem carregar e aceitando foco. Não olha se algo a cobre (ver `livre`). */
function listaNaTela(): HTMLElement | null {
  const lista = listaDeJogos();
  if (!lista || lista.getAttribute("aria-busy") === "true" || lista.closest("[inert]")) return null;
  if (lista.tabIndex !== 0 || !lista.querySelector('[role="option"]')) return null;
  lista.focus({ preventScroll: true });
  return document.activeElement === lista ? lista : null;
}

/** O primeiro item aparece por cima de tudo: a entrada do app (o polvo, ~1 s) já saiu e nada o encobre. */
function livre(lista: HTMLElement): boolean {
  const item = lista.querySelector('[role="option"]');
  if (!item) return false;
  const caixa = item.getBoundingClientRect();
  if (caixa.width === 0 || caixa.height === 0) return false;
  const topo = document.elementFromPoint(caixa.left + caixa.width / 2, caixa.top + caixa.height / 2);
  return topo !== null && item.contains(topo);
}

/**
 * Onde clicar com o mouse de verdade (o script usa o centro do primeiro item para abrir um jogo na construção de
 * release, onde não há página de medição) e onde a janela está.
 */
function geometria(lista: HTMLElement): Record<string, number> {
  const caixa = (lista.querySelector('[role="option"]') ?? lista).getBoundingClientRect();
  return {
    itemX: Math.round(caixa.left + caixa.width / 2),
    itemY: Math.round(caixa.top + caixa.height / 2),
    larguraInterna: window.innerWidth,
    alturaInterna: window.innerHeight,
    telaX: window.screenX,
    telaY: window.screenY,
    escala: window.devicePixelRatio,
  };
}

/** Espera os dois marcos e devolve o elemento da lista. Um observador do DOM, sem laço de quadros. */
function esperarBiblioteca(): Promise<HTMLElement> {
  return new Promise((resolve) => {
    let naTela = false;
    const conferir = () => {
      const lista = listaNaTela();
      if (!lista) return;
      if (!naTela) {
        naTela = true;
        performance.mark("tako:biblioteca-na-tela");
        enviar("biblioteca-na-tela", { desdeNavegacaoMs: arredondar(performance.now()) });
      }
      if (livre(lista)) {
        observador.disconnect();
        window.clearInterval(relogio);
        performance.mark("tako:biblioteca-livre");
        enviar("biblioteca-livre", { desdeNavegacaoMs: arredondar(performance.now()) });
        enviar("geometria", geometria(lista));
        resolve(lista);
      }
    };
    // Se a biblioteca não chega, o relatório diz o que a página mostra (assistente da primeira vez, leitura, erro…).
    window.setTimeout(() => {
      if (naTela) return;
      const lista = listaDeJogos();
      enviar("biblioteca-demora", {
        texto: (document.body.innerText || "").replace(/\s+/g, " ").slice(0, 300),
        itens: document.querySelectorAll('[role="option"]').length,
        lista: lista ? { ocupada: lista.getAttribute("aria-busy"), tabIndex: lista.tabIndex } : null,
      });
    }, 8000);
    const observador = new MutationObserver(conferir);
    observador.observe(document.documentElement, { subtree: true, childList: true, attributes: true });
    // A saída da entrada do app é um tempo de CSS; nenhuma mutação a anuncia.
    const relogio = window.setInterval(conferir, 50);
    conferir();
  });
}

// --- Repouso ------------------------------------------------------------------------------------------------

interface AmostraDeRepouso {
  pedidosDeQuadro: number;
  amostrasDeAnimacao: number;
  amostrasComAnimacao: number;
  /** Quem mais mexeu no DOM parado: "tipo:alvo:atributo" e quantas vezes. */
  maioresMutacoes: Record<string, number>;
  maximoDeAnimacoes: number;
  animacoes: string[];
  mutacoes: number;
  tarefasLongas: number;
}

/**
 * Parado por `ms`: o que a página pede ao compositor sem ninguém mexer. Não roda laço de `requestAnimationFrame`
 * (isso forçaria quadros e estragaria a medição): conta os pedidos do app, as animações CSS em curso, mutações
 * do DOM e tarefas longas.
 */
async function observarRepouso(ms: number): Promise<AmostraDeRepouso> {
  const inicioDosPedidos = pedidosDeQuadro;
  let mutacoes = 0;
  const porAlvo = new Map<string, number>();
  const mutador = new MutationObserver((lista) => {
    mutacoes += lista.length;
    for (const m of lista) {
      const alvo = m.target instanceof Element ? m.target : m.target.parentElement;
      const nome = alvo ? `${alvo.tagName.toLowerCase()}.${alvo.classList[0] ?? alvo.id}` : "?";
      const chave = `${m.type}:${nome}:${m.attributeName ?? ""}`;
      porAlvo.set(chave, (porAlvo.get(chave) ?? 0) + 1);
    }
  });
  mutador.observe(document.documentElement, { subtree: true, childList: true, attributes: true, characterData: true });
  let tarefasLongas = 0;
  const longas = new PerformanceObserver((lista) => {
    tarefasLongas += lista.getEntries().length;
  });
  try {
    longas.observe({ type: "longtask" });
  } catch {
    // Sem suporte: fica zero e o relatório diz que não mediu.
  }
  const nomes = new Set<string>();
  let amostras = 0;
  let comAnimacao = 0;
  let maximo = 0;
  const fim = performance.now() + ms;
  while (performance.now() < fim) {
    const correndo = document.getAnimations().filter((a) => a.playState === "running");
    amostras += 1;
    if (correndo.length > 0) comAnimacao += 1;
    maximo = Math.max(maximo, correndo.length);
    for (const animacao of correndo) {
      const alvo = animacao.effect instanceof KeyframeEffect ? animacao.effect.target : null;
      const nome =
        animacao instanceof CSSAnimation
          ? animacao.animationName
          : animacao instanceof CSSTransition
            ? animacao.transitionProperty
            : (animacao.id || "waapi");
      nomes.add(`${nome}@${alvo ? alvo.tagName.toLowerCase() : "?"}`);
    }
    await esperar(250);
  }
  mutador.disconnect();
  longas.disconnect();
  return {
    pedidosDeQuadro: pedidosDeQuadro - inicioDosPedidos,
    amostrasDeAnimacao: amostras,
    amostrasComAnimacao: comAnimacao,
    maioresMutacoes: Object.fromEntries([...porAlvo].sort((a, b) => b[1] - a[1]).slice(0, 8)),
    maximoDeAnimacoes: maximo,
    animacoes: [...nomes].slice(0, 20),
    mutacoes,
    tarefasLongas,
  };
}

async function cenarioRepouso(lista: HTMLElement): Promise<void> {
  const ms = Number(consulta.get("ms") ?? "30000");
  const assentar = Number(consulta.get("assentar") ?? "3000");
  // Com `clique=os` quem abre o jogo é o script, com o mouse do sistema (o mesmo gesto que ele faz no release).
  const cliqueDoSistema = consulta.get("clique") === "os";
  const estados: { nome: string; preparar: () => Promise<void> }[] = [
    { nome: "visao-geral", preparar: () => Promise.resolve() },
    {
      nome: "pagina-do-jogo",
      preparar: async () => {
        if (!cliqueDoSistema) {
          lista.querySelector<HTMLElement>('[role="option"]')?.click();
          return;
        }
        enviar("aguardando-clique", {});
        while (!lista.querySelector('[role="option"][aria-current="true"]')) await esperar(100);
        enviar("clique-recebido", {});
      },
    },
  ];
  for (const estado of estados) {
    await estado.preparar();
    await esperar(assentar);
    // O script mede CPU e GPU do sistema operacional exatamente nesta janela.
    enviar("repouso-janela", { estado: estado.nome, ms });
    const medida = await observarRepouso(ms);
    enviar("repouso", { estado: estado.nome, ms, ...medida });
  }
}

// --- Quadros (fps) ------------------------------------------------------------------------------------------

/** Mede o tempo entre quadros enquanto `acao` mexe na tela por `ms`. Aqui o laço de quadros é a medição. */
async function medirQuadros(
  rotulo: string,
  ms: number,
  acao: () => Promise<Record<string, unknown>>,
): Promise<void> {
  const tempos: number[] = [];
  let ultimo = 0;
  let vivo = true;
  const laco = (t: number) => {
    if (ultimo !== 0) tempos.push(t - ultimo);
    ultimo = t;
    if (vivo) pedirQuadro(laco);
  };
  pedirQuadro(laco);
  const inicio = performance.now();
  const [verificacao] = await Promise.all([acao(), esperar(ms)]);
  vivo = false;
  const duracao = performance.now() - inicio;
  const ordenados = [...tempos].sort((a, b) => a - b);
  const lentos = ordenados.filter((t) => t > LIMITE_QUADRO_MS * 1.5).length;
  enviar("quadros", {
    rotulo,
    duracaoMs: arredondar(duracao),
    quadros: tempos.length,
    fpsMedio: arredondar((tempos.length / duracao) * 1000),
    p50Ms: arredondar(percentil(ordenados, 50)),
    p95Ms: arredondar(percentil(ordenados, 95)),
    p99Ms: arredondar(percentil(ordenados, 99)),
    maxMs: arredondar(ordenados.at(-1) ?? 0),
    acimaDaMeta: ordenados.filter((t) => t > LIMITE_QUADRO_MS).length,
    // Quadro com mais de 1,5 vez o orçamento: pelo menos um quadro perdido.
    perdidos: lentos,
    // Taxa de atualização estimada do monitor (a mediana dos intervalos).
    hzEstimado: arredondar(1000 / Math.max(1, percentil(ordenados, 50))),
    metaFps: META_FPS,
    // O que a ação conseguiu fazer de fato (rolou, abriu a página, abriu a busca…): sem isso o tempo de quadro
    // poderia ser o de uma tela parada.
    verificacao,
  });
}

function rolagemDaLista(lista: HTMLElement): HTMLElement {
  return lista.scrollHeight > lista.clientHeight + 1 ? lista : (lista.parentElement ?? lista);
}

async function cenarioFps(lista: HTMLElement): Promise<void> {
  const assentar = Number(consulta.get("assentar") ?? "2000");
  await esperar(assentar);

  // 1. Rolagem da biblioteca: a rolagem suave do navegador, a mesma de uma roda de mouse.
  const rolagem = rolagemDaLista(lista);
  if (rolagem.scrollHeight > rolagem.clientHeight + 1) {
    await medirQuadros("rolagem-da-biblioteca", 6000, async () => {
      let percorrido = 0;
      let anterior = rolagem.scrollTop;
      const ao = () => {
        percorrido += Math.abs(rolagem.scrollTop - anterior);
        anterior = rolagem.scrollTop;
      };
      rolagem.addEventListener("scroll", ao, { passive: true });
      for (let i = 0; i < 8; i += 1) {
        rolagem.scrollBy({ top: i < 4 ? 700 : -700, behavior: "smooth" });
        await esperar(700);
      }
      rolagem.removeEventListener("scroll", ao);
      return { percorridoPx: Math.round(percorrido), alturaTotalPx: rolagem.scrollHeight };
    });
  } else {
    enviar("quadros-nao-medido", { rotulo: "rolagem-da-biblioteca", motivo: "a biblioteca cabe na tela" });
  }

  // 2. Troca de jogo: clicar em itens seguidos abre a página de cada um (fundo e vista animam).
  const itens = [...lista.querySelectorAll<HTMLElement>('[role="option"]')].slice(0, 8);
  await medirQuadros("troca-de-jogo", 8500, async () => {
    let confirmados = 0;
    for (const item of itens) {
      item.click();
      await esperar(1000);
      if (item.getAttribute("aria-current") === "true") confirmados += 1;
    }
    return { cliques: itens.length, paginasAbertas: confirmados };
  });

  // 3. Vidro sobre o conteúdo: a busca geral (Ctrl+K, cortina com desfoque e painel de vidro).
  const tecla = (key: string, ctrl = false) => {
    const alvo = document.activeElement ?? document.body;
    alvo.dispatchEvent(new KeyboardEvent("keydown", { key, ctrlKey: ctrl, bubbles: true, cancelable: true }));
  };
  await medirQuadros("vidro-busca-geral", 8000, async () => {
    let abertas = 0;
    for (let i = 0; i < 4; i += 1) {
      tecla("k", true);
      await esperar(1000);
      if (document.querySelector('[role="dialog"] [role="combobox"]')) abertas += 1;
      tecla("Escape");
      await esperar(1000);
    }
    return { aberturas: 4, buscaAbertaEmCadaUma: abertas };
  });

  // 4. Vidro sobre o conteúdo: a vista Perfil (barra de cima e blocos de vidro) e o caminho de volta.
  const perfil = document.querySelector<HTMLElement>(`button[aria-label="${t("profile.tab")}"]`);
  if (perfil) {
    await medirQuadros("vista-perfil", 6000, async () => {
      let vistas = 0;
      for (let i = 0; i < 3; i += 1) {
        perfil.click();
        await esperar(1000);
        if (document.querySelector(`section[aria-label="${t("profile.tab")}"]:not([inert])`)) vistas += 1;
        tecla("Escape");
        await esperar(1000);
      }
      return { aberturas: 3, vistaPerfilVisivel: vistas };
    });
  } else {
    enviar("quadros-nao-medido", { rotulo: "vista-perfil", motivo: "botão Perfil não encontrado" });
  }
}

// --- Latência de teclado ------------------------------------------------------------------------------------

/**
 * Teclas reais (o script as manda pelo sistema operacional, `SendInput`) e o tempo até o próximo quadro: Event
 * Timing (`duration` = do `timeStamp` do evento até a pintura seguinte, arredondado a 8 ms) e o primeiro quadro
 * depois do evento. Eventos abaixo do limiar de 16 ms não aparecem no Event Timing; contam como "menos de 16".
 */
function cenarioLatencia(): void {
  const duracoes: number[] = [];
  const atrasosDeEntrada: number[] = [];
  const ateQuadro: number[] = [];
  let teclas = 0;
  let ultima = 0;
  const limiar = 16;
  new PerformanceObserver((lista) => {
    for (const entrada of lista.getEntries()) {
      if (entrada.name !== "keydown") continue;
      const evento = entrada as PerformanceEventTiming;
      duracoes.push(evento.duration);
      atrasosDeEntrada.push(evento.processingStart - evento.startTime);
    }
  }).observe({ type: "event", durationThreshold: limiar, buffered: true } as PerformanceObserverInit);
  window.addEventListener(
    "keydown",
    (evento) => {
      teclas += 1;
      ultima = performance.now();
      const origem = evento.timeStamp;
      pedirQuadro(() => {
        ateQuadro.push(performance.now() - origem);
      });
    },
    { capture: true, passive: true },
  );
  const acompanhar = window.setInterval(() => {
    // A rajada acabou: 3 s sem tecla depois de pelo menos 10.
    if (teclas < 10 || performance.now() - ultima < 3000) return;
    window.clearInterval(acompanhar);
    const ordenadas = [...duracoes].sort((a, b) => a - b);
    const quadros = [...ateQuadro].sort((a, b) => a - b);
    const sobre = (lista: readonly number[], corte: number) => lista.filter((v) => v > corte).length;
    enviar("latencia", {
      teclas,
      // Só os eventos que passaram de 16 ms estão aqui; os outros ficaram abaixo do limiar.
      acimaDe16Ms: ordenadas.length,
      acimaDe50Ms: sobre(ordenadas, 50),
      maxEventTimingMs: arredondar(ordenadas.at(-1) ?? 0),
      p95EventTimingMs: arredondar(
        // Percentil sobre todas as teclas: as que não foram reportadas valem "< 16" (contadas como 0 aqui).
        percentil([...Array<number>(Math.max(0, teclas - ordenadas.length)).fill(0), ...ordenadas], 95),
      ),
      p95AteOQuadroMs: arredondar(percentil(quadros, 95)),
      p50AteOQuadroMs: arredondar(percentil(quadros, 50)),
      maxAteOQuadroMs: arredondar(quadros.at(-1) ?? 0),
      acimaDe50MsAteOQuadro: sobre(quadros, 50),
      p95AtrasoDeEntradaMs: arredondar(percentil([...atrasosDeEntrada].sort((a, b) => a - b), 95)),
      limiarEventTimingMs: limiar,
    });
  }, 500);
}

// --- Semear e retorno depois do jogo --------------------------------------------------------------------------

interface Visita {
  n: number;
}

async function lerJson<T>(caminho: string): Promise<T> {
  const resposta = await fetch(caminho);
  return (await resposta.json()) as T;
}

async function cenarioSemear(): Promise<void> {
  const caminhos = await lerJson<string[]>("/__medicao/jogos");
  let novos = 0;
  for (const path of caminhos) {
    const resposta = await call("library.add_game", { path, system: null });
    if (resposta.status === "added") novos += 1;
  }
  // Biblioteca de medição: o assistente da primeira vez concluído (a abertura medida é a de quem já usa o Tako) e a
  // busca de capas desligada (sem chave do IGDB ela falha em laço e enche o repouso de trabalho que não é da interface).
  await call("settings.set", finishChoice());
  await call("settings.set", coverChoice(false));
  const pagina = await call("library.list", { limit: 1 });
  enviar("semeado", { pedidos: caminhos.length, novos, total: pagina.total });
}

/**
 * Primeira visita: guarda o estado de tela (jogo aberto na página dele), manda jogar o jogo de teste e espera a
 * interface ser fechada pelo núcleo. Segunda visita (a interface reaberta depois do jogo): só observa e relata.
 */
async function cenarioRetorno(): Promise<void> {
  const visita = await lerJson<Visita>(`/__medicao/visita?c=retorno&r=${consulta.get("rodada") ?? ""}`);
  enviar("visita", { n: visita.n });
  const jogo = (await lerJson<string[]>("/__medicao/jogo-de-teste"))[0];
  if (jogo === undefined) {
    enviar("erro", { mensagem: "sem jogo de teste" });
    return;
  }
  if (visita.n === 1) {
    const resposta = await call("library.add_game", { path: jogo, system: null });
    if (resposta.gameId === null) {
      enviar("erro", { mensagem: `não adicionou o jogo de teste: ${resposta.status}` });
      return;
    }
    // O estado que a interface reaberta vai ler: a página do jogo (a mesma de quem apertou Jogar).
    await call("ui.state.set", { json: serializeScreenState({ ...defaultScreenState, activeGame: resposta.gameId }) });
    enviar("jogo-pedido", { gameId: resposta.gameId });
    await call("game.play", { gameId: resposta.gameId });
    return;
  }
  const fim = on("game.ended", (fim) => {
    enviar("game.ended-recebido", { fim });
  });
  const aberta = await call("game.running");
  enviar("reaberta", { jogoAindaAberto: aberta !== null });
  await esperarBiblioteca();
  await esperar(1500);
  fim();
}

// --- Entrada ------------------------------------------------------------------------------------------------

async function executar(): Promise<void> {
  enviar("carregou", { url: window.location.href, cenario, devicePixelRatio: window.devicePixelRatio });
  if (cenario === "semear") {
    await cenarioSemear();
    return;
  }
  if (cenario === "latencia") cenarioLatencia();
  if (cenario === "retorno") {
    // `cenarioRetorno` decide o que fazer em cada visita; a biblioteca interativa é marcada nas duas.
    await cenarioRetorno();
    return;
  }
  const lista = await esperarBiblioteca();
  if (cenario === "repouso") await cenarioRepouso(lista);
  if (cenario === "fps") await cenarioFps(lista);
  if (cenario !== "latencia") enviar("fim", {});
}

if (ATIVO && cenario) {
  window.requestAnimationFrame = (retorno) => {
    pedidosDeQuadro += 1;
    return pedirQuadro(retorno);
  };
  window.addEventListener("error", (evento) => {
    enviar("erro-na-pagina", { mensagem: evento.message });
  });
  executar().catch((erro: unknown) => {
    enviar("erro", { mensagem: erro instanceof Error ? erro.message : String(erro) });
  });
}
