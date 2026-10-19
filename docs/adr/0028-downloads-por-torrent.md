# ADR-0028: Downloads por torrent a partir de fontes do usuário (DL-001)

- Status: Aceita (mantenedor @nextestudios, 2026-10-18)
- Data: 2026-10-18

## Contexto

O DL-001 (PRD §9) pede baixar jogos por torrent a partir de fontes `.json` que o próprio usuário adiciona, em formato de fonte .json, vinculando o download ao jogo da biblioteca quando ele já existe ("Baixando Batman: Arkham Knight" com progresso, sem criar jogo novo). Ao terminar, "o jogo fica instalado e jogável pelo Tako". O DL-002 (Real-Debrid) vem depois.

Restrições que valem para qualquer solução:

- Decisão do mantenedor (2026-10-16): o Tako fornece só a ferramenta. Não traz, não recomenda e não lista fontes, nem na interface nem na documentação.
- CONTRIBUTING §6.7: biblioteca 100% offline; proibido executar arquivos baixados, desativar antivírus/anti-cheat ou injetar qualquer coisa em jogos. A biblioteca (ADR-0019) lê só o disco; downloads são rede nova e precisam ficar isolados dela.
- Licença AGPL-3.0-only (ADR-0014): dependências precisam ser compatíveis (MIT, Apache-2.0, BSD, MPL; GPL-3.0 e AGPL também servem, mas GPL-2.0-only não).
- Código de SO só em `tako-platform`/`tako-sources`; `tako-core` não conhece UI nem SO (ADR-0024). Dados externos não são confiáveis: limite de tamanho, sem pânico, testes com entrada quebrada.
- O design (`PainelDownloads`) já prevê a pílula na barra, o painel lateral (BAIXANDO, NA FILA, CONCLUÍDOS), "Baixando · 42%" no jogo e as instalações das lojas no mesmo painel.

O ponto difícil é o "instalado e jogável": a maioria dos torrents de jogos traz um instalador, e o Tako não pode executá-lo.

## Decisão

### 1. Fontes `.json`

- O usuário adiciona uma fonte por URL `https://` (ou arquivo local) em Configurações › Downloads. Sem fontes embutidas, sem sugestões, sem texto de exemplo com endereços reais. Primeiro uso mostra o aviso legal (seção 8) e exige confirmação, gravada nas configurações.
- Formato aceito (formato de fonte .json): `{ "name": string, "downloads": [ { "title": string, "uris": [string], "uploadDate": string, "fileSize": string } ] }`. Campos extras são ignorados.
- Validação (parser puro em `tako-core`, sem rede):
  - resposta de até 5 MiB, no máximo 20 000 itens, `title` até 200 caracteres, até 8 `uris` por item, cada uma até 4 KiB; o que passar do limite é descartado com contagem no log, não derruba a fonte;
  - só `magnet:?xt=urn:btih:` (hash de 40 hex ou 32 base32) ou URL `https://` de arquivo `.torrent`; qualquer outro esquema (`file:`, `javascript:`, caminho local) é rejeitado;
  - `fileSize` é texto livre da fonte: serve só de dica, o tamanho real vem dos metadados do torrent; texto de fonte é sempre mostrado como texto simples, nunca como marcação;
  - JSON inválido, item quebrado ou campo ausente viram erro tipado (`thiserror`) por item/fonte, nunca `unwrap`/pânico. Testes com JSON truncado, tipos trocados, URIs hostis, Unicode estranho e arquivo gigante.
- Download da fonte pelo cliente HTTP que o projeto já usa (`ureq`), com tempo-limite, limite de redirecionamentos, só `https` (HTTP puro recusado), sem cookies e sem enviar identificador do usuário. A fonte é atualizada só quando o usuário abre a tela ou pede; nada em segundo plano.
- Persistência: lista de fontes (nome, URL, última atualização) no banco; o conteúdo fica em cache no disco para busca offline, e remover a fonte apaga o cache.

### 2. Motor de torrent

Recomendado: **`librqbit`** (Rust puro, Apache-2.0, compatível com AGPL-3.0): leitura de magnet, DHT, seleção de arquivos, pausa/retomada, verificação de partes, API de progresso. Custo: traz o `tokio` (hoje o workspace não tem runtime assíncrono); ele roda numa thread própria do módulo de downloads, com canal simples para a UI, sem espalhar `async` pelo resto. Entra em `[workspace.dependencies]` e em um crate novo `tako-downloads` (depende de `tako-core`; a UI só conversa por um trait de `tako_core`, com versão em memória para testes, como em `devices`). Antes de aceitar a dependência: conferir licença da árvore (`cargo deny`), tamanho do binário e manutenção ativa.

Sem recursos de risco: só a porta UDP do DHT, sem porta TCP de escuta e sem UPnP, sem execução de scripts ou plugins, sem "seed depois de concluir" por padrão (seção 7).

### 3. Onde salva

