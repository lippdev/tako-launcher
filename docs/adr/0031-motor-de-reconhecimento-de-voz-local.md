# ADR-0031: Motor de reconhecimento de fala local (VOICE-001)

- Status: Proposta
- Data: 2026-10-18

## Contexto

O item **VOICE-001** do PRD (P3) propõe permitir ao usuário pesquisar e navegar na biblioteca do Tako usando comandos de voz ("procurar jogos falando, com reconhecimento de fala local e nos dois idiomas"). Exemplos de casos de uso:
- Abertura direta: "Abrir Hades" / "open Hades".
- Filtros por gênero: "Jogos de corrida" / "racing games".
- Filtros de estado da biblioteca: "Instalados" / "installed", "favoritos" / "favorites".

Na rodada de 17/10/2026, a issue #102 levantou os requisitos iniciais e o mantenedor determinou a criação de uma ADR para definir o motor de reconhecimento, dependências, distribuição de modelos e privacidade antes de qualquer código.

### Princípios mandatórios

1. **Privacidade absoluta (100% offline):** nenhum fragmento de áudio, forma de onda ou texto reconhecido pode ser transmitido para a rede (CONTRIBUTING §5). O reconhecimento deve ocorrer exclusivamente no dispositivo do usuário.
2. **Suporte bilíngue de primeira classe:** reconhecimento preciso em português do Brasil (pt-BR) e inglês (en-US).
3. **Multiplataforma:** funcionamento uniforme nos três sistemas operacionais suportados pelo Tako (Windows, Linux e macOS).
4. **Sem escuta passiva:** não deve existir ativação contínua ("wake word" ou hotword em segundo plano), evitando preocupações de privacidade e consumo constante de bateria e CPU. A captura deve ocorrer apenas mediante ação explícita (Push-to-Talk / atalho de controle ou teclado).
5. **Arquitetura desacoplada:** o motor de fala deve ficar isolado sob um trait em `tako-core::devices` (`SpeechRecognizer`), com implementação simulada (mock) para testes unitários sem microfone ou modelos reais.

---

## Alternativas consideradas

### 1. Vosk com gramática restrita (`libvosk` / crate `vosk`) (Recomendada)

O Vosk é um motor de reconhecimento de fala de código aberto baseado no toolkit Kaldi, distribuído sob licença Apache-2.0.

- **Modelos:** modelos compactos específicos para inferência local rápida em CPUs:
  - pt-BR: `vosk-model-small-pt-0.3` (~45 MB descompactado).
  - en-US: `vosk-model-small-en-us-0.15` (~40 MB descompactado).
- **Recurso de Gramática Dinâmica:** o Vosk permite definir uma gramática restrita em formato JSON contendo a lista finita de palavras e frases válidas. Ao alimentar a gramática com os verbos de comando ("abrir", "jogar", "filtrar", "categoria") e a lista exata dos títulos de jogos presentes na biblioteca local do usuário, a taxa de precisão atinge níveis excepcionais, eliminando alucinações fonéticas comuns com nomes estrangeiros ou inventados (ex.: "Balatro", "Hades", "NieR", "Cyberpunk").
- **Desempenho e latência:** latência de resposta extremamente baixa (< 100 ms após o fim da fala) e uso de memória moderado (~60 a 90 MB de RAM durante a sessão de reconhecimento).
- **Licença:** Apache-2.0 (totalmente compatível com a licença AGPL-3.0 do Tako).
- **Dependências:** crate `vosk` em Rust + biblioteca dinâmica C `libvosk` (fornecida para Windows x64, Linux x64/aarch64 e macOS x64/arm64).

### 2. `whisper.cpp` (`whisper-rs`)

Port em C/C++ da arquitetura OpenAI Whisper por Georgi Gerganov.

- **Modelos:** modelo `tiny` (~75 MB) ou `base` (~140 MB).
- **Prós:** motor de estado da arte para fala natural genérica, excelente com sotaques variados; licença MIT.
- **Contras:** não oferece suporte nativo a restrição rígida por gramática com vocabulário fechado, o que frequentemente leva à grafia incorreta ou alucinações ao transcrever títulos de jogos pouco usuais; alto consumo de CPU e memória RAM (200-400 MB) em inferência por CPU; impacto severo de latência em dispositivos portáteis operando em bateria (Steam Deck, ROG Ally); requer ferramenta de compilação C++ pesada (CMake) no ambiente de CI para os 3 SOs.

### 3. APIs nativas do sistema operacional

Utilizar as APIs embutidas de cada SO (`Windows.Media.SpeechRecognition` / SAPI no Windows; `SFSpeechRecognizer` / Speech Framework no macOS; Speech Dispatcher no Linux).

- **Prós:** não adiciona peso de modelos nem binários externos ao instalador do Tako.
- **Contras:**
  - **Comportamento heterogêneo:** cada SO possui formatos, capacidades e taxas de erro completamente divergentes.
  - **Windows:** a API de reconhecimento offline depende de pacotes opcionais de idioma que o usuário precisa ter instalado manualmente nas configurações do Windows; caso ausente, falha silenciosamente.
  - **macOS:** a API moderna `SFSpeechRecognizer` exige permissões de sistema específicas e, por padrão, envia dados de voz aos servidores da Apple a menos que configurada explicitamente com `requiresOnDeviceRecognition = true` (disponível apenas em versões recentes e se o modelo estiver baixado localmente).
  - **Linux:** não há um subsistema nativo padrão de reconhecimento de fala offline de alta qualidade distribuído entre as diferentes distros.
  - **Testabilidade:** inviabiliza testes automatizados determinísticos entre plataformas.

