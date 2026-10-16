# Catálogo de ideias

Ideias do mantenedor (2026-10-16) para depois da paridade. Todas são **COULD** até serem priorizadas; ao entrar no plano, uma ideia vira item do PRD (§9) com critério de aceite. A coluna "Relação" aponta o item do PRD que já cobre parte dela.

Regras que valem para todas: o que expõe dados do usuário (listas públicas, perfil, estatísticas) é opt-in; nada roda durante o jogo sem motivo; downloads seguem a decisão do épico DL.

## Biblioteca e organização

| ID | Ideia | Relação |
|---|---|---|
| IDEA-LIB-01 | Bibliotecas inteligentes: coleções automáticas por gênero, tempo jogado, plataforma e outros critérios |  |
| IDEA-LIB-02 | Status personalizados: jogando, zerado, abandonado, quero jogar, pausado |  |
| IDEA-LIB-03 | HowLongToBeat: estimativa de tempo para terminar cada jogo | provedor de metadados |
| IDEA-LIB-04 | Roleta de jogos: sortear o próximo jogo da biblioteca |  |
| IDEA-LIB-05 | Detector de jogos quebrados: executáveis removidos, caminhos inválidos, instalações corrompidas |  |
| IDEA-LIB-06 | Gerenciador de armazenamento: quanto cada jogo ocupa, por disco |  |
| IDEA-LIB-07 | Edição em massa: nomes, categorias, capas e tags de vários jogos de uma vez |  |
| IDEA-LIB-08 | Tags personalizadas |  |
| IDEA-LIB-09 | Jogos não instalados: mostrar o que o usuário possui mesmo sem instalação local |  |
| IDEA-LIB-10 | Importar coleções e configurações do ES-DE, Pegasus e LaunchBox | SRC-001 |
| IDEA-LIB-11 | Gerenciador de DLCs: instaladas, disponíveis e ausentes |  |
| IDEA-LIB-12 | Biblioteca compartilhável: exportar uma lista pública dos jogos | opt-in |

## Controle e experiência de console

| ID | Ideia | Relação |
|---|---|---|
| IDEA-INPUT-03 | Perfis de controle por jogo | INPUT-002 |
| IDEA-INPUT-04 | Giroscópio: mapear movimento para ações ou mouse | INPUT-002 |
| IDEA-INPUT-05 | Menus radiais por combinação de botões |  |
| IDEA-INPUT-06 | Macros de controle, respeitando limites de jogos e anti-cheat | INPUT-002 |
| IDEA-INPUT-07 | Indicador de bateria dos controles conectados |  |
| IDEA-INPUT-08 | Perfis comunitários de controles: baixar e compartilhar | INPUT-002 |
| IDEA-INPUT-09 | Calibração de analógicos: deadzones, curvas e sensibilidade | INPUT-001 |
| IDEA-INPUT-10 | Múltiplos controles: identificar e configurar cada jogador |  |
| IDEA-INPUT-11 | Reconexão assistida: recuperar a navegação quando um controle desconectar |  |
| IDEA-INPUT-12 | Layout adaptativo: alternar sozinho entre interface de controle e de mouse/teclado |  |
| IDEA-INPUT-13 | Navegação por gestos no touchpad do DualSense e similares |  |
| IDEA-INPUT-14 | Teclado virtual avançado: sugestões, histórico e digitação por controle |  |
| IDEA-INPUT-15 | Modo sofá: aumentar a interface conforme distância e resolução | UX-001 |
| IDEA-INPUT-16 | Perfis de vibração (háptica) quando suportado |  |

## Execução de jogos e sessões