- Pasta padrão `<Downloads do usuário>/Tako/<título>/` (a pasta de downloads do SO), trocável em Configurações › Downloads. Verificação de espaço livre antes de começar (usa o `FreeSpace` já existente); sem espaço, não inicia e diz quanto falta.
- Nome da pasta saneado (sem `..`, separadores, nomes reservados do Windows, tamanho limitado). Os caminhos internos do torrent são validados contra *path traversal* (rejeita `..`, absolutos, links simbólicos), e tudo é gravado sob a pasta do download.
- Estado (hash, pasta, fonte de origem, jogo vinculado, estado, bytes) no banco local; ao reabrir o Tako, downloads em andamento voltam a "pausado" e retomam por ação do usuário ou pela configuração "retomar ao abrir".

### 4. Vínculo com o jogo da biblioteca

- Ao iniciar, o usuário escolhe o jogo da biblioteca ao qual o download pertence ("Vincular a um jogo existente" / "Adicionar como jogo novo"). Sugestão automática por comparação normalizada do título (sem acento, caixa, pontuação, sufixos de edição), mostrada como sugestão, nunca aplicada sozinha: a fonte é não confiável e um vínculo errado trocaria o estado de um jogo real.
- Um jogo tem no máximo um download ativo. O vínculo é por id do jogo (`Game.id`), guardado do lado do download, sem mexer no registro da loja.
- Enquanto baixa, o jogo mostra "Baixando · N%" (campo `installing`, o mesmo do SRC-005, que já é mostrado na lista, no destaque e nos detalhes). Cancelar o download desfaz o vínculo e pergunta se apaga os arquivos parciais.

### 5. Ao terminar: como o jogo fica "instalado e jogável" sem executar nada

O Tako **nunca** executa instalador, script, `setup.exe`, `.msi` ou o que veio no torrent (§6.7). Opções:

- **A. Só aceitar conteúdo que já é o jogo (portátil).** Ao concluir e verificar, o Tako mostra "Baixado em <pasta>". Se a pasta tem um executável do jogo, o usuário aponta qual é (mesmo fluxo do "Adicionar jogo" manual) e o jogo passa a instalado, com `launch` desse executável quando ele clicar em Jogar. Prós: nada roda sem ação explícita; reaproveita o fluxo que já existe; funciona para jogos já extraídos. Contras: não cobre torrents com instalador; e abrir depois o executável escolhido ainda é abrir um arquivo baixado, ainda que por ação do usuário e pelo mesmo caminho de qualquer jogo adicionado à mão (pergunta ao mantenedor nº 1).
- **B. O usuário instala por conta própria.** O Tako mostra "Baixado · Abrir pasta" (abre o gerenciador de arquivos do SO na pasta, sem executar nada). O usuário roda o instalador fora do Tako; depois o jogo é detectado pela fonte de biblioteca normal (loja/pasta) ou adicionado à mão, e o vínculo passa a apontar para ele (o estado "Baixando" some quando o jogo aparece instalado). Prós: nenhum risco para o Tako; zero responsabilidade sobre o que o instalador faz. Contras: o "jogável pelo Tako" do PRD acontece em dois passos.
- **C. Executar o instalador pelo Tako** (instalação silenciosa, elevação). Prós: experiência de um clique. Contras: viola §6.7, o Tako passaria a executar código não confiável de fonte desconhecida, possivelmente elevado. **Rejeitada.**
- **D. Escanear com antivírus antes.** Não substitui o A/B: não vai ao núcleo do problema e dependeria de integração por SO. Pode ser acrescentado depois como aviso, nunca como permissão para executar.

**Decisão do mantenedor: só a B.** Concluído, o item vai para CONCLUÍDOS com "Abrir pasta"; o Tako não oferece "Definir executável" para o conteúdo baixado (a opção A foi recusada: apontar e abrir um executável baixado conta como executar arquivo baixado, §6.7). O jogo só aparece como jogável quando o usuário o instala por conta própria e ele é detectado pela fonte de biblioteca normal, ou quando o usuário o adiciona à mão pelo fluxo de sempre. Em nenhum caso o Tako executa, desativa antivírus ou anti-cheat ou altera os arquivos baixados.

### 6. Instalações das lojas (SRC-005) no mesmo painel

As instalações disparadas pelo Tako numa loja (Steam, Epic, GOG...) aparecem no mesmo grupo BAIXANDO, com o ícone da loja e "pela Steam" no lugar da fonte, como no design. O Tako **não controla** esse download: a linha vem do `Game.installing` (que o SRC-005 já mantém); pausar e cancelar apenas abrem o cliente da loja; sem porcentagem, mostra "Instalando na Steam…" sem barra. A pílula da barra soma os dois tipos. No modelo de dados do painel, cada linha tem uma origem (`Torrent` ou `Store(GameStore)`), e só a primeira oferece controles reais.

### 7. Privacidade

- Torrent expõe o IP do usuário aos outros pares e ao tracker/DHT, e o enxame (hash e IP) pode ser monitorado por terceiros. Sem VPN isso é inevitável no protocolo. O Tako avisa isso no aviso de primeiro uso e em Configurações › Downloads, em texto simples.
- Padrões conservadores: DHT ligado (necessário para magnet), PEX no padrão da biblioteca, LSD desligado, **sem semear depois de concluir**, limite de velocidade de envio configurável, sem UPnP.
- O Tako não envia telemetria de downloads, não registra hashes nem URLs de fontes em logs (só contagens e códigos de erro), e as URLs das fontes ficam só no banco local.
- Opção de vincular a uma interface de rede específica (útil com VPN) fica fora da primeira versão (pergunta nº 4).

