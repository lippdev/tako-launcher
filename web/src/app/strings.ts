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

/** Textos da tela de Configurações (WEB-13a). */
export const settingsStrings = {
  /** Nome da navegação e do conteúdo, para leitores de tela. */
  nav: "Categorias das configurações",
  count: (n: number) => `Configurações · ${String(n)} categorias`,
  loadError: "Não foi possível ler as configurações.",
  saveError: "Não foi possível salvar a configuração.",
  soon: "Esta categoria chega em breve.",
  /** Título e frase de cada categoria (a frase é a do cabeçalho). */
  categories: {
    general: { title: "Geral", description: "Início, idioma e comportamento durante os jogos." },
    theme: { title: "Tema", description: "Cartões de tema e cor de destaque." },
    appearance: { title: "Aparência", description: "Tema, tamanho da interface e efeitos." },
    sound: { title: "Som", description: "Volume e áudio do tema." },
    display: { title: "Tela", description: "Monitor, resolução e HDR dos jogos." },
    controllers: { title: "Controles", description: "Controles, botões e atalhos." },
    library: { title: "Biblioteca e lojas", description: "Lojas ligadas e a leitura da biblioteca." },
    covers: { title: "Capas e dados", description: "Capas, fontes de dados e cache." },
    downloads: { title: "Downloads", description: "Pastas e limites de download." },
    integrations: { title: "Integrações", description: "Contas e serviços ligados ao TAKO." },
    system: { title: "Sistema", description: "Desempenho, manutenção e avançado." },
    about: { title: "Sobre", description: "Versão, licenças e créditos." },
  },
  general: {
    mode: "Modo",
    bigPicture: { title: "Ir para a tela cheia", description: "Interface para controle e TV. Atalho no desktop: F11", action: "Trocar" },
    ask: {
      title: "Perguntar pela tela cheia ao achar um controle",
      description: "Quando o TAKO abre no desktop com um controle ligado",
    },
    openIn: { title: "Abrir em", window: "Janela", bigPicture: "Tela cheia" },
    language: { section: "Idioma", title: "Idioma", description: "Vale para todos os textos do TAKO", pt: "Português (Brasil)", en: "English (US)" },
    games: "Jogos",
    restore: {
      title: "Restaurar tela e áudio ao fim do jogo",
      description: "Volta ao monitor, à resolução e à saída de som de antes de jogar",
    },
  },
  appearance: {
    theme: "Tema",
    themeRow: { title: "Tema", description: "O visual de todo o TAKO" },
    themes: { tako: "TAKO", escuro: "Escuro", "meia-noite": "Meia-noite", oled: "Preto OLED", vidro: "Liquid Glass" },
    artColors: {
      title: "Cores da arte do jogo",
      description: "Brilhos e destaque pegam a cor do jogo na tela. Desligado, fica a cor pedra do logo",
      onlyTako: "Só no tema TAKO.",
    },
    layout: "Layout",
    scale: { title: "Tamanho da interface", description: "Deixa tudo maior ou menor" },
    effects: "Efeitos",
    motion: {
      title: "Reduzir animações",
      description: "Desliga transições de página, esmaecimentos e animações de passar o mouse",
    },
    glass: { title: "Efeitos de vidro", description: "Desfoque atrás de painéis, botões e cabeçalhos" },
  },
  library: {
    stores: "Lojas",
    reading: "Leitura",
    scan: {
      title: "Ler as lojas de novo",
      description: "Procura jogos novos nas lojas ligadas. Ligar ou desligar uma loja vale na próxima leitura.",
      action: "Ler as lojas de novo",
    },
    scanError: "Não foi possível ler as lojas.",
    sourcesError: "Não foi possível consultar as lojas.",
    empty: "As lojas ainda não foram lidas.",
    off: "Desligada",
    notRead: "Ainda não lida",
    noClient: "Cliente não encontrado",
    found: (n: number) => `${String(n)} ${n === 1 ? "jogo" : "jogos"}`,
    readAt: (when: string) => `Lida em ${when}`,
    progress: {
      scanning: (done: number, total: number) => `Lendo as lojas… ${String(done)} de ${String(total)}`,
      measuring: "Medindo o tamanho das pastas dos jogos…",
      done: "Leitura concluída.",
    },
  },
} as const;

/** Textos da página do jogo (WEB-12a). */
export const gameStrings = {
  label: (title: string) => `Página de ${title}`,
  loading: "Abrindo a página do jogo…",
  loadError: "Não foi possível abrir a página deste jogo.",
  retry: "Tentar novamente",
  about: "Sobre o jogo",
  details: "Detalhes do jogo",
  dataBar: "Dados do jogo",
  developer: "Desenvolvedora",
  publisher: "Publicadora",
  release: "Lançamento",
  genres: "Gêneros",
  size: "Tamanho no disco",
  installation: "Situação",
  tags: "Etiquetas",
  platform: "Plataforma",
  lastSession: "Última sessão",
  playTime: "Tempo de jogo",
  metacritic: "Metacritic",
  recentSessions: "Sessões recentes",
  verdicts: {
    universal: "Aclamação universal",
    favorable: "Críticas favoráveis",
    mixed: "Críticas mistas",
    unfavorable: "Críticas desfavoráveis",
  },
  installed: "Instalado",
  notInstalled: "Não instalado",
  upcoming: "Em breve",
  running: "Jogando",
  actions: {
    play: "Jogar",
    opening: "Abrindo…",
    resume: "Voltar ao jogo",
    stop: "Encerrar jogo",
    install: "Instalar",
    upcoming: "Em breve",
    favorite: "Favoritar",
    unfavorite: "Tirar dos favoritos",
    manage: "Gerenciar",
    settings: "Configurações do jogo",
  },
  status: {
    label: "Situação do jogo",
    set: "Definir situação",
    none: "Sem situação",
    playing: "Jogando",
    completed: "Zerado",
    queued: "Na fila",
    paused: "Pausado",
    dropped: "Largado",
  },
  notices: {
    opening: (title: string) => `Tentáculos no controle · abrindo ${title}`,
    cannotPlay: "Este jogo não pode ser aberto agora.",
    playError: "Não foi possível abrir o jogo.",
    updateError: "Não foi possível atualizar o jogo.",
  },
} as const;

/** Textos da Visão geral da biblioteca (WEB-10b). */
export const overviewStrings = {
  label: "Visão geral da biblioteca",
  // TODO(WEB-14): "Bem-vindo de volta, <nome>" quando o perfil chegar da ponte.
  welcome: "Bem-vindo de volta",
  loading: "Lendo a biblioteca…",
  loadError: "Não foi possível ler a biblioteca.",
  retry: "Tentar novamente",
  empty: "Sua biblioteca ainda está vazia.",
  heroLabel: (title: string) => `Em destaque: ${title}`,
  continuePlaying: "Continuar jogando",
  recent: "Recentes",
  previous: "Anterior",
  next: "Próximo",
  playedFor: (time: string) => `${time} jogadas`,
} as const;