| ID | Ideia | Relação |
|---|---|---|
| IDEA-RUN-01 | Perfis de inicialização: variáveis de ambiente, parâmetros e scripts por jogo | EXT-001 |
| IDEA-RUN-02 | Proton/Wine: escolher versões e configurações de compatibilidade (Linux) |  |
| IDEA-RUN-03 | Detector de crashes com diagnóstico |  |
| IDEA-RUN-04 | Gerenciador de saves: localizar e fazer backup | CLOUD-001 |
| IDEA-RUN-05 | Histórico de sessões: quando e por quanto tempo cada jogo rodou |  |
| IDEA-RUN-06 | Quick Switch: trocar rápido entre jogos abertos, quando possível | SESSION-001 |
| IDEA-RUN-07 | Suspensão de sessões como alternativa limitada ao Quick Resume | SESSION-001 |
| IDEA-RUN-08 | Modo recuperação: restaurar uma sessão interrompida quando o jogo permitir |  |
| IDEA-RUN-09 | Ações pós-jogo, com autorização | EXT-001 |
| IDEA-RUN-10 | Perfis de compatibilidade por jogo e por SO |  |
| IDEA-RUN-11 | Capturas organizadas: screenshots e gravações por jogo numa galeria |  |
| IDEA-RUN-12 | Gerenciador de mods via Vortex e Mod Organizer 2 |  |
| IDEA-RUN-13 | Verificador de integridade: abrir o reparo da loja do jogo |  |

## Loja, descobertas e promoções

| ID | Ideia | Relação |
|---|---|---|
| IDEA-DEALS-02 | Wishlist universal, independente das lojas |  |
| IDEA-DEALS-03 | Histórico de preços nas lojas integradas | DEALS-001 |
| IDEA-DEALS-04 | Comparador de edições (Standard, Deluxe, Ultimate e DLCs) |  |
| IDEA-DEALS-05 | Alertas de preço |  |
| IDEA-DEALS-06 | Detector de duplicidade: avisar ao tentar comprar um jogo que já possui |  |
| IDEA-DEALS-07 | Filtro de DRM: ativação e launchers adicionais exigidos |  |
| IDEA-DEALS-08 | Comparador de assinaturas: Game Pass, EA Play e outros catálogos |  |
| IDEA-DEALS-09 | Alertas de saída de catálogo das assinaturas |  |
| IDEA-DEALS-10 | Jogos gratuitos: 100% de desconto e giveaways legítimos |  |
| IDEA-DEALS-11 | Recomendações pela biblioteca |  |
| IDEA-DEALS-12 | Listas comunitárias |  |
| IDEA-DEALS-13 | Curadoria de ofertas com região e histórico | DEALS-001 |

## Personalização e identidade visual

| ID | Ideia | Relação |
|---|---|---|
| IDEA-VIS-01 | Editor da tela inicial: reorganizar seções e componentes |  |
| IDEA-VIS-02 | Widgets: últimos jogos, conquistas, amigos, novidades, estatísticas |  |
| IDEA-VIS-03 | Temas dinâmicos: paleta segue o jogo selecionado | o tema padrão já usa a cor do jogo em foco |
| IDEA-VIS-04 | Modo ambiente: artes e animações com o launcher ocioso |  |
| IDEA-VIS-05 | Perfis visuais por dispositivo: TV, desktop, handheld |  |
| IDEA-VIS-06 | Visualização compacta em lista, para mouse e teclado |  |
| IDEA-VIS-07 | Editor de transições entre telas |  |
| IDEA-VIS-08 | Planos de fundo interativos |  |
| IDEA-VIS-09 | Visualizador musical | THEME-007 |
| IDEA-VIS-10 | Animações contextuais do Tako (downloads, conquistas, erros, inicialização) | BRAND-001 |
| IDEA-VIS-11 | Variações e skins do mascote | BRAND-001 |
| IDEA-VIS-12 | Pacotes de sons comunitários | THEME-007 |
| IDEA-VIS-13 | Perfis de UI: salvar e alternar disposições | THEME-006 |

## Inteligência artificial e automações

