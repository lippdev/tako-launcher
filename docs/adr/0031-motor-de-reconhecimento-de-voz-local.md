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

1. **Privacidade absoluta (100% offline):** nenhum fragmento de áudio, forma de onda ou texto reconhecido pode ser transmitido para a rede (CONTRIBUTING §5). O reconhecimento deve ocorrer exclusivamente no dispositivo local.
2. **Suporte bilíngue de primeira classe:** reconhecimento em português do Brasil (pt-BR) e inglês (en-US).
3. **Multiplataforma:** funcionamento uniforme nos três sistemas operacionais suportados pelo Tako (Windows, Linux e macOS).
4. **Sem escuta passiva:** não deve existir ativação contínua ("wake word" ou hotword em segundo plano), evitando preocupações de privacidade e consumo constante de bateria e CPU. A captura deve ocorrer apenas mediante ação explícita (Push-to-Talk / botão no controle ou atalho no teclado).
5. **Arquitetura desacoplada e isolamento de FFI:**
   - O contrato abstrato fica em `tako-core::devices` (`SpeechRecognizer`), com implementação simulada (mock) para testes unitários no CI sem hardware de áudio nem modelos.
   - Todo código inseguro (`unsafe`) e vinculações FFI com a biblioteca C nativa `libvosk` e chamadas de captura de áudio com `cpal` residem **estritamente em `tako-platform`**.
6. **Governança de dependências:** qualquer dependência externa (`cpal`, `vosk`, etc.) deve ser declarada centralizadamente na seção `[workspace.dependencies]` do `Cargo.toml` raiz do repositório.

---

## O desafio da gramática dinâmica do Vosk e o vocabulário fechado

O Vosk permite definir uma gramática restrita em formato JSON contendo a lista finita de palavras e frases válidas. Ao restringir o espaço de busca acústica aos verbos de comando ("abrir", "jogar", "filtrar") e títulos cadastrados, busca-se evitar erros de sintaxe.

Entretanto, **a gramática dinâmica do Vosk possui uma limitação severa de vocabulário (OOV — *Out-Of-Vocabulary*): ela descarta sumariamente qualquer palavra que não conste no léxico/dicionário pré-compilado do modelo Kaldi base.**
Títulos de jogos estrangeiros, inventados, abreviados ou estilizados (como *"Balatro"*, *"NieR"*, *"Cyberpunk"*, *"Bayonetta"*, *"Hades"*, *"Shin Megami Tensei"*) simplesmente não existem no vocabulário padrão do modelo pt-BR ou en-US. Quando o usuário pronuncia essas palavras sob uma gramática estrita, o motor as descarta ou produz silêncio, inviabilizando a busca.

### Plano B de contingência: Casamento aproximado e fonético

Para contornar a limitação de palavras fora do vocabulário, o Tako não dependerá exclusivamente da gramática estrita fechada:
1. **Modelo Híbrido:**
   - A gramática estrita é utilizada prioritariamente para comandos operacionais do sistema ("abrir", "filtrar", "favoritos", "instalados").
   - Para busca de títulos, caso a gramática estrita não identifique o título ou se o nome contiver termos fora do vocabulário, o motor recorrerá à **transcrição aberta (texto livre)** gerada pelo modelo acústico.
2. **Casamento Fonético e por Similaridade:**
   - O texto bruto transcrito é submetido a um pipeline de comparação em memória no `tako-core` contra os títulos da biblioteca local do usuário utilizando:
     - Algoritmos de distância de edição (Levenshtein e Jaro-Winkler normalizados);
     - Codificação fonética adaptada (como Metaphone / Double Metaphone ou Soundex pt-BR) para comparar como a palavra soa foneticamente em relação ao título real do jogo cadastrado na biblioteca.

### Condição de aceite: Experimento prévio com títulos reais

A escolha definitiva do motor fica expressamente condicionada à realização de um **experimento prático obrigatório com títulos reais da biblioteca** antes de qualquer implementação de código no Tako:
- Testar o modelo com títulos populares com grafias atípicas (*"Balatro"*, *"NieR:Automata"*, *"Hades"*, *"BioShock"*, *"Stardew Valley"*);
- Medir a taxa de descarte da gramática dinâmica e validar a assertividade do algoritmo de casamento fonético aproximado com falantes reais em pt-BR e en-US.

---

## Alternativas consideradas

### 1. Vosk com gramática dinâmica e fallback fonético (`libvosk` / crate `vosk`) (Recomendada sob condição)

