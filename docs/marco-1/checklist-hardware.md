# Checklist de hardware (MVP-034)

Casos que o CI não cobre: precisam de monitores, áudio, controles, Steam e Windows reais. Rode numa versão **publicada** (instalador `Tako-Setup-x64.exe` ou `Tako-Portable-x64.exe` do artefato da PR ou da release), não pelo `dotnet run`: atualização automática e "abrir ao iniciar o Windows" só funcionam nas versões publicadas.

Como registrar: copie a tabela de resultados no fim deste arquivo para a PR ou issue, uma linha por máquina, e anexe o log do dia `logs\tako-AAAAMMDD.log` (pasta de dados, ver abaixo) quando algo falhar.

**Em toda execução, anexe o diagnóstico do sistema** (MVP-034): Configurações › **Diagnóstico do sistema** › **Gerar**. O relatório traz versão e tipo de instalação do Tako, Windows/build, processador e memória, cada monitor (resolução, proporção, escala, Hz e placa de vídeo), controles detectados (XInput e PlayStation por HID), Steam e número de jogos, estado do Xbox mode com os sinais lidos e modo seguro. Não tem nome de usuário, nome do computador nem lista de jogos (a pasta do perfil aparece como `%USERPROFILE%`). **Copiar** põe o texto na área de transferência; o arquivo fica em `logs\diagnostico-AAAAMMDD-HHMMSS.txt`. Gere de novo depois de trocar monitor ou controle (casos M7/M8, C1–C8) para registrar o estado que o Tako viu.

- Pasta de dados: instalado → `%LOCALAPPDATA%\Tako`; portátil → `Tako_Data` ao lado do `.exe`.
- Marcador de sessão: `session.json` na pasta de dados (`Running: true` enquanto o Tako está aberto).

## Atalhos de emergência (sempre disponíveis)

| Situação | Caminho de volta ao desktop |
|---|---|
| Tela do Tako responde | Barra lateral › **Sair para o desktop**, ou **Alt+F4** (fecha a janela de tela cheia na hora, sem confirmação). |
| Erro inesperado | A tela **Algo deu errado** tem **Sair para o desktop** num passo só. Mais de 3 erros em 10 s fecham a tela cheia sozinhos. |
| Tela congelada | **Ctrl+Shift+Esc** (Gerenciador de Tarefas, aparece por cima: a janela do Tako não é "sempre visível") › finalizar `Tako.exe`. **Win** abre o menu Iniciar. |
| Travou ao abrir duas vezes seguidas | A terceira abertura entra em **modo seguro** sozinha. Para forçar: `Tako.exe --safe-mode`. |

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
| C10 | Botão Guia / PS | Durante um jogo simulado | Abre a Central de Controle |
| C11 | Sem controle nenhum | Usar só o teclado do início ao fim | Todas as telas alcançáveis (Shift+Tab abre a Central de Controle) |

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
| R2 | Duas saídas sujas seguidas | Repetir R1 duas vezes; abrir a terceira | Aviso **Modo seguro**; tema padrão embutido; Configurações › Modo seguro = "Ativo" |
| R3 | Voltar ao normal | No aviso ou em Configurações › Modo seguro | Sem reiniciar: tema do disco e atualizações automáticas voltam |
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

Linha em Configurações › **Xbox mode (tela cheia do Windows)**.

| # | Máquina | Esperado |
|---|---|---|
| F1 | Windows 10 ou Windows 11 23H2 | **Indisponível** (exige 24H2) |
| F2 | Windows 11 24H2+ sem o app Xbox | **Indisponível** |
| F3 | Desktop/notebook 24H2+ com app Xbox | **Indeterminado** (rollout gradual da Microsoft; nunca "Suportado" sem sinal positivo) |
| F4 | Portátil de jogos (ROG Ally, Legion Go…) 24H2+ com app Xbox | **Suportado** |
| F5 | Comparar com Configurações › Jogos › Xbox mode do Windows | Se o Windows mostra o Xbox mode e o Tako diz "Indisponível", é falso negativo: anexar o diagnóstico do sistema (linha "Sinais": build, app Xbox, `DeviceForm`) e a saída do `Launcher.FseProbe` |

Em todos os casos o Tako abre em tela cheia convencional; registrar-se como app inicial do Xbox mode depende de autorização da Microsoft (ver `docs/spikes/MVP-007-fse.md`).

