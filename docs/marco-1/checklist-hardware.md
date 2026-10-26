# Checklist de hardware (MVP-034)

Casos que o CI não cobre: precisam de monitores, áudio, controles, Steam e Windows reais. Rode numa versão **publicada** (instalador `Tako-Setup-x64.exe` ou `Tako-Portable-x64.exe` do artefato da PR ou da release), não pelo `dotnet run`: atualização automática e "abrir ao iniciar o Windows" só funcionam nas versões publicadas.

Como registrar: copie a tabela de resultados no fim deste arquivo para a PR ou issue, uma linha por máquina, e anexe o log do dia `logs\tako-AAAAMMDD.log` (pasta de dados, ver abaixo) quando algo falhar.

**Em toda execução, anexe o diagnóstico do sistema** (MVP-034): Configurações › Sistema › **Diagnóstico do sistema** › **Gerar**. O relatório traz versão e tipo de instalação do Tako, Windows/build, processador e memória, cada monitor (resolução, proporção, escala, Hz e placa de vídeo), controles detectados (XInput e PlayStation por HID), Steam e número de jogos, estado do Xbox mode com os sinais lidos e modo seguro. Não tem nome de usuário, nome do computador nem lista de jogos (a pasta do perfil aparece como `%USERPROFILE%`). **Copiar** põe o texto na área de transferência; o arquivo fica em `logs\diagnostico-AAAAMMDD-HHMMSS.txt`. Gere de novo depois de trocar monitor ou controle (casos M7/M8, C1–C8) para registrar o estado que o Tako viu.

- Pasta de dados: instalado → `%LOCALAPPDATA%\Tako`; portátil → `Tako_Data` ao lado do `.exe`.
- Marcador de sessão: `session.json` na pasta de dados (`Running: true` enquanto o Tako está aberto).

## Atalhos de emergência (sempre disponíveis)

| Situação | Caminho de volta ao desktop |
|---|---|
| Tela do Tako responde | Barra lateral › **Sair para o desktop**, ou **Alt+F4** (fecha a janela de tela cheia na hora, sem confirmação). |
| Erro inesperado | A tela **Algo deu errado** tem **Sair para o desktop** num passo só. Mais de 3 erros em 10 s fecham a tela cheia sozinhos. |
| Tela congelada | **Ctrl+Shift+Esc** (Gerenciador de Tarefas, aparece por cima: a janela do Tako não é "sempre visível") › finalizar `Tako.exe`. **Win** abre o menu Iniciar. |
| Travou ao abrir duas vezes seguidas | A terceira abertura entra em **modo seguro** sozinha. Para forçar: `Tako.exe --safe-mode`. |
| Tela de jogo em foco e sem imagem (TV desligada, monitor desconectado) | **Ctrl+Alt+Shift+D** em qualquer momento, inclusive com o jogo na frente ou o Tako ocupado: as telas voltam para a mesa (seção 19). |

## 1. Monitores e resolução

| # | Caso | Como testar | Esperado |
|---|---|---|---|
| M1 | 1080p 16:9 | Abrir o Tako num monitor 1920×1080 | Tela cheia sem faixas; texto nítido |
| M2 | 1440p (2K) | Monitor 2560×1440, escala 100% e 125% | Layout igual ao 1080p, imagens sem serrilhado |
| M3 | 4K | Monitor/TV 3840×2160, escala 150% e 200% | Texto e capas nítidos (sem desfoque de escala) |
| M4 | Ultrawide 21:9 | 2560×1080 ou 3440×1440 | Palco ocupa a largura toda; grade da biblioteca ganha colunas |
| M5 | 32:9 | 5120×1440 (se houver) | Sem faixas pretas; nada cortado |
| M6 | 16:10 | 1920×1200 ou portátil 1280×800 | Sem faixas; dicas da barra inferior visíveis |
| M7 | Dois monitores | Abrir o Tako com o monitor principal trocado | Abre no monitor principal, em tela cheia |
| M8 | Monitor desligado/trocado com o Tako aberto | Desligar o monitor secundário, mudar a resolução pelas Configurações do Windows | O Tako se reajusta; nenhuma tela preta presa |
| M9 | HDR ligado no Windows | Ligar HDR e abrir o Tako | Cores corretas, sem tela lavada |

## 2. Áudio

| # | Caso | Esperado |
|---|---|---|
| A1 | Fone Bluetooth conectado com o Tako aberto | Nenhum travamento; o áudio do Windows segue a saída padrão |
| A2 | Saída HDMI (TV) desconectada | Nenhum travamento |
| A3 | Central de Controle › Áudio | Painel abre e ajusta o volume (Marco 0: simulado; a restauração real após o jogo está na seção 9) |

## 3. Controles (MVP-031)

Teste cada um por USB e, quando houver, por Bluetooth.

| # | Caso | Como testar | Esperado |
|---|---|---|---|
| C1 | Controle Xbox (USB) | Conectar com o Tako aberto | Aviso "Controle conectado: Controle Xbox"; dicas mudam para A/B/X/Y |
| C2 | Controle Xbox (Bluetooth / adaptador sem fio) | Idem | Idem |
| C3 | DualSense (USB) | Conectar | Aviso "Controle PS5 (DualSense)"; dicas com ✕ ○ □ △ |
| C4 | DualSense (Bluetooth) | Parear e conectar | Idem C3 |
| C5 | DualShock 4 (USB e Bluetooth) | Idem | Aviso "Controle PS4 (DualShock 4)" |
| C6 | Desconectar no meio da navegação | Segurar → no direcional e puxar o cabo / desligar o controle | Aviso "Controle desconectado — o teclado continua funcionando"; o foco **para** (não fica andando sozinho); setas e Enter do teclado funcionam |
| C7 | Reconectar segurando um botão | Reconectar com → ou A pressionado | Nada acontece até soltar; depois funciona normalmente, **sem reiniciar o Tako** |
| C8 | Trocar de controle | Desligar o Xbox e ligar o DualSense | Aviso de troca; dicas mudam; nada acionado sozinho |
| C9 | Voltar de um jogo com A pressionado | Fechar um jogo com A e voltar ao Tako | O A que fechou o jogo não abre nada no Tako |
| C10 | Atalho da Central no controle | Durante um jogo simulado, segurar View + Y (Create + △ no DualSense, Share + △ no DualShock 4) por meio segundo | Abre a Central; segurar de novo fecha. Apertar rápido não abre. O Guia/PS sozinho não abre mais a Central |
| C11 | Sem controle nenhum | Usar só o teclado do início ao fim | Todas as telas alcançáveis (Ctrl+Shift+Tab abre a Central de Controle) |

## 4. Steam

| # | Caso | Esperado |
|---|---|---|
| S1 | Steam instalada, várias bibliotecas (outros discos) | Todos os jogos instalados aparecem |
| S2 | Steam não instalada | Início mostra como adicionar jogos; nada trava |
| S3 | Abrir um jogo da Steam | "Iniciando" e o jogo abre pela Steam |
| S4 | B durante "Iniciando" | Cancela de verdade: o jogo não abre depois |
| S5 | Steam fechada ao abrir um jogo | A Steam abre e o jogo inicia |

## 5. Atualização automática

| # | Caso | Esperado |
|---|---|---|
| U1 | Versão instalada antiga com release nova publicada | Baixa em segundo plano; pergunta "Reiniciar e atualizar" |
| U2 | "Depois (instala ao sair)" e fechar | Atualiza ao fechar, sem reabrir |
| U3 | Portátil | Troca o `.exe` ao fechar |
| U4 | Sem internet | Status "não foi possível procurar atualizações"; nada trava |
| U5 | Modo seguro | Nenhuma busca automática; volta depois de "Voltar ao normal" |

## 6. Modo seguro e recuperação (MVP-036 / MVP-021 / MVP-011)

| # | Caso | Como testar | Esperado |
|---|---|---|---|
| R1 | Uma saída suja | Abrir o Tako e finalizar `Tako.exe` no Gerenciador de Tarefas; abrir de novo | Abre normal (uma falha só não é loop) |
| R2 | Duas saídas sujas seguidas | Repetir R1 duas vezes; abrir a terceira | Aviso **Modo seguro**; tema padrão embutido; Configurações › Sistema › Modo seguro = "Ativo" |
| R3 | Voltar ao normal | No aviso ou em Configurações › Sistema › Modo seguro | Sem reiniciar: tema do disco e atualizações automáticas voltam |
| R4 | Fechar limpo no modo seguro | Sair para o desktop e abrir de novo | Abre normal |
| R5 | `--safe-mode` | `Tako.exe --safe-mode` | Modo seguro com o motivo "Pedido na linha de comando" |
| R6 | Tema do disco apagado | Apagar `Themes\Default\Tokens.xaml` da pasta do Tako | Abre com o tema embutido e tela de recuperação do tema |
| R7 | Pasta de dados somente leitura (portátil) | Portátil numa pasta sem permissão de escrita | Abre normal (sem proteção contra loop, sem travar) |
| R8 | Alt+F4 | Em qualquer tela, inclusive com diálogo aberto | Fecha a tela cheia e volta ao desktop |
| R9 | Registro de falhas | Depois de R1/R2 | O log do dia (`logs\tako-AAAAMMDD.log`) registra os erros reais; finalizar pelo Gerenciador de Tarefas não gera registro de erro |

## 7. Abrir ao iniciar o Windows (MVP-010)

