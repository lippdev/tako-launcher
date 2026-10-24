// Textos do app desktop (WEB-10a). Ficam num arquivo só, em português, para o WEB-06 trocar pelos catálogos de
// `locales/` sem caçar texto nos componentes.

import type { ListaDeJogosLabels } from "../components/ListaDeJogos/ListaDeJogos";
import type { ViewId } from "./uiState";

export const strings = {
  /** Título de cada vista (a vista vazia mostra só ele, por enquanto). */
  views: {
    biblioteca: "Biblioteca",
    configuracoes: "Configurações",
    perfil: "Perfil",
    downloads: "Downloads",
  } satisfies Record<ViewId, string>,

  topbar: {
    /** Nome do botão da marca (abre o MenuDoTako). */
    menu: "Menu do TAKO",
    /** Nome do grupo de abas da barra. */
    nav: "Navegação principal",
    library: "Biblioteca",
    downloads: "Downloads",
    profile: "Perfil",
    bigPicture: "Tela cheia",
    /** Nome do ícone de "conectado" do perfil, para leitores de tela. */
    online: "Conectado",
  },

  window: {
    group: "Janela",
    minimize: "Minimizar",
    // A interface ainda não sabe se a janela está maximizada (não há evento para isso): um rótulo só.
    toggleMaximize: "Maximizar ou restaurar",
    close: "Fechar",
  },

  menu: {
    settings: "Configurações",
    checkUpdates: "Procurar atualizações",
    quit: "Sair",
  },

  list: {
    label: "Biblioteca",
    tabsLabel: "Filtro da biblioteca",
    tabs: { all: "Todos", favorites: "Favoritos", installed: "Instalados" },
    searchPlaceholder: "Buscar",
    searchLabel: "Buscar na biblioteca",
    collapse: "Recolher a lista",
    expand: "Abrir a lista",
    shortcut: "Ctrl B",
    overview: "Visão geral",
  },

  search: {
    title: "Busca geral",
    escape: "Esc",
    placeholder: "Buscar jogos, configurações e ações",
    label: "Busca geral",
    // TODO: a busca geral é de outra tarefa; por enquanto só o marcador.
    soon: "A busca geral chega em breve.",
  },
} as const;

/** Textos da ListaDeJogos (mesmos da demo do componente). */
export const listLabels: ListaDeJogosLabels = {
  running: "Jogando",
  updateAvailable: "Atualização disponível",
  notInstalled: "Não instalado",
  favorite: "Favorito",
  downloading: (percent) => `Baixando · ${String(percent)}%`,
  status: { playing: "Jogando", completed: "Zerado", queued: "Na fila", paused: "Pausado", dropped: "Largado" },
  loading: "Lendo a biblioteca…",
  empty: "Nem com oito braços deu para achar esse.",
};