O Vosk é um motor de reconhecimento de fala de código aberto baseado no toolkit Kaldi, distribuído sob licença Apache-2.0.

- **Modelos oficiais (Licença Apache-2.0):**
  - pt-BR: `vosk-model-small-pt-0.3` (**31 MB** descompactado).
  - en-US: `vosk-model-small-en-us-0.15` (~40 MB descompactado).
- **Taxa oficial de erro de palavras (Word Error Rate - WER):**
  - No modelo pt-BR pequeno (`vosk-model-small-pt-0.3`), os benchmarks oficiais reportam:
    - **68,9% de WER no corpus CORAA** (áudio espontâneo e conversacional);
    - **32,6% de WER no corpus Common Voice**.
  - Os números evidenciam que a precisão em linguagem natural aberta é modesta, reforçando a dependência de comandos bem definidos e do algoritmo de casamento fonético da biblioteca.
- **Latência e consumo de recursos:**
  - Latência de resposta: *a medir em hardware*.
  - Uso de memória RAM: *a medir em hardware* (estimativa típica de ~60 a 90 MB de RAM durante a inferência).
- **Risco da crate `vosk` (v0.3.1):**
  - O wrapper Rust `vosk` (versão 0.3.1 no crates.io) possui manutenção escassa e pouca atividade recente. Caso surjam incompatibilidades com novas versões do toolchain Rust ou sistemas operacionais, a equipe do Tako precisará manter bindings FFI diretos com a `libvosk` em C dentro de `tako-platform`.
- **Download opt-in com SHA-256:**
  - O download dos modelos acústicos nunca deve ser embutido forçadamente no instalador nem baixado sem permissão. O download é estritamente **opt-in**, com confirmação explícita do usuário nas Configurações, validando o hash **SHA-256** antes de extrair.
- **Empacotamento da `libvosk` no ecossistema Linux:**
  - **Flatpak:** inclusão de um módulo no manifesto do Flatpak (`com.lippdev.tako.json` / `.yaml`) que baixa a biblioteca pré-compilada upstream da `libvosk` e instala os binários em `/app/lib/`.
  - **AUR (Arch Linux):** declaração no PKGBUILD de dependência do pacote `vosk-api` ou embutimento da biblioteca compartilhada `libvosk.so` no diretório privado da aplicação (`/usr/lib/tako/`).

### 2. `whisper.cpp` (`whisper-rs`)

Port em C/C++ da arquitetura OpenAI Whisper por Georgi Gerganov.

- **Modelos:** modelo `tiny` (~75 MB) ou `base` (~140 MB).
- **Prós:** motor de arquitetura neural contemporânea para fala natural; licença MIT; boa resiliência a sotaques.
- **Contras:**
  - Não possui mecanismo nativo de restrição rígida de vocabulário ou gramática;
  - Latência de inferência e uso de memória: *a medir em hardware* (alta dependência de aceleração vetorial AVX/NEON na CPU, com sobrecarga estimada sensivelmente superior ao Kaldi em CPUs modestas);
  - Requer toolchain C++ pesado (CMake / compilador C++17) no ambiente de CI para os 3 SOs.

### 3. APIs nativas do sistema operacional

Utilizar as APIs embutidas de cada SO (`Windows.Media.SpeechRecognition` / SAPI no Windows; `SFSpeechRecognizer` no macOS; Speech Dispatcher no Linux).

- **Prós:** 0 MB de download de modelos no Tako.
- **Contras:** comportamento heterogêneo entre plataformas; no macOS há risco de envio de áudio aos servidores da Apple caso o modelo local não esteja presente; no Linux não há padrão consolidado offline entre distribuições; inviabiliza testes automatizados padronizados.

### 4. PocketSphinx

Motor tradicional de reconhecimento contínuo de fala (CMU Sphinx).

- **Prós:** extremamente leve em C puro.
- **Contras:** modelos acústicos legados com precisão substancialmente inferior aos modelos Kaldi contemporâneos em português do Brasil.

---

## Quadro comparativo