## 9. Restaurar monitores e áudio (MVP-012)

Opção em Configurações › **Restaurar monitores e áudio ao sair do jogo** (padrão: ligada). Decisões e riscos na [ADR-0013](../adr/0013-estado-de-monitores-e-audio.md); o CI só testa com monitores e áudio simulados.

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

Configurações › **Tela de jogo** (padrão: desligada). Use um PC com monitor + TV (ou dois monitores) e o mesmo jogo de teste da seção 9. Anexe `logs\tako-AAAAMMDD.log` (linhas "Tela de jogo").

| # | Caso | Como testar | Esperado |
|---|---|---|---|
| G1 | Escolher a tela | Ligar a opção; ativar "Tela onde você joga" várias vezes | Sugere a tela que não é a principal; passa por todas as conectadas, inclusive a TV desligada da área de trabalho |
| G2 | TV desligada da área de trabalho | TV conectada mas fora da área de trabalho (Win+P "Somente tela do PC"); abrir um jogo | A TV liga, vira principal, o monitor desconecta, o Tako vai para a TV; ao fechar o jogo tudo volta como estava (TV fora da área de trabalho) |
| G3 | Confirmação | Primeira vez com G2 | "Você consegue ver esta tela?" na TV, com contagem de 15 s; A/Enter/clique em "Sim, manter" abre o jogo; na próxima vez não pergunta |
| G4 | Sem resposta | Como G3, mas sem responder (ou B/Esc, ou "Voltar ao normal") | As telas voltam sozinhas; aviso "Tela de jogo não confirmada"; o jogo não abre |
| G5 | Cobrir de preto | "Outras telas" › Cobrir de preto | O monitor fica preto (ligado), sem roubar o foco do jogo; some ao fechar o jogo |
| G6 | Resolução e taxa | Escolher um modo (ex.: 3840 × 2160 · 120 Hz) | Aplicado na TV durante o jogo; volta ao fechar; trocar o modo pede confirmação de novo |
| G7 | Modo recusado | Escolher um modo que a TV não aceita | O jogo abre no modo atual da TV; o log registra a recusa |
| G8 | HDR | "Ligar HDR" numa TV com HDR desligado | HDR liga durante o jogo e desliga ao sair; numa tela sem HDR, nada acontece e o log diz "HDR não disponível" |
| G9 | TV que não acorda | TV desligada da tomada/sem sinal | Depois de ~8 s o Tako desiste, as telas ficam como estavam e o jogo abre na tela atual |
| G10 | Tela desconectada | Escolher a TV e desconectar o cabo | Aviso "a tela de jogo escolhida não está conectada · o jogo abre na tela atual" |
| G11 | Jogo que não abre / sem acompanhamento | Atalho inválido; URI sem pasta | As telas voltam logo |
| G12 | Queda com a tela de jogo em foco | Finalizar `Tako.exe` durante o jogo e abrir de novo | Com "Restaurar monitores e áudio" ligado, aparece a oferta de restauração (seção 9, E14) e as telas voltam |
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
| G1 | Guia (Xbox) ou PS durante o jogo | O Tako vem para a frente com a Central aberta sobre a arte do jogo; B, Guia ou "Voltar ao jogo" devolvem o jogo. Anotar se a Barra de Jogo do Xbox também abre |
| G2 | Janelas abertas | A fileira lista as janelas do Alt+Tab com ícones; A alterna para uma (o jogo segue aberto); X fecha (um programa com algo não salvo pergunta antes) |
| G3 | Encerrar jogo (jogo real) | O jogo recebe o pedido de fechar e volta à frente; ao fechar, "Bem-vindo de volta" |
| G4 | Desempenho | Cartão e painel mostram CPU, GPU (com temperatura, se o driver informar) e RAM próximos do Gerenciador de Tarefas |
| G5 | Contador de FPS desligado | Painel de Desempenho mostra as opções de FPS esmaecidas com "Ative o contador de FPS nas Configurações do Tako, com teclado e mouse."; nada trava |
| G6 | Contador de FPS ligado | Ver §14 (contador próprio, sem RTSS) |
| G7 | Limite de FPS | Pelo driver de vídeo: ver §13 |
| G8 | Ponteiro | Esconder some com o ponteiro em todos os apps e ele volta ao fim da sessão; mover com o analógico funciona com Xbox e com DualSense; A, A duplo e segurar A clicam, clicam com o direito e arrastam |
| G9 | Queda com o ponteiro escondido ou o contador de FPS medindo | Finalizar `Tako.exe` e abrir de novo: ponteiro visível; `PresentMon.exe` fechado e `logman query -ets` sem a sessão `Tako_FPS` |
| G10 | ControlFS | Instalado: abre por cima do jogo; não instalado: "Baixar o ControlFS" abre a página |