| # | Caso | Esperado |
|---|---|---|
| W1 | Ligar em Configurações | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` ganha `Tako` = `"<caminho do Tako.exe>"`; ao entrar no Windows, o Tako abre em tela cheia |
| W2 | Desligar | O valor `Tako` some |
| W3 | Apagar o valor pelo `regedit` e abrir o Tako | A opção aparece desligada (reflete o registro) |
| W4 | Portátil movido de pasta | A opção aparece desligada até ligar de novo (o valor antigo aponta para outra cópia) |
| W5 | Configurações › Apps › Inicialização do Windows | "Tako" aparece e pode ser desligado por lá; o Tako reflete na próxima abertura |

## 8. Xbox mode / FSE (MVP-008)

Linha em Configurações › Sistema › **Tela cheia do Windows (Xbox mode)**.

| # | Máquina | Esperado |
|---|---|---|
| F1 | Windows 10 ou Windows 11 23H2 | **Indisponível** (exige 24H2) |
| F2 | Windows 11 24H2+ sem o app Xbox | **Indisponível** |
| F3 | Desktop/notebook 24H2+ com app Xbox | **Indeterminado** (rollout gradual da Microsoft; nunca "Suportado" sem sinal positivo) |
| F4 | Portátil de jogos (ROG Ally, Legion Go…) 24H2+ com app Xbox | **Suportado** |
| F5 | Comparar com Configurações › Jogos › Xbox mode do Windows | Se o Windows mostra o Xbox mode e o Tako diz "Indisponível", é falso negativo: anexar o diagnóstico do sistema (linha "Sinais": build, app Xbox, `DeviceForm`) e a saída do `Launcher.FseProbe` |

Em todos os casos o Tako abre em tela cheia convencional; registrar-se como app inicial do Xbox mode depende de autorização da Microsoft (ver `docs/spikes/MVP-007-fse.md`).

## 9. Restaurar monitores e áudio (MVP-012)

Opção em Configurações › Sistema › **Restaurar telas e som ao sair do jogo** (padrão: ligada). Decisões e riscos na [ADR-0013](../adr/0013-estado-de-monitores-e-audio.md); o CI só testa com monitores e áudio simulados.

Jogo de teste: adicione `C:\Windows\notepad.exe` como jogo, com fim do jogo por "Processo iniciado". Com o Bloco de Notas aberto pelo Tako, faça a mudança pelas Configurações do Windows (ou pelo ícone de som da barra de tarefas) e feche o Bloco de Notas. Repita os casos principais (E1, E2, E9) com um jogo real que troque a resolução em tela cheia exclusiva.

| # | Caso | Como testar | Esperado |
|---|---|---|---|
| E1 | Resolução | Durante o jogo, mudar a resolução do monitor principal | Ao voltar, resolução de antes; aviso "Bem-vindo de volta · … · monitores e áudio restaurados" |
| E2 | Taxa de atualização | 144 → 60 Hz (ou 60 → 59,94 Hz) | Taxa de antes, inclusive fracionária |
| E3 | Monitor principal | Dois monitores; trocar o principal durante o jogo | Principal e posições de antes |
| E4 | Rotação | Girar o monitor secundário para retrato | Volta para paisagem |
| E5 | Monitor desligado pelo jogo | Win+P › "Somente tela do PC" (ou "Somente segunda tela") durante o jogo | Os dois monitores voltam, com o mesmo arranjo |
| E6 | Monitor desconectado durante o jogo | Puxar o cabo do secundário e fechar o jogo | Nada trava; o log registra a falha/o que não pôde voltar; o principal fica como estava |
| E7 | Nada mudou | Jogar e fechar sem mexer em nada | Nenhuma piscada de tela; log "já estavam como antes" |
| E8 | HDR | HDR ligado antes; jogo desliga (ou ao contrário) | Anotar o resultado (HDR não faz parte do estado restaurado nesta versão) |
| E9 | Saída padrão | Fone/TV como padrão; durante o jogo trocar para alto-falantes | Saída de antes nos três papéis (sons do sistema, multimídia, comunicação) |
| E10 | Volume e mudo | Volume em 30%; durante o jogo pôr 100% e mudo | 30% e com som, sem pico de volume |
| E11 | Fone desconectado durante o jogo | Fone Bluetooth padrão antes; desligar o fone durante o jogo | Nada trava; log "não está conectado"; volume dos outros dispositivos restaurado |
| E12 | `IPolicyConfig` indisponível | (Se uma atualização do Windows quebrar a troca de saída) | Volume/mudo continuam sendo restaurados; o log diz "trocar o dispositivo padrão não está disponível" |
| E13 | Opção desligada | Desligar a opção e repetir E1/E9 | Nada é restaurado |
| E14 | Queda durante o jogo | Mudar a resolução durante o jogo, finalizar `Tako.exe` no Gerenciador de Tarefas, fechar o jogo e abrir o Tako | Pergunta "Restaurar monitores e áudio?" com **Restaurar monitores e áudio como estavam antes do jogo**; ao aceitar, tudo volta; "Manter como está" não muda nada e não pergunta de novo |
| E15 | Queda sem diferença | Como E14, mas sem mudar nada | Nenhuma pergunta |
| E16 | Reinício do PC no meio | Como E14, mas reiniciar o Windows antes de abrir o Tako | A pergunta aparece; se a restauração exata falhar (identificadores dos adaptadores mudam), resolução/taxa/posição voltam monitor a monitor; o log registra o caminho usado |
| E17 | Log | Depois de E1–E11 | `logs\tako-AAAAMMDD.log` tem "registrados antes de", cada passo restaurado/mantido e "verificada" ou "incompleta" |
| E18 | Arquivo pendente | Durante o jogo | `session-environment.json` existe na pasta de dados; some depois de voltar |

## 9b. Tela de jogo (MVP-002, [ADR-0015](../adr/0015-monitores-do-console-mode-e-tela-de-jogo.md))

Modo Console › **Tela de jogo** (padrão: desligada). Use um PC com monitor + TV (ou dois monitores) e o mesmo jogo de teste da seção 9. Anexe `logs\tako-AAAAMMDD.log` (linhas "Tela de jogo").

| # | Caso | Como testar | Esperado |
|---|---|---|---|
| CJ1 | Escolher a tela | Ligar a opção; ativar "Tela onde você joga" várias vezes | Sugere a tela que não é a principal; passa por todas as conectadas, inclusive a TV desligada da área de trabalho |
| CJ2 | TV desligada da área de trabalho | TV conectada mas fora da área de trabalho (Win+P "Somente tela do PC"); abrir um jogo | A TV liga, vira principal, o monitor desconecta, o Tako vai para a TV; ao fechar o jogo tudo volta como estava (TV fora da área de trabalho) |
| CJ3 | Confirmação | Primeira vez com G2 | "Você consegue ver esta tela?" na TV, com contagem de 15 s; A/Enter/clique em "Sim, manter" abre o jogo; na próxima vez não pergunta |
| CJ4 | Sem resposta | Como G3, mas sem responder (ou B/Esc, ou "Voltar ao normal") | As telas voltam sozinhas; aviso "Tela de jogo não confirmada"; o jogo não abre |
| CJ5 | Cobrir de preto | "Outras telas" › Cobrir de preto | O monitor fica preto (ligado), sem roubar o foco do jogo; some ao fechar o jogo |
| CJ6 | Resolução e taxa | Escolher um modo (ex.: 3840 × 2160 · 120 Hz) | Aplicado na TV durante o jogo; volta ao fechar; trocar o modo pede confirmação de novo |
| CJ7 | Modo recusado | Escolher um modo que a TV não aceita | O jogo abre no modo atual da TV; o log registra a recusa |
| CJ8 | HDR | "Ligar HDR" numa TV com HDR desligado | HDR liga durante o jogo e desliga ao sair; numa tela sem HDR, nada acontece e o log diz "HDR não disponível" |
| G9 | TV que não acorda | TV desligada da tomada/sem sinal | Depois de ~8 s o Tako desiste, as telas ficam como estavam e o jogo abre na tela atual |
| G10 | Tela desconectada | Escolher a TV e desconectar o cabo | Aviso "a tela de jogo escolhida não está conectada · o jogo abre na tela atual" |
| G11 | Jogo que não abre / sem acompanhamento | Atalho inválido; URI sem pasta | As telas voltam logo |
| G12 | Queda com a tela de jogo em foco | Finalizar `Tako.exe` durante o jogo e abrir de novo | Com "Restaurar monitores e áudio" ligado, aparece a oferta de restauração (seção 9, E14) e as telas voltam (com "Restaurando monitores e áudio…", sem congelar) |
| G13 | Restauração sem a configuração exata | Como G12, mas reiniciar o Windows antes | A restauração religa os monitores que estavam ligados e desconecta a TV; o log registra o caminho usado |

## 10. Supervisor (MVP-006, [ADR-0012](../adr/0012-supervisor-e-ipc.md))

| # | Caso | Esperado |
|---|---|---|
| S1 | Abrir o Tako (instalado e portátil) | Dois processos `Tako.exe` no Gerenciador de Tarefas (supervisor sem janela + UI); `logs/supervisor/` com "UI iniciada" |
| S2 | Fechar o Tako normalmente / "Sair para o desktop" | Os dois processos saem |
| S3 | Encerrar só a UI (Detalhes › Finalizar tarefa no PID da UI) | A UI reabre em ~1 s; na 4ª vez em 5 min abre em modo seguro; na 5ª o supervisor desiste e fica o desktop |
| S4 | Travar a UI (ex.: pausar o processo da UI com o Process Explorer > 20 s) | O supervisor encerra e reabre a UI; o log registra "sem heartbeat" |
| S5 | Atualização portátil e instalada (U1–U3) com o supervisor | Nenhum `Tako.exe` reaberto no meio da troca; depois de atualizar, abre de novo pelo supervisor |
| S6 | "Abrir ao iniciar o Windows" ligado, reiniciar o PC | Abre pelo supervisor (dois processos) |
| S7 | Sair do Windows (logoff) com o Tako aberto | O supervisor não tenta reabrir a UI |
| S8 | `Tako.exe --no-supervisor` | Só a UI, sem supervisor |
| S9 | E14 com o supervisor (finalizar só a UI durante o jogo) | A UI reabre sozinha e faz a pergunta "Restaurar monitores e áudio?" |

## 11. Central de Controle sobre o jogo (MVP-030)

Com um jogo real aberto pelo Tako (ex.: um jogo da Steam em tela cheia sem bordas e outro em tela cheia exclusiva).

| # | Caso | Esperado |
|---|---|---|
| CJ1 | Atalho da Central (segurar View + Y, ou Ctrl+Shift+Tab) durante o jogo | Em janela ou tela cheia sem bordas: a Central abre por cima do jogo, que continua visível e rodando atrás (ver §23). Em tela cheia exclusiva: o Tako vem para a frente com a Central sobre a arte do jogo. B, o atalho ou "Voltar ao jogo" devolvem o jogo. A Barra de Jogo do Xbox **não** abre junto |
| CJ2 | Janelas abertas | A fileira lista as janelas do Alt+Tab com ícones; A alterna para uma (o jogo segue aberto); X fecha (um programa com algo não salvo pergunta antes) |
| CJ3 | Encerrar jogo (jogo real) | O jogo recebe o pedido de fechar e volta à frente; ao fechar, "Bem-vindo de volta" |
| CJ4 | Desempenho | Cartão e painel mostram CPU, GPU (com temperatura, se o driver informar) e RAM próximos do Gerenciador de Tarefas |
| CJ5 | Contador de FPS desligado | Painel de Desempenho mostra as opções de FPS esmaecidas com "Ative o contador de FPS nas Configurações do Tako, com teclado e mouse."; nada trava |
| CJ6 | Contador de FPS ligado | Ver §14 (contador próprio, sem RTSS) |
| CJ7 | Limite de FPS | Pelo driver de vídeo: ver §13 |
| CJ8 | Ponteiro | Esconder some com o ponteiro em todos os apps e ele volta ao fim da sessão; mover com o analógico funciona com Xbox e com DualSense; A, A duplo e segurar A clicam, clicam com o direito e arrastam |
| G9 | Queda com o ponteiro escondido ou o contador de FPS medindo | Finalizar `Tako.exe` e abrir de novo: ponteiro visível; `PresentMon.exe` fechado e `logman query -ets` sem a sessão `Tako_FPS` |
| G10 | ControlFS | Instalado: abre por cima do jogo; não instalado: "Baixar o ControlFS" abre a página |

## 12. Áudio, Steam e TV durante o jogo (MVP-002, [ADR-0016](../adr/0016-audio-steam-e-tv-na-sessao.md))

Modo Console › **Áudio do jogo**, **Steam na bandeja** e **TV** (TV: Google TV / Android TV com "Depuração USB" ou "Depuração pela rede" ativada nas Opções do desenvolvedor).

| # | Caso | Como testar | Esperado |
|---|---|---|---|
| A1 | Saída do jogo | Escolher a TV (ou um fone) como saída do jogo; abrir e fechar um jogo | Durante o jogo o som sai pela escolhida nos três papéis; ao fechar, volta a de antes |
| A2 | Troca durante o jogo | Como A1, mas trocar a saída pelo Windows durante o jogo | Ao fechar, a escolha feita durante o jogo fica (só os papéis ainda na saída do jogo voltam) |
| A3 | Saída que aparece com a TV | TV desligada, saída do jogo = áudio HDMI da TV, TV configurada | A troca acontece assim que a TV liga (até 15 s); log "saída padrão trocada" |
| A4 | Saída ausente | Saída do jogo desconectada | Nada muda; log "não aplicada (DeviceMissing)" |
| A5 | Restaurar ligado e desligado | Repetir A1 com "Restaurar monitores e áudio" ligada e desligada | Mesma saída de antes nos dois casos |
| S1 | Início pela Steam | Jogo da Steam cujo exe fica fora da pasta (ou com anti-cheat) | O Tako entra em "Jogando" quando a Steam informa o jogo |
| S2 | Fim pela Steam | Fechar um jogo que deixa processo para trás (ex.: UnityCrashHandler) | O Tako volta em ~4 s depois da Steam zerar o `RunningAppID` |
| S3 | Steam sem pasta | Jogo da Steam sem pasta de instalação conhecida | Acompanhado só pela Steam; tempo de jogo registrado |
| S4 | Bandeja | Opção ligada; fechar um jogo da Steam com a janela da Steam aberta | A janela da Steam fecha e ela continua na bandeja; o jogo nunca recebe o pedido |
| S5 | Outro jogo | Como S4, com outro jogo da Steam ainda aberto | A Steam fica aberta; log "GameRunning" |
| T1 | Pareamento | Modo Console › TV: IP, **Testar** | A TV pergunta "Permitir depuração"; com "Sempre permitir", a TV liga e muda para a entrada do PC |
| T2 | Testar de novo | **Testar** outra vez | Sem pergunta na TV; liga e troca a entrada |
| T3 | Início do jogo | TV em espera, controle da TV ligado; abrir um jogo | A TV liga na entrada do PC; o jogo abre normalmente mesmo se a TV demorar |
| T4 | Espera profunda | TV que desliga a rede em espera; MAC preenchido | Wake-on-LAN acorda a TV; log "Wake-on-LAN enviado" |
| T5 | Espera ao sair | "Colocar a TV em espera ao sair do jogo" ligado | Ao fechar o jogo a TV entra em espera |
| T6 | TV fora da rede | IP errado | **Testar** mostra "A TV em … não respondeu"; abrir um jogo não trava |
| T7 | Depuração sem fio | TV só com "Depuração sem fio" | **Testar** explica como ativar "Depuração USB"/"pela rede" |
| T8 | Chave cifrada | Copiar a pasta `Tako_Data` para outro usuário/PC e testar | A TV pede permissão de novo (chave nova); `adbkey.pem` começa com `dpapi:` |

## 13. Limite de FPS pelo driver de vídeo ([ADR-0018](../adr/0018-limite-de-fps-pelo-driver.md))

Uma máquina de cada fabricante: **NVIDIA** (GeForce com driver recente), **AMD** (Radeon dedicada e, se possível, um portátil com APU, como ROG Ally/Legion Go) e **Intel** (Arc ou Iris Xe). Para conferir o FPS use o contador do Tako (F1) ou o do próprio driver (NVIDIA FrameView/overlay, AMD Adrenalin, Intel Arc Control). Anotar o modelo da placa e a versão do driver.

| # | Caso | Como testar | Esperado |
|---|---|---|---|
| L1 | Item na Central | Abrir um jogo; Central › Desempenho | "Limite de FPS" diz o fabricante e "só neste jogo" (NVIDIA, Intel por jogo) ou "em toda a placa" (AMD); com o driver sem a biblioteca, "indisponível" e o toque explica |
| L2 | Aplicar | Escolher 60 FPS com o jogo aberto | O contador fica em ~60. **Anotar** se valeu na hora ou só depois de reabrir o jogo (NVIDIA e Intel por perfil) |
| L3 | Valores | Passar por 30, 40, 45, 90, 120, 144 | Cada valor vale; acima da taxa da tela sem VRR o FPS fica preso na taxa da tela |
| L4 | Tela − 3 (VRR) | Monitor/TV de 120 ou 144 Hz com G-Sync/FreeSync; escolher "Tela − 3" | Mostra "Tela − 3 (117 FPS)" (ou 141); o FPS fica 3 abaixo da taxa e o VRR continua ativo (OSD do monitor) |
| L5 | Valor do usuário volta | Antes: pôr um limite próprio no painel do driver (ex.: 75 no Painel NVIDIA para o jogo; FRTC/Chill no Adrenalin; Intel Arc Control). Abrir o jogo pelo Tako, mudar para 60, fechar o jogo | O painel do driver mostra de novo o valor de antes. NVIDIA sem perfil antes: o perfil "Tako · jogo.exe" some |
| L6 | Sem limite | Com 60 aplicado, escolher "Sem limite" | O valor do usuário volta na hora |
| L7 | Restauração após queda | Aplicar 30 FPS; finalizar `Tako.exe` no Gerenciador de Tarefas; fechar o jogo; abrir o Tako de novo | O valor de antes volta (conferir no painel do driver); `Tako_Data\driver-fps-limit.json` some |
| L8 | Afterburner aberto | MSI Afterburner + RTSS abertos com o OSD deles; aplicar e tirar o limite | Nada conflita: o OSD do Afterburner continua; o limite do RTSS (se houver) não é tocado pelo Tako |
| L9 | Sem administrador | Tako aberto como usuário comum (padrão) | NVIDIA: **anotar** se o limite grava; se o log disser "exige administrador", o item mostra "Indisponível sem permissão de administrador" e nenhum UAC aparece |
| L10 | Notebook híbrido | Intel + NVIDIA; jogo com "Alto desempenho" em Configurações › Tela › Gráficos | O limite vai para a NVIDIA; com "Economia de energia", para a Intel. **Anotar** o que acontece sem preferência |
| L11 | Jogo da Steam | Jogo da Steam (exe só conhecido quando o processo aparece) | O limite é aplicado quando o jogo aparece; **anotar** se precisou reabrir o jogo |
| L12 | Duas placas Intel | Arc + integrada | O limite vale no jogo e volta nas duas ao fechar |

## 14. Contador de FPS sem RTSS ([ADR-0017](../adr/0017-contador-de-fps-com-presentmon.md))

Contador próprio: o PresentMon da Intel (junto com o Tako) lê os eventos de apresentação do Windows e o Tako desenha
uma janela transparente por cima do jogo. Nada é injetado no jogo. Ligar exige que a conta esteja no grupo
"Usuários de log de desempenho" (uma vez, com UAC). Usar a versão publicada (instalador ou portátil): rodando pelo
código-fonte, o PresentMon não vem junto.

| # | Caso | Como testar | Esperado |
|---|---|---|---|
| F1 | Com controle nas Configurações | Usar só o controle e ir em Modo Console › Contador de FPS › Mostrar FPS | Linha esmaecida com "Use teclado e mouse para ativar"; A só mostra o aviso, nenhum UAC abre |
| F2 | Ligar com teclado e mouse | Clicar em Mostrar FPS | Explica "Para medir o FPS sem tocar no jogo, o Windows precisa liberar o Tako uma única vez." e, em Continuar, abre o UAC |
| F3 | Usuário padrão | Conta sem administrador; no UAC digitar a senha de outro administrador | A conta **que pediu** entra no grupo (conferir com `net localgroup "Usuários de log de desempenho"` ou o nome do grupo no idioma do Windows); a conta do administrador não |
| F4 | Sair e entrar | Depois de F2/F3, abrir um jogo sem sair do Windows | Configurações mostra "Saia e entre no Windows de novo para o contador funcionar."; depois de sair e entrar, o contador funciona |
| F5 | UAC com controle | Windows 11 em portátil (ROG Ally / Xbox Ally, Legion Go) só com controle | Anotar se o UAC aceita o controle; com o controle o Tako não deixa abrir o UAC (F1) |
| F6 | Instalador com a caixa marcada | Instalar com "Ativar contador de FPS (o Windows vai pedir permissão)" marcado | No fim, um UAC; Mostrar FPS já ligado ao abrir o Tako; depois de sair e entrar, funciona. Com a caixa desmarcada (padrão), nenhum UAC |
| F7 | Atualização silenciosa | Atualizar pelo Tako uma instalação feita com a caixa marcada | Nenhum UAC durante a atualização |
| F8 | Estilos | Na Central: Só FPS, Compacto, Detalhado, Desligado; posição, tamanho e fundo escuro | Muda na hora sobre o jogo; Detalhado mostra tempo de quadro, 1% low e API (DXGI, D3D9…) |
| F9 | Telas 2K e 4K | Monitor 1440p e 2160p com escala 125%, 150% e 200%; jogo no segundo monitor | Contador no canto certo do monitor do jogo, texto nítido e no mesmo tamanho físico |
| F10 | Janela do contador | Durante o jogo, clicar onde está o contador; Alt+Tab | O clique vai para o jogo; o contador não aparece no Alt+Tab nem rouba o foco |
| F11 | Tela cheia exclusiva | Jogo em tela cheia exclusiva (não "sem bordas") | O contador não aparece sobre o jogo (limite do Windows); a Central mostra "FPS do jogo" com números |
| F12 | Precisão | Comparar com CapFrameX ou FrameView no mesmo jogo | FPS e 1% low parecidos (±2 FPS em cena parada) |
| F13 | Convivência | Afterburner + RTSS abertos com OSD, e CapFrameX capturando | Os três seguem funcionando; o Tako não muda nada no RTSS/Afterburner; `logman query -ets` mostra a sessão `Tako_FPS` ao lado das deles |
| F14 | Fim da sessão | Fechar o jogo | `PresentMon.exe` fecha junto e a sessão `Tako_FPS` some |
| F15 | Queda do Tako | Finalizar `Tako.exe` (Gerenciador de Tarefas) com o contador ligado | `PresentMon.exe` fecha junto (Job Object); ao abrir o Tako de novo, a sessão `Tako_FPS` some |
| F16 | Anti-cheat | Um jogo com Easy Anti-Cheat e um com BattlEye, contador ligado | O jogo abre e roda sem aviso nem expulsão (o PresentMon não entra no processo do jogo); anotar jogo e versão |
| F17 | Remover permissão | Modo Console › Contador de FPS › Remover a permissão do Windows (teclado e mouse) | UAC; a conta sai do grupo; o contador fica desligado |

## 15. Jogos do Xbox / PC Game Pass (STORE-004)

O Tako lê os jogos instalados pelo app Xbox sem contornar o pacote: as pastas de jogos de cada unidade (arquivo
oculto `.GamingRoot`, normalmente `X:\XboxGames\<jogo>\Content\MicrosoftGame.config`) e os pacotes MSIX/AppX
registrados para o usuário que são jogos (`MicrosoftGame.config`, `xboxservices.config` ou registrados pelos Serviços
de Jogos). Abre sempre pelo AUMID (`shell:AppsFolder\<família>!<app>`), nunca pelo `.exe`. O fim do jogo é visto
pelos processos dentro da pasta do pacote. Precisa de conta com PC Game Pass (ou um jogo comprado na Microsoft Store)
e do app Xbox com os Serviços de Jogos.

Limitações conhecidas: jogos que rodam um launcher fora da pasta (EA app, Ubisoft Connect) só contam como jogando
quando o `.exe` do jogo abre; jogos UWP antigos sem Xbox Live (sem `xboxservices.config`) não aparecem; nomes vêm do
manifesto e, se ele usar recurso de idioma que o Windows não resolver, do nome da pasta.

| # | Caso | Como testar | Esperado |
|---|---|---|---|
| X1 | Jogo do Game Pass em `C:\XboxGames` | Instalar um jogo do PC Game Pass pelo app Xbox; Configurações › Biblioteca e lojas › Procurar jogos instalados | Aparece em Biblioteca › Xbox com o nome certo; a linha da loja diz "N jogos instalados" |
| X2 | Outra unidade | Escolher `D:` como pasta de jogos no app Xbox e instalar um jogo lá | Aparece também (lido pelo `.GamingRoot` de `D:\`) |
| X3 | Jogo antigo em WindowsApps | Um jogo UWP com Xbox Live (ex.: Microsoft Solitaire Collection) | Aparece; apps comuns da Store (ex.: Calculadora, Fotos) não aparecem |
| X4 | Arte e dados | Abrir os detalhes de X1 depois de alguns segundos | Capa e fundo da Steam quando o nome casar; sem correspondência, o logo do pacote; Opções › Corrigir correspondência funciona |
| X5 | Abrir | Jogar em X1 | O jogo abre pelo Windows (sem erro de acesso); a tela mostra "aguardando o jogo iniciar" e depois "Jogando" |
| X6 | Sessão | Jogar alguns minutos, abrir a Central de Controle (Guia) | Central sobre o jogo; contador de FPS acompanha o processo do jogo |
| X7 | Fim | Fechar o jogo pelo menu do próprio jogo | Em até ~5 s o Tako volta; "Jogados recentemente" e tempo de jogo atualizados |
| X8 | Desligar a fonte | Configurações › Biblioteca e lojas › Xbox (PC Game Pass) › desligar | Os jogos do Xbox somem da biblioteca; ligar de novo traz de volta |
| X9 | Sem app Xbox | Numa máquina sem o app Xbox (ou após desinstalá-lo) | A linha "Xbox (PC Game Pass)" avisa "App Xbox não encontrado…"; nada trava |
| X10 | Sem Serviços de Jogos | Parar/remover os Serviços de Jogos (ou máquina recém-formatada só com o app Xbox) | Aviso "Serviços de Jogos não encontrados: abra o app Xbox para instalá-los" |
| X11 | Jogo desinstalado | Desinstalar X1 pelo app Xbox e procurar de novo | O jogo some (pasta que sobrar sem pacote registrado não aparece) |
| X12 | Outro usuário do Windows | Entrar com outra conta do Windows no mesmo PC | Só aparecem os jogos registrados para essa conta |

## 16. TVs LG (webOS) e Samsung (Tizen) (MVP-002, [ADR-0016](../adr/0016-audio-steam-e-tv-na-sessao.md))

Modo Console › **TV** com **Tipo: LG (webOS)** ou **Tipo: Samsung (Tizen)**. Precisa de
uma LG e de uma Samsung reais (Samsung 2016 em diante; anotar modelo e ano). Na LG: "LG Connect Apps" ligado e, para
ligar a TV desligada, "Ligar via Wi-Fi" + MAC. Na Samsung: "Acesso de dispositivos IP remotos" ligado. HDMI-CEC fica
fora (exige adaptador).

| # | Caso | Como testar | Esperado |
|---|---|---|---|
| L1 | Detectar | IP da LG, **Detectar pelo IP** (TV ligada) | "Encontrada: LG (webOS)"; o campo de comando avançado some |
| L2 | Pareamento | **Testar** pela primeira vez | A TV mostra o aviso do "LG Remote App"; com "Aceitar", troca para a HDMI escolhida; log "a TV aceitou este PC" |
| L3 | Testar de novo | **Testar** outra vez | Sem aviso; troca a entrada |
| L4 | Entrada inexistente | Escolher uma HDMI que a TV não tem | Mensagem com as entradas da TV (nomes da própria TV) |
| L5 | Recusar | Apagar `tv-pairing.dat`, **Testar** e escolher "Recusar" na TV | "A TV LG recusou o Tako…" |
| L6 | Recurso desligado | Desligar "LG Connect Apps", **Testar** | "…recusou a conexão. Nas Configurações da TV, ligue LG Connect Apps…" |
| L7 | Início do jogo com a TV em espera | TV desligada pelo controle remoto, MAC preenchido; abrir um jogo | Wake-on-LAN liga a TV e ela vai para a entrada do PC; anotar o tempo no log |
| L8 | Espera ao sair | "Colocar a TV em espera ao sair do jogo" ligado | Ao fechar o jogo a TV desliga |
| L9 | Porta antiga | LG de 2016–2017 (só `ws://3000`), se houver | Funciona igual (o Tako tenta 3001 e depois 3000) |
| SG1 | Detectar | IP da Samsung, **Detectar pelo IP** | "Encontrada: Samsung (Tizen)" |
| SG2 | Pareamento | **Testar** pela primeira vez | A TV pergunta se permite o "Tako"; com "Permitir", vai para a HDMI escolhida |
| SG3 | Testar de novo | **Testar** outra vez | Sem pergunta; troca a entrada (token salvo) |
| SG4 | Recusar | Na TV, Gerenciador de conexão de dispositivo › Lista de dispositivos: negar o Tako; **Testar** | "A TV Samsung recusou o Tako…" |
| SG5 | Recurso desligado | Desligar "Acesso de dispositivos IP remotos", **Testar** | Mensagem pedindo para ligar o recurso |
| SG6 | Em espera com rede | TV desligada pelo controle remoto (modelo 2018+), sem MAC; abrir um jogo | A tecla Power liga a TV e a tecla HDMI troca a entrada |
| SG7 | Espera profunda | Mesma coisa com MAC preenchido | Wake-on-LAN liga a TV; a tecla Power **não** é mandada de novo (a TV não desliga) |
| SG8 | Espera ao sair | Opção ligada; fechar o jogo; repetir com a TV já desligada antes de fechar | Desliga só se estava ligada (nunca religa a TV) |
| SG9 | Tecla HDMI | Anotar se `KEY_HDMI1..4` troca a entrada no modelo testado | Se não trocar, anotar o modelo (algumas Samsung só aceitam `KEY_SOURCE`/`KEY_HDMI`) |
| TV1 | Chaves cifradas | Copiar `Tako_Data` para outro usuário/PC e **Testar** nas duas TVs | As duas TVs pedem permissão de novo; `tv-pairing.dat` começa com `dpapi:` |
| TV2 | Certificado | Conferir no log que a conexão LG usou `wss://IP:3001` e a Samsung `wss://IP:8002` | Funciona com o IP; com um nome (ex.: `tv.lan`) as Configurações pedem o IP |
| TV3 | Detectar sem TV | IP sem TV, **Detectar pelo IP** | "Nenhuma TV conhecida respondeu…" em poucos segundos |