| Critério | Vosk (Kaldi) | whisper.cpp | APIs nativas do SO | PocketSphinx |
|---|---|---|---|---|
| **Privacidade 100% offline auditável** | Sim (código aberto e verificável) | Sim (código aberto e verificável) | Risco no macOS (Apple Speech) | Sim |
| **Taxa de erro oficial (pt-BR)** | CORAA: 68,9% WER / Common Voice: 32,6% WER | *a medir* | Não documentada de forma aberta | Elevada taxa de erro |
| **Palavras fora do vocabulário (OOV)** | Descarta na gramática estrita; exige plano B fonético | Transcreve aproximação livre | Transcreve aproximação livre | Descarta |
| **Latência de resposta** | *a medir* | *a medir* | *a medir* | *a medir* |
| **Uso de RAM / CPU** | *a medir* | *a medir* | Gerenciado pelo SO | Mínimo |
| **Consistência nos 3 SOs** | Idêntica | Idêntica | Fragmentada | Idêntica |
| **Tamanho dos modelos (pt-BR + en-US)** | ~71 MB total (pt-BR: 31 MB, en-US: ~40 MB) | ~150-280 MB | 0 MB no Tako (usa do SO) | ~60 MB |
| **Licença do motor e modelos** | Apache-2.0 (código e modelos) | MIT | Proprietária do SO | BSD-2 |
| **Manutenção da crate Rust** | Risco: crate `vosk` 0.3.1 estagnada | Ativa (`whisper-rs`) | N/A (bindings nativos) | Estagnada |

---

## Decisão proposta

1. **Condicionamento a Experimento Real:** a adoção do Vosk com gramática dinâmica fica condicionada ao resultado do experimento prático com títulos reais da biblioteca e validação do algoritmo de contingência fonética.
2. **Plano B de Contingência Fonética:** incorporar no `tako-core` rotinas de casamento aproximado (Levenshtein e Metaphone/Soundex) sobre a saída de texto livre para jogos com títulos fora do vocabulário do modelo.
3. **Distribuição dos Modelos via Download Opt-in:**
   - Os modelos acústicos oficiais Apache 2.0 (pt-BR de **31 MB** e en-US de ~40 MB) não virão embutidos no instalador.
   - O download ocorrerá sob demanda, estritamente *opt-in*, com confirmação prévia do usuário nas Configurações e conferência de integridade via **hash SHA-256**.
4. **Isolamento de Código FFI e Áudio:**
   - Captura de áudio PCM via crate `cpal` e vinculação com `libvosk` restritas exclusivamente a **`tako-platform`**.
   - Trait de abstração pura `SpeechRecognizer` em `tako-core::devices`.
5. **Governança de Dependências:**
   - Dependências (`cpal`, `vosk`) registradas centralizadamente em `[workspace.dependencies]`.
   - Ciente do risco de pouca manutenção da crate `vosk` 0.3.1, com plano de FFI manual se necessário.
6. **Empacotamento nos Canais Linux:**
   - Inclusão da `libvosk` no manifesto Flatpak (`/app/lib/`) e no PKGBUILD do AUR.
7. **Privacidade e Acionamento:**
   - Captura estritamente orientada a Push-to-Talk (sem escuta contínua de fundo).
   - Áudio descartado imediatamente da memória RAM após a transcrição; sem escrita em disco ou transmissão de rede.

---

## Consequências

- A biblioteca ganha busca por voz 100% offline e privada, sem expor nenhum dado do usuário à rede.
- Usuários que optarem pelo recurso farão o download verificado via SHA-256 dos modelos (~71 MB no total para ambos os idiomas).
- A adição do módulo exige a inclusão da `libvosk` empacotada no Flatpak e AUR.
- O experimento com títulos reais definirá a viabilidade final antes da escrita de código de integração.

---

## Perguntas para o mantenedor

1. **Aprovação do Vosk com Plano B e experimento:** O projeto aceita a adoção do Vosk mesmo com o descarte de nomes fora do vocabulário na gramática estrita, aprovando a estratégia de contingência com casamento fonético e exigindo o experimento prévio com títulos reais antes de codar?
2. **Distribuição e integridade dos modelos:** Concorda com a distribuição dos modelos pt-BR (31 MB) e en-US (~40 MB) de forma estritamente *opt-in* sob demanda com confirmação do usuário e validação de hash SHA-256?
3. **Modo de acionamento do microfone:** Preferir Push-to-Talk estrito (segurar botão no controle ou atalho no teclado enquanto fala) ou modo de alternância (clicar para iniciar escuta e parar por clique ou detecção de silêncio após 1.5 s)?
4. **Empacotamento da `libvosk` no Linux:** Concorda com o empacotamento da `libvosk` compartilhada no Flatpak (`/app/lib/`) e via AUR?
5. **Gerenciamento de idiomas em memória:** O reconhecimento deve carregar o modelo de acordo com o idioma da interface configurado no Tako ou manter ambos os modelos carregados em memória para comandos bilíngues instantâneos?