## 12. Áudio, Steam e TV durante o jogo (MVP-002, [ADR-0016](../adr/0016-audio-steam-e-tv-na-sessao.md))

Configurações › **Saída de áudio durante o jogo**, **Mandar a Steam para a bandeja ao sair do jogo** e **Ligar a TV e trocar a entrada** (TV: Google TV / Android TV com "Depuração USB" ou "Depuração pela rede" ativada nas Opções do desenvolvedor).

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
| T1 | Pareamento | Configurações › TV: IP, **Testar** | A TV pergunta "Permitir depuração"; com "Sempre permitir", a TV liga e muda para a entrada do PC |
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
| F1 | Com controle nas Configurações | Usar só o controle e ir em Configurações › Mostrar FPS | Linha esmaecida com "Use teclado e mouse para ativar"; A só mostra o aviso, nenhum UAC abre |
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
| F17 | Remover permissão | Configurações › Remover permissão do contador de FPS (teclado e mouse) | UAC; a conta sai do grupo; o contador fica desligado |

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
| X1 | Jogo do Game Pass em `C:\XboxGames` | Instalar um jogo do PC Game Pass pelo app Xbox; Configurações › Procurar jogos instalados | Aparece em Biblioteca › Xbox com o nome certo; o aviso diz "N jogos do Xbox encontrados" |
| X2 | Outra unidade | Escolher `D:` como pasta de jogos no app Xbox e instalar um jogo lá | Aparece também (lido pelo `.GamingRoot` de `D:\`) |
| X3 | Jogo antigo em WindowsApps | Um jogo UWP com Xbox Live (ex.: Microsoft Solitaire Collection) | Aparece; apps comuns da Store (ex.: Calculadora, Fotos) não aparecem |
| X4 | Arte e dados | Abrir os detalhes de X1 depois de alguns segundos | Capa e fundo da Steam quando o nome casar; sem correspondência, o logo do pacote; Opções › Corrigir correspondência funciona |
| X5 | Abrir | Jogar em X1 | O jogo abre pelo Windows (sem erro de acesso); a tela mostra "aguardando o jogo iniciar" e depois "Jogando" |
| X6 | Sessão | Jogar alguns minutos, abrir a Central de Controle (Guia) | Central sobre o jogo; contador de FPS acompanha o processo do jogo |
| X7 | Fim | Fechar o jogo pelo menu do próprio jogo | Em até ~5 s o Tako volta; "Jogados recentemente" e tempo de jogo atualizados |
| X8 | Desligar a fonte | Configurações › Jogos do Xbox (PC Game Pass) › desligar | Os jogos do Xbox somem da biblioteca; ligar de novo traz de volta |
| X9 | Sem app Xbox | Numa máquina sem o app Xbox (ou após desinstalá-lo) | A linha "Jogos do Xbox" avisa "App Xbox não encontrado…"; nada trava |
| X10 | Sem Serviços de Jogos | Parar/remover os Serviços de Jogos (ou máquina recém-formatada só com o app Xbox) | Aviso "Serviços de Jogos não encontrados: abra o app Xbox para instalá-los" |
| X11 | Jogo desinstalado | Desinstalar X1 pelo app Xbox e procurar de novo | O jogo some (pasta que sobrar sem pacote registrado não aparece) |
| X12 | Outro usuário do Windows | Entrar com outra conta do Windows no mesmo PC | Só aparecem os jogos registrados para essa conta |

## Resultados

Os campos Máquina, Windows, Monitor(es), Controles e Versão saem do diagnóstico do sistema; cole o relatório completo logo abaixo da tabela, dentro de um bloco de código.

| Data | Máquina (CPU/GPU) | Windows (build) | Monitor(es) | Controles | Versão do Tako | Falhas (IDs) | Diagnóstico anexado | Observações |
|---|---|---|---|---|---|---|---|---|
| | | | | | | | | |

Para medir desempenho na mesma máquina, rode também o [benchmark](benchmark.md) e preencha a linha dela em "Máquinas de referência".