## 17. Epic Games e GOG ([ADR-0019](../adr/0019-fontes-de-biblioteca.md))

Precisa de contas reais com jogos **instalados**. Os jogos são lidos sem as lojas abertas; a Epic sempre abre pelo
próprio launcher; o GOG abre pelo GOG Galaxy quando instalado, senão pelo executável do jogo.

| # | Caso | Como testar | Esperado |
|---|---|---|---|
| EG1 | Jogos da Epic | Epic instalada com 2+ jogos (um em outro disco), Epic **fechada**; abrir o Tako | Todos aparecem em "Epic Games" com selo e cor da Epic; DLCs e o Unreal Engine não aparecem |
| EG2 | Abrir pela Epic | Jogar um jogo da Epic com a Epic fechada | A Epic abre (login/DRM) e o jogo inicia; "Abrindo … pela Epic Games"; o Tako reconhece o jogo rodando e o fim ao fechar |
| EG3 | Jogo com anti-cheat ou launcher próprio | Ex.: jogo que abre um launcher antes | Sessão acompanhada pela pasta do jogo; anotar se o fim foi reconhecido |
| EG4 | Jogo movido | Mover um jogo pela Epic (Gerenciar › Mover) e "Procurar jogos instalados" | O jogo continua lá, com a pasta nova (Opções › Abrir pasta) |
| EG5 | Desinstalar | Desinstalar um jogo pela Epic e "Procurar jogos instalados" | O jogo some da biblioteca |
| EG6 | Epic ausente | PC sem a Epic (ou desinstalada, manifests apagados) | Configurações › Biblioteca e lojas › Epic Games: "Epic Games Launcher não encontrado neste PC"; nada trava |
| EG7 | Epic removida com manifests | Desinstalar só o launcher, deixando os jogos | Jogos aparecem; Configurações avisam; Jogar mostra "Epic Games Launcher não encontrado: instale-o para abrir …" |
| GG1 | Jogos do GOG Galaxy | Galaxy com 2+ jogos instalados, Galaxy fechado | Aparecem em "GOG" com selo e cor do GOG; DLCs não aparecem |
| GG2 | Abrir pelo Galaxy | Jogar um jogo do GOG | Abre pelo Galaxy ("pelo GOG Galaxy"); nuvem sincroniza; o fim da sessão é reconhecido |
| GG3 | Instalador offline sem Galaxy | Instalar um jogo pelo instalador offline do GOG num PC sem Galaxy | O jogo aparece; abre direto pelo executável com os argumentos do jogo; Configurações › Biblioteca e lojas › GOG: "Sem GOG Galaxy: os jogos abrem direto pelo executável" |
| GG4 | Desinstalar | Desinstalar um jogo do GOG e "Procurar jogos instalados" | O jogo some |
| LJ1 | Mesmo jogo em duas lojas | Um jogo instalado na Steam e na Epic (ou GOG) | Dois itens, cada um com a sua loja; detalhes mostram "Também em"; cada um abre pela própria loja |
| LJ2 | Capas e dados | Jogos da Epic/GOG recém-encontrados | Capa, descrição e gêneros chegam pelo agente de metadados; "Opções › Corrigir correspondência" troca a escolha |
| LJ3 | Desligar lojas | Configurações › Biblioteca e lojas › Epic Games (A) e depois de novo | Os jogos da Epic somem e voltam; favoritos e tempo de jogo continuam |
| LJ4 | Só controle | Fazer LJ3 só com o controle | Linhas das lojas alcançáveis e alternáveis com A |

