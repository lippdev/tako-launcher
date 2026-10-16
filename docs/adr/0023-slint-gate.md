# ADR-0023: Slint na interface e resultado do gate do MIG-001

- Status: Aceita, com números de referência pendentes (ver Consequências)
- Data: 2026-10-16

## Contexto

O PRD v0.2 (D-02) escolheu Slint para a interface, condicionado a um spike (MIG-001): grade de 1.000 capas,
foco animado, controle via SDL3 e um tema `.slint` carregado em tempo de execução, cumprindo as metas da §8.1 em
duas máquinas de referência. Se falhar, a alternativa é Avalonia/.NET, com a decisão voltando aos mantenedores.

O spike está em `crates/spike-slint` (`cargo run --release -p spike-slint -- --bench`). Ele compila o tema com o
`slint-interpreter` em tempo de execução, monta a grade com virtualização manual (só as linhas visíveis existem),
gera as capas sob demanda num LRU de 96 MB, navega sozinho pela grade inteira a 60 passos por segundo, simula
uma sessão de jogo (esconde a janela e libera texturas e caches) e imprime os números em JSON.

## Números medidos

Ambiente: contêiner Linux x64 de 4 núcleos, **sem GPU** (Xvfb, Mesa llvmpipe, que emula OpenGL na CPU). Servem
para achar erros de arquitetura, não para o gate.

| Métrica | Meta (§8.1) | FemtoVG (OpenGL via llvmpipe) | Software |
|---|---|---|---|
| Tema compilado em runtime | sim | 5 ms | 5 ms |
| Primeira tela interativa | ≤ 1,5 s | 0,24 s | não medido (sem notificador de quadros) |
| Quadros na navegação | 60 fps | 21 fps (CPU emulando a GPU) | não medido |
| Resposta ao controle (p95) | ≤ 50 ms | 48–53 ms | não medido |
| Memória em repouso | ≤ 150 MB | 245–260 MB (o llvmpipe guarda as texturas na RAM) | 137 MB |
| Memória durante o jogo, mesmo processo | ≤ 40 MB | 141 MB (bibliotecas do Mesa/LLVM) | 34 MB |

Primeira versão do spike: 690 MB, porque o modelo de linhas criava as 1.000 capas de uma vez. Passar a capa por
um callback (`cover-image(índice)`), chamado só pelas linhas visíveis, resolveu. Fica como regra: **imagens nunca
vão dentro dos modelos que o tema percorre inteiros**.

## Decisão

1. **Slint fica (D-02).** O tema em runtime funciona pelo `slint-interpreter`, compila em milissegundos e o modelo
   de dados (structs, modelos, callbacks) cobre a API de tema da §7. O tema padrão também passa pelo intérprete,
   então qualquer tema usa o mesmo caminho do padrão.
2. **Durante o jogo, o processo da interface termina.** O supervisor (mesmo binário, ver ADR-0026) acompanha a
   sessão, restaura tela e áudio e reabre a interface no fim, no jogo de onde o usuário saiu. Assim a meta de
   memória e "zero quadros" durante o jogo não depende do driver de vídeo nem do renderizador.
3. **Renderizador:** FemtoVG (OpenGL) por padrão; o software fica como alternativa (`SLINT_BACKEND=winit-software`
   ou a opção de diagnóstico), para GPUs problemáticas.
4. **Imagens:** decodificadas fora da thread da interface, no tamanho em que aparecem, num cache LRU por bytes.

## Consequências

- **Gate pendente em hardware real.** Os números de quadros, latência e memória com GPU precisam vir de duas
  máquinas de referência (Steam Deck ou equivalente, e um PC com Windows) rodando o spike ou `tako --bench`
  (MIG-041). O desenvolvimento seguiu porque a arquitetura (item 2) tira o risco principal (memória durante o
  jogo) e os outros números estão perto da meta mesmo sem GPU. Se as máquinas de referência reprovarem, os
  números voltam aos mantenedores, como manda a D-02.
- Reabrir a interface no fim do jogo custa a abertura a frio (0,24 s medidos, meta de 1,5 s).
- A central rápida aberta por cima do jogo também abre uma interface nova; o atalho é ouvido pelo supervisor.