### 4. PocketSphinx

Motor tradicional de reconhecimento contínuo de fala (CMU Sphinx).

- **Prós:** biblioteca muito leve e rápida em C puro.
- **Contras:** modelos acústicos legados e taxa de precisão muito inferior em português do Brasil comparado a modelos neurais ou Kaldi contemporâneos; desenvolvimento em ritmo de manutenção legada.

---

## Quadro comparativo

| Critério | Vosk (Kaldi) | whisper.cpp | APIs nativas do SO | PocketSphinx |
|---|---|---|---|---|
| **Privacidade 100% offline auditável** | Sim (código aberto e verificável) | Sim (código aberto e verificável) | Risco no macOS (Apple Speech) | Sim |
| **Precisão com nomes de jogos** | **Excelente** (com gramática dinâmica) | Média (sem gramática fechada) | Média/Baixa | Baixa |
| **Latência de resposta** | **< 100 ms** | 500 ms – 1500 ms | 200 ms – 500 ms | < 100 ms |
| **Uso de RAM / CPU** | Baixo (~75 MB RAM / CPU leve) | Alto (~250 MB RAM / CPU pesada) | Baixo (gerenciado pelo SO) | Mínimo |
| **Consistência nos 3 SOs** | Idêntica | Idêntica | Fragmentada | Idêntica |
| **Tamanho dos modelos (pt-BR + en-US)** | ~85 MB total | ~150-280 MB | 0 MB no Tako (usa do SO) | ~60 MB |
| **Complexidade de build** | Média (`libvosk` dinâmica) | Alta (CMake + C++14/17) | Média (bindings nativos) | Média |
| **Licença** | Apache-2.0 | MIT | Proprietária do SO | BSD-2 |

---

## Decisão proposta

1. **Motor de Reconhecimento:** adotar **Vosk com gramática dinâmica** como motor padrão de transcrição de fala para o VOICE-001.
2. **Captura de Áudio:** utilizar a crate `cpal` (Apache-2.0 / MIT) no `tako-platform` para captura de áudio PCM monocal via WASAPI (Windows), ALSA/PulseAudio/PipeWire (Linux) e CoreAudio (macOS).
3. **Contrato de Abstração:** criação do trait `tako_core::devices::SpeechRecognizer` com métodos assíncronos de inicialização, alimentação de fluxo PCM e notificação de eventos de transcrição. A implementação pura aceita simulação de fluxo em memória para testes do CI sem dependência de hardware.
4. **Construção Dinâmica da Gramática:** ao iniciar uma sessão de voz, o Tako gera dinamicamente uma gramática com:
   - Verbos e termos de comando em pt-BR e en-US ("abrir", "iniciar", "jogar", "pesquisar", "filtrar", "gênero", "instalados", "favoritos", etc.).
   - Os títulos exatos de todos os jogos cadastrados na biblioteca local do usuário.
5. **Garantia Técnica de Privacidade:**
   - O módulo de voz não possui dependências de rede e é isolado de sockets.
   - A captura de microfone é disparada **exclusivamente** via ação explícita (Push-to-Talk).
   - O buffer de áudio é mantido temporariamente em memória RAM e imediatamente sobrescrito e liberado após o encerramento do reconhecimento, sem gravação de arquivos de áudio em disco.
6. **Interface e Feedback:** enquanto o botão de voz estiver pressionado, a interface exibirá uma sobreposição sutil ("Ouvindo...") com indicador visual de nível de áudio e atualização instantânea do texto reconhecido na barra de pesquisa.

---

## Consequências

- Inclusão da dependência de áudio `cpal` e da biblioteca nativa `libvosk`.
- O tamanho dos artefatos de distribuição ou da primeira execução sofrerá acréscimo de aproximadamente 85 MB para acomodar os modelos de linguagem e acústica de pt-BR e en-US.
- A biblioteca só necessitará reconstruir a gramática quando jogos forem adicionados, renomeados ou removidos da coleção.
- Caso o usuário não possua microfone ou recuse a permissão de acesso ao dispositivo no sistema operacional, o Tako manterá a busca por texto tradicional sem qualquer degradação.

---

## Perguntas para o mantenedor

1. **Distribuição dos modelos de voz:** Os modelos compactos de pt-BR e en-US (~85 MB no total) devem vir embutidos diretamente no instalador oficial do Tako, ou devem ser baixados sob demanda na primeira vez que o usuário acionar o recurso de voz nas configurações (com confirmação explícita)?
2. **Modo de acionamento preferido:** O design da entrada por voz deve adotar Push-to-Talk estrito (segurar botão no controle/atalho de teclado enquanto fala) ou modo de alternância (clicar para iniciar escuta e clicar para encerrar ou parar por detecção de silêncio de 1.5 s)?
3. **Empacotamento da `libvosk`:** Preferir distribuir a biblioteca dinâmica pré-compilada (`libvosk.so` / `vosk.dll` / `libvosk.dylib`) empacotada na pasta do executável do Tako, ou configurá-la como dependência de sistema nos pacotes Linux (AUR, Flatpak, deb)?
4. **Tratamento de múltiplos idiomas simultâneos:** O reconhecimento deve carregar o modelo de acordo com o idioma da interface escolhido pelo usuário no Tako (ex.: interface em pt-BR carrega modelo pt-BR) ou manter ambos os modelos carregados simultaneamente para aceitar comandos bilíngues a qualquer momento?