## 18. Configurações deste jogo (UX-007)

Detalhes do jogo › Configurações deste jogo (ou ☰ numa capa). Cada opção começa em "Padrão: ..." (o valor das
Configurações do Tako) e só vale para aquele jogo. Precisa de tela de jogo escolhida (seção 9b), duas saídas de áudio e
GPU com limite pelo driver (seção 13).

| # | Caso | Como testar | Esperado |
|---|---|---|---|
| CJ1 | Tela de jogo só neste jogo | Tela de jogo desligada nas Configurações; no jogo A, "Usar a Tela de jogo: Sim"; abrir A e depois B | A abre na TV (com a pergunta "Você consegue ver esta tela?" na primeira vez); B abre na tela atual |
| CJ2 | Não usar neste jogo | Tela de jogo ligada; no jogo A, "Não" | A abre na tela atual, sem mexer nos monitores |
| CJ3 | Resolução e HDR por jogo | No jogo A, 1920 × 1080 · 60 Hz e HDR desligado; abrir A, fechar, abrir B (padrão 4K/HDR) | A abre em 1080p sem HDR; B em 4K com HDR. Ao fechar cada um, as telas voltam como estavam. A pergunta de confirmação aparece só na primeira vez de cada resolução |
| CJ4 | Saída de áudio | Padrão = TV; no jogo A, "Fone"; no jogo B, "Não alterar" | A toca no fone; B não troca a saída; ao fechar, o som volta para a saída de antes |
| CJ5 | Limite de FPS | Padrão sem limite; no jogo A, 60 FPS | A fica em 60 FPS (painel do driver mostra o perfil do jogo); B sem limite. Mudar o limite na Central durante A vale só para a sessão (o padrão do Tako não muda) |
| CJ6 | Mostrar FPS | Contador desligado nas Configurações (com a permissão já dada); no jogo A, "Mostrar FPS: Sim" | O contador aparece só em A |
| CJ7 | Argumentos extras | Jogo adicionado (.exe) com "-windowed" (ou outro argumento conhecido do jogo) | O jogo abre com o argumento (Gerenciador de Tarefas › coluna Linha de comando) |
| CJ8 | Voltar ao padrão | "Usar tudo do Tako" e abrir o jogo de novo | Comporta-se como os outros jogos |

