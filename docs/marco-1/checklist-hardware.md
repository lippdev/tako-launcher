# Checklist de hardware (MVP-034)

Casos que o CI não cobre: precisam de monitores, áudio, controles, Steam e Windows reais. Rode numa versão **publicada** (instalador `Tako-Setup-x64.exe` ou `Tako-Portable-x64.exe` do artefato da PR ou da release), não pelo `dotnet run`: atualização automática e "abrir ao iniciar o Windows" só funcionam nas versões publicadas.

Como registrar: copie a tabela de resultados no fim deste arquivo para a PR ou issue, uma linha por máquina, e anexe o log do dia `logs\tako-AAAAMMDD.log` (pasta de dados, ver abaixo) quando algo falhar.

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
| A3 | Central de Controle › Áudio | Painel abre e ajusta o volume (Marco 0: simulado; restauração real é do Console Runtime) |

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
| F5 | Comparar com Configurações › Jogos › Xbox mode do Windows | Se o Windows mostra o Xbox mode e o Tako diz "Indisponível", é falso negativo: anotar build, `DeviceForm` e a saída do `Launcher.FseProbe` |

Em todos os casos o Tako abre em tela cheia convencional; registrar-se como app inicial do Xbox mode depende de autorização da Microsoft (ver `docs/spikes/MVP-007-fse.md`).

## Resultados

| Data | Máquina (CPU/GPU) | Windows (build) | Monitor(es) | Controles | Versão do Tako | Falhas (IDs) | Observações |
|---|---|---|---|---|---|---|---|
| | | | | | | | |
