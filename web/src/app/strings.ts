// Textos do app desktop (WEB-10a). Ficam num arquivo só, em português, para o WEB-06 trocar pelos catálogos de
// `locales/` sem caçar texto nos componentes.

import type { ListaDeJogosLabels } from "../components/ListaDeJogos/ListaDeJogos";
import type { ViewId } from "./uiState";

export const centralStrings = {
  title: "Central rápida",
  actionError: "Não foi possível aplicar a alteração. Tente novamente.",
  cpuTime: "Tempo de CPU do núcleo",
  memory: (mib: number) => `${mib.toFixed(1)} MiB`,
  seconds: (seconds: number) => `${seconds.toFixed(1)} s`,
} as const;

/** Complementos do onboarding web; os textos dos passos vêm do catálogo `first_run`. */
export const onboardingStrings = {
  loading: "Lendo as lojas e os emuladores…",
  loadError: "Não foi possível ler a configuração inicial.",
  saveError: "Não foi possível salvar. Tente novamente.",
  retry: "Tentar novamente",
  enable: "Ligar",
  disable: "Desligar",
  toggleStore: (name: string) => `Ler jogos de ${name}`,
  found: "Encontrado",
  gameCount: (n: number) => `${String(n)} ${n === 1 ? "jogo" : "jogos"}`,
  folderHelp: "Digite o caminho completo de uma pasta de ROMs que já existe neste PC.",
  addFolder: "Adicionar pasta",
  cancel: "Cancelar",
  noGame: "Sem jogo para comparar",
  igdbKeyNote: "Usa a chave da IGDB (Twitch) já salva no Tako. Se ainda não houver chave, a busca aguarda a configuração do provedor.",
  fullscreenNote: "A escolha fica salva para abrir o Tako. O modo de tela cheia chegará à versão web em uma próxima atualização.",
} as const;

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
    // Atalho do campo (aria-keyshortcuts) e o nome da lista de resultados, para leitores de tela.
    keyShortcuts: "Control+K",
    results: "Resultados da busca",
    placeholder: "Buscar jogos, telas e configurações",
    label: "Busca geral",
    // desktop.global_search_empty
    empty: "Nenhum resultado encontrado",
    error: "Não foi possível buscar agora.",
    loading: "Buscando…",
    // desktop.global_search_library: a etiqueta de um jogo; a de um atalho de tela é nova.
    tagGame: "Biblioteca",
    tagShortcut: "Atalho",
    viewsGroup: "Ir para",
    settingsGroup: "Configurações",
    installed: "Instalado",
    notInstalled: "Não instalado",
    count: (n: number) => `${String(n)} ${n === 1 ? "resultado" : "resultados"}`,
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

/**
 * Textos dos modos da lista (WEB-11c) que não existem nos catálogos de `locales/`. Os nomes dos modos e dos
 * agrupamentos (`desktop.view.*`, `desktop.group.*`) vêm de `t()`.
 */
export const listViewStrings = {
  categoriesRegion: "Biblioteca por categorias",
  loadingGroups: "Lendo as categorias…",
  /** Nome do ▶ do cartão da grade. */
  play: "Jogar",
  playError: "Não foi possível abrir o jogo.",
  cannotPlay: "Este jogo não pode ser aberto agora.",
  previous: "Anterior",
  next: "Próximo",
} as const;

/**
 * Textos das coleções na lista (WEB-11b) que não existem nos catálogos de `locales/`. Os que existem
 * (`smart.*`, `library.collection.*`, `collections.*`) vêm de `t()`.
 */
