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
  status: {
    playing: "Jogando",
    completed: "Zerado",
    queued: "Na fila",
    paused: "Pausado",
    dropped: "Largado",
  },
  loading: "Lendo a biblioteca…",
  empty: "Nem com oito braços deu para achar esse.",
};

export const libraryStrings = {
  selectionActions: "Ações da seleção",
  filters: "Ordenar e filtrar",
  close: "Fechar",
  clear: "Limpar",
  store: "Loja",
  status: "Situação",
  tags: "Etiquetas",
  installed: "Instalação",
  sort: "Ordenação",
  collection: "Coleção",
  allCollections: "Todas as coleções",
  any: "Todos",
  installedOnly: "Instalados",
  notInstalled: "Não instalados",
  reversed: "Ordem inversa",
  noStatus: "Sem situação",
  favorite: "Favoritar",
  unfavorite: "Tirar dos favoritos",
  addCollection: "Adicionar à coleção",
  removeCollection: "Remover da coleção",
  apply: "Aplicar",
  addTags: "Adicionar etiquetas",
  removeTags: "Remover etiquetas",
  tagPlaceholder: "Etiqueta",
  actionSuccess: "Biblioteca atualizada.",
  actionError: "Não foi possível atualizar a biblioteca.",
  loadError: "Não foi possível ler a biblioteca.",
  retry: "Tentar novamente",
  busy: "Atualizando…",
  emptyInstalled: "Nenhum jogo instalado.",
  emptyLibrary: "Sua biblioteca ainda está vazia.",
  emptyCollection: "Esta coleção ainda não tem jogos.",
  emptyFavorites: "Nenhum favorito ainda — use ☆ na página do jogo.",
  count: (n: number) => `${String(n)} ${n === 1 ? "jogo" : "jogos"}`,
  selected: (n: number, total: number) => `${String(n)} selecionados de ${String(total)}`,
  filterCount: (n: number) => `Ordenar e filtrar · ${String(n)} filtros`,
  collectionLabel: (name: string, count: number | null) => (count === null ? name : `${name} · ${String(count)}`),
  stores: {
    steam: "Steam",
    epic: "Epic",
    gog: "GOG",
    xbox: "Xbox",
    ubisoft: "Ubisoft",
    ea: "EA",
    battle_net: "Battle.net",
    amazon: "Amazon",
    heroic: "Heroic",
    lutris: "Lutris",
    bottles: "Bottles",
    itch: "itch.io",
    emulators: "Emuladores",
    manual: "Manual",
  },
  statuses: {
    playing: "Jogando",
    beaten: "Zerado",
    wishlist: "Na fila",
    paused: "Pausado",
    abandoned: "Largado",
  },
  sorts: {
    recently_played: "Jogado por último",
    name: "A–Z",
    play_time: "Tempo de jogo",
    recently_added: "Adicionado recentemente",
    rating: "Nota",
    size: "Tamanho",
  },
} as const;