## 19. Modo console ao abrir (MVP-002/012/036, [ADR-0015](../adr/0015-monitores-do-console-mode-e-tela-de-jogo.md))

Modo Console › **Tela de jogo** ligada (seção 9b) e **Ao abrir o Tako, ir para a tela de jogo** (padrão: desligada). Mesmo PC da seção 9b (monitor + TV), de preferência com a TV da seção 12 configurada. Anexe `logs\tako-AAAAMMDD.log` (linhas "Modo console") e, nos casos de queda, `logs\supervisor\tako-AAAAMMDD.log`.

| # | Caso | Como testar | Esperado |
|---|---|---|---|
| K1 | Abrir no modo console | Ligar a opção e abrir o Tako (atalho) | "Indo para a tela de jogo…" na tela; a TV liga e troca para a entrada do PC (com a TV configurada), vira a principal, as outras telas desligam/escurecem, o som vai para a saída do jogo e o Tako abre na TV |
| K2 | Iniciar com o Windows | Opção ligada + "Abrir ao iniciar o Windows"; reiniciar o PC | Como K1, sem tocar em nada |
| K3 | Primeira vez | K1 com uma tela ou modo nunca confirmado | "Você consegue ver esta tela?" com 15 s; "Sim, manter" fica; sem resposta, B/Esc ou "Voltar ao normal", as telas voltam e o Tako fica na mesa; na próxima abertura pergunta de novo só se a tela/modo mudou |
| K4 | Jogos no modo console | Abrir e fechar dois jogos seguidos | Nenhuma piscada de tela entre os jogos: o jogo não foca nem restaura a tela de jogo, não troca o áudio e não põe a TV em espera; "Mandar a Steam para a bandeja" continua valendo no fim de cada jogo |
| K17 | Configurações deste jogo no modo console (seção 18) | Um jogo com "Usar a Tela de jogo" desligado; outro com resolução/HDR próprios; outro com saída de áudio própria | Os dois primeiros abrem na tela de jogo do modo console, sem piscar, com o aviso ("use Voltar ao PC" / "a resolução e o HDR deste jogo não valem"); o terceiro troca o som no início e volta para a saída do modo console ao fechar; FPS e argumentos do jogo valem |
| K5 | Voltar ao PC pelo menu | Barra lateral › **Voltar ao PC** › confirmar | "Voltando para a mesa…" com a barra andando (a tela não congela); telas, HDR, áudio e TV voltam a como estavam antes do modo console; o item some do menu |
| K6 | Voltar ao PC pela Central | Durante um jogo: atalho da Central (segurar View + Y) › Tela › **Voltar ao PC** | Confirmação dizendo que o jogo continua aberto; as telas voltam com o jogo aberto; ao fechar o jogo depois, nada volta para a TV |
| K7 | Sair do Tako | Barra lateral › Sair para o desktop | O aviso diz que as telas voltam para a mesa; "Voltando para a mesa…" e depois o Tako fecha com tudo como antes |
| K8 | Alt+F4 | Alt+F4 no modo console | As telas voltam para a mesa antes de o Tako fechar |
| K9 | Atalho de emergência | No modo console (e numa sessão da seção 9b), com o jogo na frente, apertar **Ctrl+Alt+Shift+D** | As telas voltam para a mesa sem depender da janela do Tako; a TV não vai para a espera; aviso com o atalho; o log registra "Atalho de emergência" |
| K10 | Queda no modo console | Finalizar só a UI (seção 10, S3) no modo console | A UI reabre, volta para a mesa sozinha ("O Tako fechou no modo console…") e, com a opção ligada, vai de novo para a tela de jogo |
| K11 | Queda sem supervisor | `Tako.exe --no-supervisor`, finalizar no modo console e abrir de novo | Na abertura, as telas voltam para a mesa sem pergunta; nenhuma oferta "Restaurar monitores e áudio?" sobre o jogo que estava aberto |
| K12 | Supervisor desiste | Finalizar a UI 5 vezes em 5 min no modo console (S3) | Na desistência o supervisor volta as telas para a mesa antes de sair (log do supervisor "Recuperação do modo console"); ninguém fica no desktop com as telas desligadas |
| K13 | Modo seguro | `Tako.exe --safe-mode` com a opção ligada | Abre na mesa com o aviso "Modo seguro: o Tako abriu na mesa" |
| K14 | Sem a TV | Opção ligada com a TV desconectada/desligada da tomada | Aviso "a tela de jogo escolhida não está conectada · o Tako abriu na mesa" ou, depois de ~8 s, "as telas voltaram como estavam" |
| K15 | Fim de jogo sem congelar | Seção 9b (sem o modo console): fechar o jogo e mexer no controle enquanto as telas voltam | "Voltando para a mesa…" com a barra andando; o supervisor não acusa "sem heartbeat"; "Jogar" só funciona depois que a volta termina |
| K16 | Atalho ocupado | Outro programa usando Ctrl+Alt+Shift+D antes de abrir o Tako | O Tako abre normalmente; o log registra que o atalho está indisponível |