### 8. Avisos legais

- O Tako fornece só a ferramenta: não hospeda, não indexa, não recomenda e não fiscaliza fontes nem conteúdo. Está escrito no aviso de primeiro uso e na documentação, sem nomear fontes.
- O aviso diz que baixar material protegido sem autorização pode ser ilegal onde o usuário mora e que a responsabilidade é dele; o usuário confirma antes de adicionar a primeira fonte.
- A documentação e a interface não trazem exemplo de fonte real, nem link para elas (decisão do mantenedor de 2026-10-16). Isso inclui testes e fixtures: só fontes e magnets fictícios.
- Lojas e clientes podem ter termos que proíbem certos usos; o Tako não altera nem contorna DRM, licenças ou anti-cheat.

### 9. Fora do escopo (fica para o DL-002)

Real-Debrid e serviços equivalentes: conta/token do usuário (guardado no cofre do SO, nunca em log), conversão de magnet em link direto, download HTTP em vez de torrent, limites e erros do serviço. O DL-001 deve deixar um trait `Downloader` (torrent hoje, HTTP depois) para o DL-002 entrar sem reescrever o painel e o vínculo; nada além do trait é implementado antes.

### 10. Entrega sugerida (quando aceita)

1. `tako-core`: modelo de fonte e parser validado (sem rede), estado de download, trait `Downloader` + versão em memória.
2. `tako-downloads` com `librqbit`, banco e pasta de destino.
3. Interface: Configurações › Downloads (fontes, pasta, aviso), pílula e painel, vínculo no jogo.
4. Checklists em `docs/testing/` para o que depende de rede e disco reais.

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| `librqbit` embutido (recomendada) | Rust puro, Apache-2.0, sem dependência de C++, API própria de progresso e pausa | Traz `tokio`; projeto mais jovem; manutenção a acompanhar |
| `libtorrent-rasterbar` via binding (`libtorrent` crates) | Motor maduro e muito usado (BSD-3) | C++/Boost: build pesado nos 3 SOs, `unsafe` e FFI (a regra do projeto limita `unsafe` a `tako-platform`) |
| `cratetorrent` | Rust, MIT | Incompleto e sem manutenção relevante; sem DHT/magnet completo |
| Cliente externo (qBittorrent/Transmission por RPC) | Nenhum código de torrent no Tako | Depende de o usuário instalar e configurar; GPL separa por processo, mas o painel perde controle e privacidade; experiência ruim |
| Não baixar; só abrir o magnet no cliente padrão do SO | Zero responsabilidade técnica e legal | Não entrega o DL-001 (progresso no card, vínculo ao jogo) |
| Executar o instalador ao fim (opção C) | Um clique | Viola §6.7; risco de segurança |

## Consequências

- O DL-001 só entrega "instalado e jogável" sem passo manual quando o conteúdo já é o jogo (A); para instaladores, o fluxo é em dois passos (B). Isso muda o critério do PRD ("fica instalado e jogável") e deve ser aceito pelo mantenedor ou o texto do PRD ajustado.
- Nova dependência pesada (motor de torrent com `tokio`) e novo crate; aumenta o binário e a superfície de ataque de rede. A regra "biblioteca 100% offline" continua valendo: só o módulo de downloads usa a rede, e só quando o usuário o liga.
- O Tako recebe tráfego de pares desconhecidos: o motor precisa de limite de conexões, de tamanho de metadados e de arquivos, e atualização rápida em caso de falha de segurança dele.
- A UI do painel é única para torrent e lojas, mas o controle é diferente: o modelo de dados precisa guardar a origem.
- Risco legal e de reputação do projeto aberto: mitigado só em parte pelos avisos; a decisão do mantenedor de não listar fontes precisa ser mantida em código, docs, issues e testes.

## Decisões do mantenedor (2026-10-18)

1. Ao terminar: **só "Abrir pasta"** (opção B). A opção A sai; o critério do PRD passa a ser "baixado e vinculado; jogável depois que o usuário instala ou adiciona o jogo".
2. Motor: **`librqbit` com `tokio`** aceitos, condicionados à checagem de licença e manutenção com `cargo deny` antes do merge.
3. Privacidade e aviso legal: **os padrões desta ADR** — aviso de IP exposto e aviso legal com confirmação no primeiro uso, sem semear ao concluir, sem UPnP, limite de upload e função desligada até o usuário ligar nas Configurações. Interface de rede fixa (VPN) fica para depois.

## Ainda abertas (não bloqueiam a primeira versão)

- Revisão jurídica do texto do aviso legal antes de uma release estável.
- Limites das fontes (5 MiB, 20 000 itens, 8 URIs) e pasta padrão em Downloads do SO: valem os padrões desta ADR até alguém pedir outros.