| ID | Ideia | Relação |
|---|---|---|
| IDEA-AI-02 | Busca semântica ("RPG sombrio com combate difícil") | AI-001 |
| IDEA-AI-03 | Organização automática por gênero, estilo e categoria | AI-001 |
| IDEA-AI-04 | Assistente de compatibilidade para configurar a execução |  |
| IDEA-AI-05 | Recomendação pelo tempo disponível | LIB HowLongToBeat |
| IDEA-AI-06 | Assistente de erros: ler logs e explicar falhas de inicialização |  |
| IDEA-AI-07 | Comandos em linguagem natural ("meus jogos de corrida instalados") | VOICE-001 |
| IDEA-AI-08 | Gerar coleções a partir de uma descrição |  |
| IDEA-AI-09 | Reconhecer o jogo por screenshot |  |
| IDEA-AI-10 | Identificador de ROMs difíceis | SRC-001 |
| IDEA-AI-11 | Assistente de instalação por lojas e fontes oficiais ou autorizadas |  |
| IDEA-AI-12 | IA local opcional, com modelos do próprio usuário |  |
| IDEA-AI-13 | Rotinas automáticas para organizar, verificar e atualizar a biblioteca |  |

## Emulação e retro

| ID | Ideia | Relação |
|---|---|---|
| IDEA-EMU-01 | RetroAchievements |  |
| IDEA-EMU-02 | Gerenciador de BIOS fornecidas pelo usuário: presença, versão, compatibilidade |  |
| IDEA-EMU-03 | Organizador de ROMs: regiões, revisões e versões |  |
| IDEA-EMU-04 | Perfis por console: controles, shaders e configurações |  |
| IDEA-EMU-05 | Gerenciador de save states |  |
| IDEA-EMU-06 | Netplay em emuladores compatíveis |  |
| IDEA-EMU-07 | Biblioteca de shaders (CRT, LCD) pelos emuladores |  |
| IDEA-EMU-08 | Capas por região (JP, US, EU) |  |
| IDEA-EMU-09 | Disc art, cartuchos e caixas 3D |  |
| IDEA-EMU-10 | Jogos multidisco como um único jogo |  |
| IDEA-EMU-11 | Quais emuladores instalados rodam cada plataforma |  |
| IDEA-EMU-12 | Scanner de pastas para novas ROMs |  |

## Social e comunidade

| ID | Ideia | Relação |
|---|---|---|
| IDEA-SOC-01 | Discord Rich Presence |  |
| IDEA-SOC-02 | Agregador de conquistas das lojas num perfil |  |
| IDEA-SOC-03 | Estatísticas pessoais e retrospectiva |  |
| IDEA-SOC-04 | Perfil público opcional | opt-in |
| IDEA-SOC-05 | Importar coleções de outras pessoas |  |
| IDEA-SOC-06 | Comparar bibliotecas com amigos |  |
| IDEA-SOC-07 | Modo Party: multiplayer local disponível |  |
| IDEA-SOC-08 | Notas e comentários privados |  |
| IDEA-SOC-09 | Diário de jogos: impressões, progresso, datas de conclusão |  |
| IDEA-SOC-10 | Modo família: perfis com restrições | UX-004 |

## Sistema e extensibilidade

| ID | Ideia | Relação |
|---|---|---|
| IDEA-SYS-01 | Atalhos para cloud gaming (GeForce NOW, Xbox Cloud Gaming) |  |
| IDEA-SYS-02 | Comandos `tako://` para abrir jogos e telas | ART-002 usa o mesmo protocolo |
| IDEA-SYS-03 | Stream Deck | EXT-002 |
| IDEA-SYS-04 | API de automação para Home Assistant e ferramentas locais | EXT-002 |
| IDEA-SYS-05 | Modo de segurança: abrir sem as personalizações que causam erro |  |
| IDEA-SYS-06 | Perfis de instalação: várias bibliotecas e ambientes na mesma máquina | UX-004 |
| IDEA-SYS-07 | Importar e exportar configurações em formato aberto | CLOUD-001 |
| IDEA-SYS-08 | Gerenciador de extensões | EXT-003 |
| IDEA-SYS-09 | Central de permissões das integrações | EXT-003 |
| IDEA-SYS-10 | Diagnóstico automático de configuração | PERF-001 |
| IDEA-SYS-11 | Limpeza de caches obsoletos e arquivos órfãos |  |
| IDEA-SYS-12 | Painel de novidades do Tako |  |
| IDEA-SYS-13 | Ambiente de testes para temas | THEME-002 |