## 20. DualSense Edge e controles juntos (MVP-031)

Relato da alpha.9: "o DualSense Edge não vai para cima". Causa provável: no Bluetooth, sem a Steam aberta, o controle
manda o relatório simples e o Tako lia os botões no byte errado (↑ ficava "sempre pressionado"). Rode cada caso com a
Steam **fechada** e depois **aberta** (com "Suporte a controles PlayStation" ligado). Se algo falhar, abra
Configurações › Sistema › Teste de controle, aperte o botão que falhou, use "Copiar" e cole na issue junto com o diagnóstico.

| # | Caso | Como testar | Esperado |
|---|---|---|---|
| DE1 | Edge pelo cabo (USB) | Conectar com o Tako aberto | Aviso "Controle conectado: Controle PS5 (DualSense Edge)"; teste de controle mostra "HID · DualSense Edge · USB · 054C:0DF2 · relatório 0x01 USB completo, 64 bytes" |
| DE2 | Edge pelo Bluetooth, Steam fechada | Parear e conectar | Mesmo aviso; teste mostra "Bluetooth" e "relatório 0x01 simples, 78 bytes" (ou "0x31 Bluetooth completo") |
| DE3 | ↑ em toque curto | Em DE1 e DE2, na tela inicial e nas Configurações | Cada toque anda **uma** casa para cima |
| DE4 | ↑ segurado | Segurar ↑ numa lista longa (Biblioteca, Configurações) | Anda uma casa, espera um pouco e repete; para ao soltar |
| DE5 | Direcional e analógico | ↓ ← → no direcional e no analógico esquerdo; analógico solto | Tudo anda uma casa por toque; analógico parado não anda sozinho |
| DE6 | Botões | ✕ ○ □ △, L1/R1, Options, Create, PS | ✕ confirma, ○ volta, L1/R1 trocam de seção, Options abre o menu; no teste de controle cada um aparece na linha do HID |
| DE7 | Fn e botões traseiros | No teste de controle, apertar Fn esquerdo/direito e os traseiros | Aparecem em "extras" (Fn esq., Fn dir., traseiro esq., traseiro dir.) sem mudar o direcional; não fazem nada na navegação |
| DE8 | Steam aberta | Repetir DE3 a DE6 com a Steam aberta (fora de jogo) | ↑ continua andando **uma** casa (não duas); segurar não acelera; teste mostra "Steam em execução: sim" |
| DE9 | XInput virtual junto (DS4Windows, DSX ou Steam) | Com um programa que cria um controle Xbox virtual a partir do Edge | Teste mostra o HID e o XInput; cada toque aciona uma vez; soltar um não "solta" o outro |
| DE10 | Xbox e Edge ao mesmo tempo | Conectar os dois | Os dois navegam; o aviso mostra o Edge; nenhum comando em dobro |
| DE11 | DualSense comum pelo Bluetooth | Repetir DE2 e DE3 com o DualSense (0CE6) | Igual ao Edge |
| DE12 | Diagnóstico | Configurações › Sistema › Diagnóstico do sistema com o Edge ligado | Seção "[Teste de controle]" com modelo, USB/Bluetooth, product id e relatórios crus; sem caminho do dispositivo nem endereço Bluetooth |

## 21. Dados dos jogos pela IGDB ([ADR-0021](../adr/0021-metadados-igdb.md))

Precisa de uma conta da Twitch com verificação em duas etapas e de jogos de várias lojas (Steam, Epic, GOG, Xbox e um
adicionado à mão). Anexe `logs\tako-AAAAMMDD.log` (linhas "IGDB" e "Metadados"). Nada disso roda no CI: lá não há
chamadas reais à Twitch nem à IGDB.

| # | Caso | Como testar | Esperado |
|---|---|---|---|
| IG1 | Sem chave | Instalação nova, sem configurar a IGDB | Configurações › Biblioteca e lojas › Dados dos jogos (IGDB): "Desligada"; os dados vêm da Steam como antes; nenhuma linha "IGDB" no log |
| IG2 | Passo a passo | Seguir os passos da tela: dev.twitch.tv, registrar app (`http://localhost`, Application Integration), Client ID, Novo segredo | Os textos batem com o site da Twitch; anotar o que mudou de nome |
| IG3 | Colar com o controle | Copiar o Client ID no navegador, voltar ao Tako e usar "Colar" só com o controle | O campo é preenchido sem espaços; status "Client ID colado" |
| IG4 | Testar com erros | Testar com o Client ID errado; depois com o segredo errado; depois sem internet | Mensagens diferentes e claras para cada caso; nada é salvo |
| IG5 | Testar e salvar | Chave certa › Testar › Salvar | "Tudo certo"; ao salvar, "IGDB ligada · buscando dados…"; o resumo da linha vira "Ligada · sua chave da Twitch" |
| IG6 | Segredo protegido | Abrir `%LOCALAPPDATA%\Tako\igdb.dat` no Bloco de Notas; reabrir a tela | O arquivo começa com `dpapi:` e não mostra a chave; o campo do segredo vem vazio com "já salvo" |
| IG7 | Jogos fora da Steam | Jogos da Epic, GOG e Xbox que não existem na Steam | Ganham descrição (inglês), gêneros/temas em português, capa e fundo da IGDB, nota "Crítica" e "Série" nos detalhes |
| IG8 | Casamento por id | Jogo do GOG e da Epic que também existem na Steam | Opções mostra "IGDB + Steam"; descrição em português da Steam; capa grande da Steam; sem busca pelo nome no log |
| IG9 | Jogo da Steam | Jogo da Steam com a IGDB ligada | Dados da Steam mantidos; temas, modos e série completados pela IGDB |
| IG10 | Corrigir correspondência | Jogo adicionado com nome ambíguo › Opções › Corrigir correspondência | Resultados da IGDB e da Steam, cada um com a fonte, ano e plataforma; escolher um da IGDB fixa e traz a capa |
| IG11 | Biblioteca grande | 200+ jogos, ligar a IGDB pela primeira vez | Poucas consultas em lote no início; nenhum erro 429 persistente; o Tako não trava; os jogos ganham dados aos poucos |
| IG12 | Aviso nos detalhes | IGDB desligada, jogo adicionado sem descrição | Linha discreta sugerindo ligar a IGDB; some ao ligar |
| IG13 | Qualidade na TV | Capas da IGDB (`t_cover_big`) na grade em 4K | Anotar se ficam borradas (se sim, trocar para `t_cover_big_2x`) |
| IG14 | Região Brasil e lojas | Jogo com nome/capa brasileira na IGDB; jogo da Epic e do Xbox | Nome e capa da localização Brasil; anotar se Epic (CatalogItemId) e Xbox (StoreId) casam por id ou só pelo nome |