export const collectionStrings = {
  nameLabel: "Nome da coleção",
  useCurrent: "Usar os filtros atuais",
  useCurrentFor: (name: string) => `Usar os filtros atuais em ${name}`,
  renameFor: (name: string) => `Renomear ${name}`,
  deleteFor: (name: string) => `Excluir ${name}`,
  savedRule: "Filtros da coleção atualizados.",
  created: "Coleção criada.",
  renamed: "Coleção renomeada.",
  deleted: "Coleção excluída.",
  saveError: "Não foi possível salvar a coleção.",
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
    theme: { title: "Tema", description: "Escolha o visual do TAKO." },
    appearance: { title: "Aparência", description: "Tema, tamanho da interface e efeitos." },
    sound: { title: "Som", description: "Volume e áudio do tema." },
    display: { title: "Tela", description: "Onde o jogo aparece e toca, e o que muda durante a partida." },
    controllers: { title: "Controles", description: "Controles, botões e atalhos." },
    library: { title: "Biblioteca e lojas", description: "Lojas ligadas, leitura da biblioteca, emuladores e coleções automáticas." },
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
    // WEB-13c1: smart.title, smart.text, smart.empty, smart.delete_*, settings.library.rename/delete
    smart: {
      section: "Coleções automáticas",
      text: "Salve uma combinação de filtros na biblioteca. Os jogos de cada coleção se atualizam sozinhos.",
      empty: "Nenhuma coleção automática",
      rename: (name: string) => `Renomear ${name}`,
      remove: (name: string) => `Excluir ${name}`,
      renameTitle: "Renomear a coleção automática",
      nameLabel: "Nome da coleção",
      renamed: "Coleção renomeada.",
      deleted: "Coleção excluída.",
      loadError: "Não foi possível ler as coleções automáticas.",
      saveError: "Não foi possível salvar a coleção.",
      count: (n: number) => `${String(n)} ${n === 1 ? "jogo" : "jogos"}`,
    },
  },
  // Emuladores e ROMs (WEB-13c1): os textos de settings.emulators.* do catálogo do app atual.
  emulators: {
    section: "Emuladores e ROMs",
    text: "O Tako junta os jogos que o RetroArch, o ES-DE e os emuladores avulsos já conhecem com as suas pastas de ROMs, e abre cada um no emulador certo.",
    folders: "Pastas de ROMs",
    noFolders: "Nenhuma pasta de ROMs",
    folderAuto: "Uma subpasta por sistema",
    folderSystem: (path: string) => `Sistema da pasta ${path}`,
    add: {
      title: "Adicionar pasta…",
      description: "A pasta de um sistema (ex.: snes) ou uma com uma subpasta por sistema, como a do ES-DE",
      label: "Caminho da pasta",
      placeholder: "C:\\Jogos\\ROMs",
      action: "Adicionar",
      choose: "Escolher…",
      // O seletor nativo de pasta do sistema preenche o campo.
      help: "Digite o caminho completo da pasta.",
    },
    added: (path: string) => `Pasta adicionada: ${path}`,
    removed: "Pasta removida",
    remove: "Remover pasta",
    removeFor: (path: string) => `Remover a pasta ${path}`,
    emulators: "Emuladores",
    found: "Encontrado",
    missing: "Não encontrado",
    missingDescription: "Escolha o executável se ele estiver em outra pasta",
    flatpak: (id: string) => `Flatpak · ${id}`,
    chosen: (path: string) => `Escolhido por você: ${path}`,
    rescan: { title: "Ler as pastas de ROMs de novo", description: "Procura ROMs novas nas pastas e nos emuladores", action: "Ler de novo" },
    errors: {
      load: "Não foi possível consultar os emuladores.",
      add: "Não foi possível adicionar a pasta. Confira se o caminho completo existe.",
      remove: "Não foi possível remover a pasta.",
      system: "Não foi possível mudar o sistema da pasta.",
    },
  },
  // Os textos de Tema a Sobre (WEB-13b) são os dos catálogos do app atual (`crates/tako-core/locales/pt-BR`); a
  // chave de cada bloco vem no comentário, para o WEB-06 achar. Texto sem chave lá é novo desta tela.
  theme: {
    // settings.appearance.themes_header, desc_tako … desc_glass
    section: "Tema",
    cards: "Escolha do tema",
    descriptions: {
      tako: "Preto tinta e o polvo",
      escuro: "Monocromático, como desenhado",
      "meia-noite": "Preto azulado",
      oled: "Preto puro para OLED",
      vidro: "Translúcido, iluminado pelos seus jogos",
    },
    // O nome da seção das opções do tema escolhido (README do Temas).
    takoOptions: "TAKO",
    // WEB-13c1: settings.theme.accent_header, desk_accent_*, custom, desk_accent_custom_hint, desk_accent_invalid
    accent: {
      section: "Cor de destaque",
      cards: "Escolha da cor de destaque",
      names: {
        stone: "Pedra TAKO",
        orange: "Laranja",
        gold: "Ouro",
        green: "Verde",
        cyan: "Ciano",
        blue: "Azul",
        violet: "Violeta",
        pink: "Rosa",
        white: "Branco",
      },
      custom: { title: "Cor própria", label: "Código da cor", hint: "Código da cor, como #D8C8BC", invalid: "Use o formato #RRGGBB" },
    },
    // settings.appearance.desk_theme_glass, glass_ambient, glass_opacity, glass_pill (e as _desc)
    glass: {
      section: "Liquid Glass",
      ambient: {
        title: "Arte do jogo como luz ambiente",
        description: "Tinge a janela com a arte do jogo na tela. Desligado, usa uma aurora na cor de destaque",
      },
      opacity: {
        title: "Opacidade das superfícies",
        description: "Valores maiores deixam os painéis mais sólidos e fáceis de ler, de 2 a 20%",
      },
      pill: { title: "Controles em pílula", description: "Botões, seletores, abas e chips viram pílulas" },
    },
  },
  sound: {
    // settings.sound.*
    music: "Música",
    musicEnabled: { title: "Música de fundo do tema", description: "Toca a música definida pelo tema na tela cheia." },
    musicVolume: {
      title: "Volume da música do tema",
      description: "0% silencia a música do tema. A música nunca toca durante os jogos.",
    },
    navigation: "Navegação",
    navigationVolume: {
      title: "Volume dos sons de navegação",
      description: "0% desliga. Tons locais curtos para navegar; nunca tocam durante um jogo.",
    },
    // accessibility.sound_muted
    muted: "Mudo",
  },
  screen: {
    // settings.screen.*
    screen: "Tela",
    managed: {
      title: "Tela da sessão",
      value: "Gamescope",
      description: "Esta sessão (gamescope) cuida das telas; o Tako não muda tela, resolução nem HDR",
    },
    enabled: {
      title: "Usar a tela de jogo",
      description: "Ao abrir um jogo, usa só a tela escolhida (ex.: a TV); tudo volta ao sair do jogo",
    },
    display: {
      title: "Tela onde você joga",
      description: "Na primeira vez com uma tela nova, o Tako pergunta se você está vendo a imagem",
      choose: "Escolher",
      none: "Nenhuma tela encontrada",
      // Sem chave no catálogo.
      error: "Não foi possível ler as telas.",
    },
    hide: { title: "Desligar as outras telas durante o jogo", description: "Saem da área de trabalho e voltam ao fim do jogo" },
    mode: { title: "Resolução e taxa", description: "Valem na tela de jogo só durante o jogo", keep: "Manter a atual" },
    hdr: { title: "Ligar o HDR", description: "Liga o HDR na tela de jogo e desliga ao sair" },
    // vrr.toggle, vrr.toggle_desc, vrr.unsupported
    vrr: {
      title: "Ligar o VRR",
      description: "Liga a taxa variável (FreeSync, G-Sync) na tela de jogo e desliga ao sair",
      unsupported: "Esta tela não informa VRR",
    },
    audio: "Áudio",
    audioOutput: {
      title: "Saída de áudio do jogo",
      description: "Ao abrir um jogo, o som passa a sair por aqui; ao fechar, volta para a saída de antes",
      keep: "Não alterar",
      locked: "Este sistema não deixa o Tako trocar a saída de áudio",
      // Sem chave no catálogo.
      error: "Não foi possível ler as saídas de áudio.",
    },
    during: "Durante o jogo",
    restore: {
      title: "Restaurar tela e som ao sair do jogo",
      description:
        "Volta resolução, taxa, tela principal, VRR, saída de áudio e volume como estavam antes do jogo. Desligada, o VRR ligado para o jogo continua ligado",
    },
    cursor: { title: "Esconder o ponteiro", description: "O ponteiro do mouse some enquanto o jogo está aberto" },
    mouse: {
      title: "Mover o ponteiro com o analógico",
      description: "Durante o jogo, o analógico move o ponteiro e os gatilhos clicam (launchers e menus sem controle)",
    },
  },
  integrations: {
    // settings.system.sunshine*, discord*
    sunshine: "Sunshine / Apollo",
    sunshineSync: {
      title: "Manter sincronizado ao reescanear",
      description: "Atualiza os apps do Sunshine quando a biblioteca for reexaminada. O Tako não envia dados pela rede.",
    },
    discord: "Discord",
    discordPresence: {
      title: "Mostrar o jogo no Discord",
      description: "Seus amigos veem no Discord o que você está jogando pelo Tako",
    },
  },
  system: {
    // settings.system.advanced_header, developer, developer_desc
    advanced: "Avançado",
    developer: {
      title: "Opções de desenvolvedor",
      description: "Ferramentas para quem cria temas ou testa o Tako; deixe desligado no dia a dia",
    },
  },
  about: {
    // settings.about.*
    app: "TAKO",
    version: "Versão",
    license: {
      title: "Licença",
      description: "Software livre: você pode usar, estudar, mudar e compartilhar o Tako",
      value: "AGPL-3.0",
      action: "Ver o texto",
      // O texto completo da AGPL-3.0; abre no navegador do sistema.
      url: "https://www.gnu.org/licenses/agpl-3.0.html",
    },
    // Sem chave no catálogo.
    source: {
      title: "Código-fonte",
      description: "github.com/lippdev/tako-launcher",
      action: "Abrir no navegador",
      url: "https://github.com/lippdev/tako-launcher",
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
    endError: "Não foi possível encerrar o jogo.",
  },
  /** O que o design do ErroDoJogo tem e os catálogos não (o Slint não mostra os detalhes técnicos). */
  errorDialog: {
    technicalDetails: "Detalhes técnicos",
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

/**
 * Textos do painel "Configurações deste jogo" (WEB-12b). São os do catálogo do app atual
 * (`crates/tako-core/locales/pt-BR/game.toml`, tabela `game_settings`; `vrr.toml`); a chave de cada um vem no comentário.
 */
export const gameSettingsStrings = {
  // ui.toml: details.game_settings
  title: "Configurações deste jogo",
  close: "Fechar (Esc)",
  groups: {
    // game_settings.screen, sound, launch
    screen: "Tela",
    sound: "Som",
    launch: "Abrir o jogo",
  },
  rows: {
    useScreen: "Usar a tela de jogo", // game_settings.use_screen
    screen: "Tela deste jogo", // game_settings.which_screen
    mode: "Resolução e taxa", // game_settings.mode
    hdr: "HDR", // game_settings.hdr
    vrr: "VRR", // vrr.game
    audio: "Saída de áudio", // game_settings.audio
    arguments: "Argumentos extras", // game_settings.arguments
    folder: "Pasta de trabalho", // game_settings.folder
    environment: "Variáveis de ambiente", // game_settings.environment
    commands: "Comandos antes e depois", // game_settings.before_command / after_command
  },
  notes: {
    useScreen: "A tela é a escolhida em Configurações › Tela de jogo, ou a deste jogo logo abaixo.", // screen_note
    screen:
      "Outra tela só para este jogo (por exemplo, o monitor da mesa em vez da TV). A resolução e o HDR abaixo valem para ela.", // which_screen_desc
    launch: "Valem quando o Tako abre o executável. Jogos da Steam usam as opções de inicialização da Steam.", // launch_note
    environment: "NOME=valor, separadas por ponto e vírgula. Valem quando o Tako abre o executável.", // environment_hint
    // Sem chave no catálogo: a gravação de comandos espera a permissão ao abrir o jogo.
    commands: "Ainda não dá para editar os comandos por aqui.",
    vrrUnsupported: "Esta tela não informa VRR", // vrr.unsupported
    audioLocked: "Este sistema não deixa o Tako trocar a saída de áudio", // settings.screen
  },
  commandsBefore: "Antes",
  commandsAfter: "Depois",
  values: {
    default: "Padrão: ", // game_settings.default
    on: "Ligado", // game_settings.on
    off: "Desligado", // game_settings.off
    none: "Nenhum", // game_settings.none
    noneFeminine: "Nenhuma", // game_settings.environment
    noScreen: "Nenhuma", // game_settings.no_screen
    keepCurrent: "Manter a atual", // game_settings.keep_current
    keepAudio: "Não alterar", // game_settings.keep_audio
    executableFolder: "Pasta do executável", // game_settings.exe_folder
    disconnected: (name: string) => `${name} (desconectada)`, // game_settings.disconnected
  },
  changes: {
    one: "mudança", // game_settings.changes
    other: "mudanças",
    none: "nenhuma mudança",
  },
  reset: {
    action: "Usar tudo do Tako", // game_settings.reset
    description: (changes: number) =>
      changes === 0 ? "Nenhuma mudança neste jogo" : `Volta as ${String(changes)} ${changes === 1 ? "mudança" : "mudanças"} ao padrão`,
    title: "Usar tudo do Tako neste jogo?", // game_settings.reset_title
    text: "As mudanças deste jogo (tela, som e como ele abre) somem e ele volta a seguir as Configurações do Tako.", // reset_text
    cancel: "Cancelar", // common.cancel
  },
  loadError: "Não foi possível ler as configurações deste jogo.",
  saveError: "Não foi possível salvar agora; veja os registros", // game_settings.save_failed
  folderMissing: (folder: string) => `Pasta de trabalho não encontrada: ${folder}`, // game_settings.folder_missing
  invalid: "Valor inválido: confira o tamanho e o formato.",
  invalidEnvironment: "Variáveis de ambiente inválidas: use NOME=valor, até 32.",
} as const;

/** Textos do editor de etiquetas da página do jogo (WEB-12b); do catálogo `tags` e `desktop.info.tags`. */
export const tagEditorStrings = {
  list: "Etiquetas do jogo",
  add: "Etiqueta", // tags.placeholder ("+ Etiqueta")
  field: "Nova etiqueta",
  menuTitle: "Etiquetas", // desktop.info.tags
  remove: (name: string) => `Tirar ${name}`, // tags.remove
  create: (name: string) => `Criar "${name}"`, // tags.create
  games: (n: number) => `${String(n)} ${n === 1 ? "jogo" : "jogos"}`, // common.games
  rule: "Até 40 caracteres. Uma tag que já existe é só aplicada.", // tags.name_rule
  error: "Não foi possível atualizar as etiquetas.",
} as const;

/**
 * Textos da tela Perfil e da gestão de perfis (WEB-14c) que não existem nos catálogos de `locales/`. Os que existem
 * (`profile.*`, `profiles.*`, `collections.*`) vêm de `t(...)`.
 */
export const profileStrings = {
  loading: "Lendo o perfil…",
  loadError: "Não foi possível ler o perfil.",
  retry: "Tentar novamente",
  /** Nome do grupo de abas. */
  tabs: "Seções do perfil",
  /** Nome do seletor de período da guia Sessões. */
  period: "Período",
  /** Nomes das barras e grupos das guias, para leitores de tela. */
  sessionsToolbar: "Filtros de sessões",
  collectionsToolbar: "Opções de coleções",
  collectionsGrid: "Coleções de jogos",
  collectionActions: "Ações de coleção",
  /** Nome da lista das últimas sessões na Visão geral. */
  recentList: "Sessões recentes",
  previous: "Anterior",
  next: "Próximo",
  /** O perfil gravado para a próxima abertura difere do que está em uso. */
  nextOpen: "Na próxima abertura",
  restartNotice: (name: string) => `O perfil ${name} será usado quando você reabrir o Tako.`,
  saveError: "Não foi possível salvar. Tente de novo.",
} as const;

/**
 * Textos de "Editar jogo", "Adicionar jogo", "Remover" e "Fontes e IDs" (WEB-12c). São os do catálogo do app atual
 * (`crates/tako-core/locales/pt-BR/game.toml`, tabelas `edit_game`, `add_game`, `source_ids`; `ui.toml`, `options`);
 * a chave de cada um vem no comentário.
 */
export const editGameStrings = {
  menu: {
    sourceIds: "Fontes e IDs", // options.source_ids
    edit: "Editar", // options.edit
    remove: "Remover da biblioteca", // options.remove
  },
  close: "Fechar (Esc)",
  cancel: "Cancelar", // common.cancel
  save: "Salvar", // common.save
  title: "Editar jogo", // edit_game.title
  name: "Nome", // edit_game.name
  arguments: "Argumentos", // edit_game.arguments
  argumentsHint: "Nenhum", // game_settings.none
  folder: "Pasta de trabalho", // game_settings.folder
  folderHint: "Vazio = a pasta do executável.", // game_settings.folder_hint
  choose: "Escolher…",
  tracking: "Fim do jogo", // edit_game.tracking
  target: "Arquivo", // edit_game.target
  trackingOptions: {
    auto: "Automático", // edit_game.track.auto
    process: "Processo iniciado", // edit_game.track.process
    install_folder: "Pasta do jogo", // edit_game.track.folder
    none: "Não acompanhar", // edit_game.track.none
  },
  nameRequired: "Dê um nome ao jogo.", // edit_game.name_required
  folderMissing: (folder: string) => `Pasta de trabalho não encontrada: ${folder}`, // game_settings.folder_missing
  // Sem chave no catálogo: a tela de Slint não tem estes casos (ela só aceitava o que o teclado virtual deixava).
  invalid: "Valor inválido: confira o tamanho e o formato.",
  notManual: "Só jogos adicionados à mão podem ser editados.",
  loadError: "Não foi possível ler os dados deste jogo.",
  saveError: "Não foi possível salvar agora; veja os registros", // source_ids.save_failed
  saved: (game: string) => `${game} atualizado`, // edit_game.saved
  removeTitle: (game: string) => `Remover ${game}?`, // options.remove_title
  removeText: "O jogo sai do Tako; os arquivos no disco ficam onde estão.", // options.remove_text
  removeAction: "Remover da biblioteca", // options.remove
  removed: "Jogo removido", // options.removed
  removeError: "Não foi possível remover o jogo.",
} as const;

export const addGameStrings = {
  title: "Adicionar jogo", // add_game.title
  menu: "Adicionar jogo…", // desktop menu: add_game
  file: "Arquivo do jogo",
  fileHint: "Caminho do executável, do atalho ou da ROM.",
  filePlaceholder: "C:\\Jogos\\MeuJogo\\jogo.exe",
  choose: "Escolher…",
  close: "Fechar (Esc)",
  cancel: "Cancelar", // common.cancel
  add: "Adicionar",
  adding: "Adicionando…",
  whichSystem: "De qual sistema é este jogo?", // add_game.which_system
  systemText: "O Tako usa o sistema para achar os dados e as artes. Abrir pelo emulador chega numa próxima versão.", // add_game.system_text
  otherSystem: "Outro sistema", // add_game.other_system
  added: (game: string) => `${game} adicionado`, // add_game.added
  addedWithSystem: (game: string, system: string) => `${game} (${system}) adicionado`,
  already: (game: string) => `${game} já está na biblioteca`, // add_game.already
  unreadable: "Não consegui ler esse atalho", // add_game.unreadable
  // Sem chave no catálogo: no Slint o seletor do sistema operacional só deixava escolher o que existe.
  empty: "Digite o caminho do arquivo.",
  notFound: "Não encontrei esse arquivo.",
  invalid: "Esse caminho não é válido.",
  error: "Não foi possível adicionar o jogo.",
} as const;

export const sourceIdsStrings = {
  title: (game: string) => `Fontes e IDs de ${game}`, // source_ids.title
  close: "Fechar (Esc)",
  term: "Nome buscado", // source_ids.term
  search: "Buscar",
  noSources: "Nenhuma fonte de metadados ligada", // source_ids.no_sources
  typeId: "Digitar ou colar o ID", // source_ids.type_id
  idIn: (source: string) => `ID no ${source}`, // source_ids.id_in
  check: "Conferir",
  none: "Nenhum", // source_ids.none
  noneText: "Não usar esta fonte para este jogo", // source_ids.none_text
  automatic: "Automático", // source_ids.automatic
  automaticText: "Soltar a escolha; o Tako casa sozinho de novo", // source_ids.automatic_text
  noneChosen: "Nenhum · escolhido por você", // source_ids.none_chosen
  noMatch: "Sem correspondência", // source_ids.no_match
  chosen: "escolhido por você", // source_ids.chosen
  automaticLower: "automático", // source_ids.automatic_lower
  searching: (term: string) => `Buscando "${term}"…`, // source_ids.searching
  nothing: (term: string) => `Nada encontrado para "${term}".`, // source_ids.nothing
  checking: (source: string) => `Conferindo no ${source}…`, // source_ids.checking
  unknownId: (source: string) => `O ${source} não conhece esse ID. Confira o número ou use a busca.`, // source_ids.unknown_id
  confirmTitle: (name: string) => `Usar ${name}?`, // source_ids.confirm_title
  confirmText: (source: string, id: string) =>
    `${source} · ID ${id}. Os dados e as imagens desta fonte vêm deste jogo a partir de agora.`, // source_ids.confirm_text
  use: "Usar", // source_ids.use
  cancel: "Cancelar", // common.cancel
  pinnedDone: (game: string, name: string, source: string) => `${game} usa ${name} no ${source}`, // source_ids.pinned_done
  noneDone: (game: string, source: string) => `${source} não será usado para ${game}`, // source_ids.none_done
  autoDone: (source: string) => `${source} volta a casar sozinho`, // source_ids.auto_done
  saveError: "Não foi possível salvar agora; veja os registros", // source_ids.save_failed
  loadError: "Não foi possível ler as fontes deste jogo.",
  searchError: "Não foi possível buscar agora.",
  current: "Escolhido agora",
  best: "Mais provável",
} as const;

export const artReviewStrings = {
  title: "Revisar capas", // art_review.title
  desc: "As fontes casam a biblioteca sozinhas; aqui ficam só os jogos que valem conferir", // art_review.desc
  open: "Abrir", // art_review.open
  text: "Capa, fundo, logo e ícone vêm sozinhos das fontes ligadas. Confira os jogos achados só pelo nome e os que ficaram sem alguma imagem.", // art_review.text
  computing: "Conferindo a biblioteca…", // art_review.computing
  summary: (count: number, total: number) => `${count} de ${total} jogos para conferir`, // art_review.summary
  allGood: (total: number) => `Os ${total} jogos estão com as imagens e as correspondências em ordem`, // art_review.all_good
  refresh: "Buscar de novo para a biblioteca inteira", // art_review.refresh
  refreshDesc: "Consulta as fontes de novo para todos os jogos, em segundo plano", // art_review.refresh_desc
  doubt: (source: string, found: string) => `Achado pelo nome no ${source} como "${found}"`, // art_review.doubt
  missing: (kinds: string) => `Sem ${kinds}`, // art_review.missing
  pickArt: "Escolher imagens", // art_review.pick_art
  fixMatch: "Corrigir correspondência", // art_review.fix_match
  correct: "Está certo", // art_review.correct
  highTitle: (n: number) => `${n} com confiança alta`, // art_review.high_title
  highDesc: "O nome achado é praticamente o mesmo do jogo", // art_review.high_desc
  acceptAll: (n: number) => `Aceitar as ${n}`, // art_review.accept_all
  toReview: (n: number) => `Para revisar · ${n}`, // art_review.to_review
  noCover: "Sem capa", // art_review.no_cover
  confHigh: (percent: number) => `Confiança alta · ${percent}%`, // art_review.conf_high
  confMedium: (percent: number) => `Confiança média · ${percent}%`, // art_review.conf_medium
  confLow: (percent: number) => `Confiança baixa · ${percent}%`, // art_review.conf_low
  accept: "Aceitar", // art_review.accept
  other: "Escolher outra", // art_review.other
  more: (shown: number, n: number) => `Mostrando ${shown} de ${n}; resolva estes para ver os outros`, // art_review.more
  accepted: (game: string) => `Capa de ${game} trocada`, // art_review.accepted
  acceptedMany: (n: number) => `${n} capas trocadas`, // art_review.accepted_many
  finish: "Concluir",
  back: "Voltar",
} as const;