## 22. Dados de ROMs pelo ScreenScraper ([ADR-0022](../adr/0022-metadados-screenscraper.md))

Precisa de uma conta em screenscraper.fr (e-mail confirmado), de uma **versão de release com as credenciais do
software** (segredos `SCREENSCRAPER_DEVID`/`SCREENSCRAPER_DEVPASSWORD` criados) e de algumas ROMs suas: um cartucho
solto (`.sfc`/`.gba`), um `.zip` com um cartucho, um conjunto do MAME (`.zip`), uma ISO grande (PS2) e um `.chd`.
Anexe `logs\tako-AAAAMMDD.log` (linhas "ScreenScraper" e "Metadados"). Nada disso roda no CI: lá não há chamadas reais
ao ScreenScraper.

| # | Caso | Como testar | Esperado |
|---|---|---|---|
| SS1 | Versão sem acesso | Build local (ou release sem os segredos) › Configurações › Biblioteca e lojas › Dados de ROMs | Linha "Indisponível nesta versão"; a tela avisa "Esta versão do Tako ainda não tem acesso ao ScreenScraper"; dá para salvar a conta; nenhuma linha "ScreenScraper" no log; o log do `Publish-Tako.ps1` avisa que saiu sem acesso |
| SS2 | Passo a passo | Seguir os passos: criar conta, confirmar e-mail, usuário e senha | Os textos batem com o site; anotar o que mudou de nome |
| SS3 | Testar com erros | Senha errada; depois sem internet | Mensagens diferentes e claras; nada é salvo |
| SS4 | Testar e salvar | Conta certa › Testar › Salvar | "Tudo certo: conta …, nível …, N de M consultas restam hoje"; ao salvar, "ScreenScraper ligado"; linha "Ligado · sua conta" |
| SS5 | Conta protegida | Abrir `%LOCALAPPDATA%\Tako\screenscraper.dat` no Bloco de Notas; reabrir a tela | Começa com `dpapi:` e não mostra usuário nem senha; o campo da senha vem vazio com "já salva" |
| SS6 | Adicionar ROM | Adicionar jogo › escolher um `.sfc` | Pergunta "De qual sistema…" com SNES primeiro; título sem "(USA)"; detalhes com "Sistema: SNES" e botão "Abrir no emulador (em breve)", que só explica |
| SS7 | Casamento pelo hash | ROM de cartucho reconhecida | Nome brasileiro quando houver, sinopse em português (senão inglês), caixa como capa, fanart/captura no fundo, "Jogadores", nota "Crítica"; Opções mostra "ScreenScraper + IGDB" |
| SS8 | .zip e arcade | `.zip` com um cartucho (sistema SNES) e um `.zip` do MAME (sistema Arcade) | Os dois casam pelo hash; anotar se algum só casou pelo nome |
| SS9 | ISO grande e .chd | ISO de PS2 (> 64 MB) e um `.chd` de PS1 | A tela não trava durante o hash; anotar se a ISO casa só com CRC e se o `.chd` casa pelo nome; reabrir o Tako não lê a ISO de novo (`cache\rom-hashes.json`) |
| SS10 | Mídia sem credenciais | Procurar a senha em `cache\metadata\*.json` e `cache\screenscraper\*.json` | A senha e o `devpassword` não aparecem em nenhum arquivo; as capas aparecem (baixadas de `ssmedia://`) |
| SS11 | Corrigir correspondência | ROM com nome ambíguo › Opções › Corrigir correspondência | Resultados do ScreenScraper (✓ no casado pelo hash) e da IGDB, com ano e sistema; escolher um fixa |
| SS12 | Cota | Conta perto do limite diário (ou muitas ROMs desconhecidas) | Ao acabar, a linha (ao abrir as Configurações) diz "Pausado · cota do dia"; as ROMs não ficam marcadas como "sem dados" e voltam a ser consultadas depois da meia-noite de Paris |
| SS13 | Jogos de PC | Biblioteca com jogos da Steam/Epic e o ScreenScraper ligado | Nenhuma consulta ao ScreenScraper para jogos de PC no log |
| SS14 | Capas na TV | Caixas deitadas (SNES, Mega Drive) na grade em 4K | Anotar se o recorte 2:3 fica aceitável |

## 23. Central de Controle ao vivo e atalhos (MVP-002 / UX-005)

Com um jogo real aberto pelo Tako. Teste num jogo em **janela sem bordas** (ex.: a maioria dos jogos da Steam no modo "Tela cheia sem bordas"/"Borderless") e num em **tela cheia exclusiva** ("Tela cheia" em jogos DirectX 11 antigos). Repita em 1080p, 1440p e 4K e com a escala do Windows em 100%, 150% e 200% quando houver esses monitores.

| # | Caso | Como testar | Esperado |
|---|---|---|---|
| AO1 | Central ao vivo (sem bordas) | Segurar View + Y no jogo | A Central aparece por cima do jogo, que continua **visível e se movendo** atrás, só um pouco escurecido; sem a arte do jogo no fundo. O teclado e o controle navegam na Central |
| AO2 | Fechar devolve o jogo | B, segurar View + Y de novo, ou "Voltar ao jogo" | A Central some, o jogo volta a ter o foco (o teclado volta a funcionar no jogo) e a biblioteca do Tako **não** pisca na tela |
| AO3 | Jogo em outro monitor (TV) | Tako num monitor e o jogo na TV (tela de jogo ou arrastando a janela); abrir a Central | A Central cobre o monitor do jogo; ao fechar, o Tako volta ao monitor dele |
| AO4 | Tela cheia exclusiva | Abrir a Central num jogo em tela cheia exclusiva | O Tako vem para a frente com a Central sobre a arte do jogo (comportamento anterior); ao fechar, o jogo volta. Anotar se o jogo minimizou e se voltou à resolução certa |
| AO5 | Jogo lendo o controle em segundo plano | Abrir a Central num jogo e navegar com o direcional e A | Anotar o jogo se ele reagir aos botões usados na Central (não há como o Tako bloquear; documentado no DESIGN.md). Anotar também se o jogo pausa ou silencia ao perder o foco |
| AO6 | Início / alternar janela / ControlFS | Na Central ao vivo, "Início"; depois, abrir de novo e escolher uma janela em "Janelas abertas" | Início: o Tako aparece normal (opaco) com o jogo atrás. Janela: ela vem para a frente e o Tako sai de cima do jogo |
| AO7 | Contador de FPS ligado | Contador ligado; abrir a Central ao vivo | A Central fica por cima do jogo e do contador; ao fechar, o contador continua no canto |
| JA1 | Janelas com títulos longos | Abrir 8+ janelas (navegador com abas de título longo, Explorer em pasta funda, WhatsApp, Discord) e abrir a Central | Cada cartão mostra ícone, título em até 2 linhas com "…" e o nome do programa embaixo; nada cortado em cima nem sobreposto ao ícone. O título da fileira mostra a contagem |
| JA2 | Rolagem da fileira | → até a última janela e ← de volta | O cartão focado fica sempre inteiro na tela, com folga nas bordas; as bordas da fileira esmaecem quando há mais cartões fora da tela |
| AT1 | Atalho no DualSense / DualShock 4 | Segurar Create + △ (Share + △ no DS4) no jogo | Abre a Central; as dicas mostram "Create △" |
| AT2 | Trocar o atalho do controle | Configurações › Controles › "Abrir a Central de Controle no controle" › segurar View + X por 1 s | Salvo; no jogo, View + X abre e View + Y não. Tentar LB + RB: recusado com "já servem para navegar no Tako". Tocar B cancela |
| AT3 | Trocar o atalho do teclado | Configurações › Controles › "… no teclado" › Ctrl+Alt+F10 | Salvo; Ctrl+Alt+F10 abre a Central **com o jogo na frente**. Shift+Tab é recusado (Steam); Ctrl+Shift+O é aceito com aviso (AMD) |
| AT4 | Atalho já usado por outro programa | Escolher um atalho que outro programa registrou (ex.: o do Discord ou do OBS) e abrir um jogo | Aviso "já é usado por outro programa · troque o atalho nas Configurações"; o atalho continua valendo com o Tako na frente |
| AT5 | Restaurar padrão | "Restaurar atalhos padrão" | Volta a View + Y e Ctrl+Shift+Tab |
| AT6 | Steam e Barra de Jogo | Com a Steam aberta, segurar View + Y no jogo | Só a Central abre (a sobreposição da Steam e a Barra de Jogo não) |

## 24. Configurações em categorias e menu Modo Console (UX-006/009, [ADR-0020](../adr/0020-organizacao-das-configuracoes.md))

Testar com controle **e** com teclado, numa tela 4:3 ou 16:10 (ex.: 1600×1200), numa 16:9 e, se houver, numa 21:9, com
escala do Windows em 100% e em 150–200%. Um jogo com arte clara em foco ajuda a ver a legibilidade.

| # | Caso | Como testar | Esperado |
|---|---|---|---|
| CF1 | Categorias | ☰ (ou M) › Configurações; ↓ pela coluna da esquerda | Cada categoria mostra o conteúdo na hora; nenhum texto com "MVP", "H2", "Marco" ou "carregador XAML" |
| CF2 | Entrar e voltar | → numa categoria, ↓ até a última linha, B, B | → entra na primeira linha; a linha focada fica inteira e acima da barra de dicas o tempo todo (também com o direcional segurado); 1º B volta à categoria, 2º B sai das Configurações |
| CF3 | LB/RB | LB/RB (Q/E) com o foco na coluna e no conteúdo | Troca de categoria e dá a volta; no conteúdo, o foco vai para a primeira linha da nova categoria |
| CF4 | Barra de dicas | Em cada tela/escala da lista acima, com controle e com teclado | Nenhuma dica cortada; sem espaço, os textos encurtam ("Ajustes", "OK") e somem primeiro "Seção"/"Buscar" |
| CF5 | Legibilidade | Configurações e Modo Console com um jogo de arte clara em foco | Valores (pílulas) e descrições legíveis a 3 metros |
| CF6 | Preferências mantidas | Atualizar de uma alpha anterior com opções mudadas (lojas, cor dinâmica, tela de jogo, TV, FPS) | Tudo continua como estava depois de atualizar |
| CF7 | Persistência | Mudar uma opção de cada categoria e do Modo Console; fechar e abrir o Tako | Todas voltam como foram deixadas |
| CF8 | Opções de desenvolvedor | Sistema › Opções de desenvolvedor | Desligadas por padrão; ligadas mostram recarregar tema, prévia do Playnite e teste de recuperação |
| MC1 | Modo Console | Barra lateral › Modo Console | Seis cartões com selo (Ligado, Desligado, Falta configurar, Com problema) coerente com o PC; A abre o detalhe, B volta ao mesmo cartão |
| MC2 | Assistente na TV | "Configurar para jogar na TV" com a TV ligada no HDMI | A TV é sugerida; "Testar agora" mostra o Tako na TV e pergunta "Você consegue ver esta tela?"; as telas voltam sozinhas; TV e áudio (a saída HDMI da TV sugerida) configurados; o primeiro jogo abre na TV sem perguntar de novo |
| MC3 | Assistente sem confirmar | Repetir MC2 sem responder (ou com B) | Em 15 s as telas voltam; o passo diz que não foi confirmado e oferece tentar de novo ou pular |
| MC4 | Tela desconectada | Desligar a TV escolhida e abrir o Modo Console | Cartão Tela de jogo "Com problema" dizendo que a tela não está conectada |
| MC5 | Contador de FPS | Modo Console › Contador de FPS com o controle e depois com teclado e mouse | Com o controle "Mostrar FPS" e "Remover a permissão" ficam esmaecidas e explicam; com teclado e mouse pedem o UAC (seção 14) |

## 25. Linha focada legível na TV (MVP-029 / UX-009)

Na TV, a 3 metros, com o controle. Em cada linha focada (branca) os controles continuam visíveis.

| # | Caso | Como testar | Esperado |
|---|---|---|---|
| FR1 | Filtros | Biblioteca › Filtros; focar uma chave ligada, uma desligada, uma opção de Ordenar e um gênero com contagem | Chave ligada verde-escura com bolinha branca; desligada com contorno e bolinha escuros; ○/◉ escuros; contagem cinza-escura legível |
| FR2 | Fim do painel | Descer até a última linha dos filtros e de Configurações deste jogo | Nenhuma linha aparece por baixo da barra de dicas; a linha focada fica inteira acima dela |
| FR3 | Central de Controle | Painéis Áudio (Mudo, saída), Tela (HDR, resolução) e Controle (bateria) | Mesmo comportamento de FR1; o valor da bateria legível na linha focada |
| FR4 | Barra lateral e categorias | Item selecionado da barra lateral com contagem; categorias das Configurações | Contagem e ícone legíveis sobre o branco |

## 26. Ubisoft Connect, EA app, Battle.net, Amazon Games e Playnite ([ADR-0019](../adr/0019-fontes-de-biblioteca.md))

Precisa de contas reais com **um jogo instalado em cada cliente**. Os jogos são lidos com os clientes **fechados**;
abrir passa sempre pelo cliente oficial (login e DRM ficam com ele). Anote em "Observações" o jogo usado em cada linha.

| # | Caso | Como testar | Esperado |
|---|---|---|---|
| UB1 | Jogos da Ubisoft | Ubisoft Connect com 1+ jogo instalado (de preferência um em outro disco), cliente fechado; abrir o Tako | O jogo aparece em "Ubisoft Connect" com o nome certo (não o da pasta, quando há chave de desinstalação) |
| UB2 | Abrir pela Ubisoft | Jogar o jogo | "Abrindo … pela Ubisoft Connect"; o cliente abre (login), o jogo inicia; o Tako reconhece o jogo rodando e o fim ao fechar |
| EA1 | Jogos do EA app | EA app com 1+ jogo instalado, EA app fechado | O jogo aparece em "EA app" com o nome em português ou inglês |
| EA2 | Abrir pelo EA app | Jogar o jogo | "Abrindo … pelo EA app"; o EA app abre e inicia **este** jogo (anotar se `origin2://` com o id de conteúdo funcionou ou se só abriu o EA app) |
| EA3 | Só o Origin | PC com o Origin antigo e sem o EA app (se houver como reproduzir) | Configurações › EA app: "Só o Origin está instalado e ele não abre mais os jogos: instale o EA app"; Jogar mostra o mesmo aviso |
| BN1 | Jogos do Battle.net | Battle.net com 1+ jogo (ex.: Overwatch 2, Hearthstone, Diablo IV), Battle.net fechado | O jogo aparece em "Battle.net" com o nome da tabela |
| BN2 | Abrir pelo Battle.net | Jogar o jogo | "Abrindo … pelo Battle.net"; o Battle.net abre e inicia o jogo (`--exec="launch <código>"`); fim da sessão reconhecido |
| BN3 | Jogo fora da tabela | Um jogo do Battle.net que o Tako não conhece (ex.: WoW Classic, se instalado) | Aparece com o nome do Windows ou da pasta; Jogar abre o Battle.net para escolher |
| AM1 | Jogos do Amazon Games | Amazon Games com 1+ jogo (ex.: do Prime Gaming) instalado, cliente fechado | O jogo aparece em "Amazon Games" |
| AM2 | Abrir pela Amazon | Jogar o jogo | "Abrindo … pela Amazon Games"; o cliente abre e o jogo inicia; fim da sessão reconhecido |
| CL1 | Cliente ausente | Desinstalar um dos clientes deixando o jogo (ou testar num PC sem ele) | Configurações › Biblioteca e lojas: "<cliente> não encontrado: instale-o para abrir estes jogos"; Jogar mostra o aviso com o nome do jogo; nada trava |
| CL2 | Desligar lojas | Configurações › Biblioteca e lojas: A em cada uma das quatro lojas e de novo | Os jogos somem e voltam; favoritos e tempo continuam; tudo só com o controle |
| PN1 | Importar do Playnite | Playnite com biblioteca real (lojas + jogos adicionados à mão + categorias + 1 ROM), Playnite **fechado**; Configurações › Importar do Playnite | Diálogo de consentimento com a pasta; "Ler biblioteca" mostra a prévia (novos, já no Tako, coleções, ignorados) |
| PN2 | Confirmar | "Importar" na prévia | Jogos adicionados à mão entram em "Adicionados" e abrem; jogos de loja **não** duplicam; tempo, favoritos e coleções aparecem; a ROM entra com o sistema |
| PN3 | Nada muda no Playnite | Comparar data/tamanho dos arquivos de `%APPDATA%\Playnite\library` antes/depois e abrir o Playnite | Arquivos iguais; o Playnite abre normalmente |
| PN4 | Importar de novo | Repetir PN1–PN2 | Prévia diz que os jogos já estão no Tako; nada duplica |
| PN5 | Playnite aberto | Repetir com o Playnite aberto | Lê normalmente ou pede "feche o Playnite e tente de novo"; nunca trava |

## Resultados

Os campos Máquina, Windows, Monitor(es), Controles e Versão saem do diagnóstico do sistema; cole o relatório completo logo abaixo da tabela, dentro de um bloco de código.

| Data | Máquina (CPU/GPU) | Windows (build) | Monitor(es) | Controles | Versão do Tako | Falhas (IDs) | Diagnóstico anexado | Observações |
|---|---|---|---|---|---|---|---|---|
| | | | | | | | | |

Para medir desempenho na mesma máquina, rode também o [benchmark](benchmark.md) e preencha a linha dela em "Máquinas de referência".
