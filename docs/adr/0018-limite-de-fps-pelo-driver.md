# ADR-0018: Limite de FPS pelo driver de vídeo

- Status: Substituída por [ADR-0025](0025-fronteira-com-o-console-mode.md): o limite de FPS sai do Tako
- Data: 2026-10-15

## Contexto

O limite de FPS da Central de Controle usava o RivaTuner Statistics Server (RTSS): o Tako gravava `FramerateLimit` no perfil global pela `RTSSHooks64.dll`. O RTSS vai sair do Tako porque conflita com o MSI Afterburner (que usa o mesmo RTSS) e porque obriga o usuário a instalar um programa à parte. A medição de FPS passa para o PresentMon com overlay próprio (frente F1); o limite precisa de outro caminho, sem injeção no jogo (anti-cheat) e sem pedir elevação (UAC).

Os três fabricantes expõem um limitador no driver, com biblioteca instalada junto com o driver:

| Fabricante | Biblioteca | Recurso | Escopo |
|---|---|---|---|
| NVIDIA | `nvapi64.dll` (NVAPI, `nvapi_QueryInterface`) | DRS, configuração `FRL_FPS` (`0x10835002`, "Max Frame Rate"), 0 a 1023 | Perfil por aplicativo |
| AMD | `amdadlx64.dll` (ADLX) | `IADLX3DFrameRateTargetControl` (FRTC) ou `IADLX3DChill` (mín./máx.) | Global da placa |
| Intel | `ControlLib.dll` (IGCL) | `ctlGetSet3DFeature` com `CTL_3D_FEATURE_FRAME_LIMIT` (2) | Por aplicativo quando `PerAppSupport`, senão global |

## Decisão

1. **Contrato** em `ConsoleMode.Runtime/SessionTools`: `IFrameRateLimiter` (capacidades, ler, aplicar para o exe do jogo, restaurar) e `NullFrameRateLimiter`. É a mesma forma que a F1 cria para a Central; ao unir as frentes fica uma interface só (a da F1 com estes membros, ou esta).
2. **Lógica pura e testada** (`ConsoleMode.Runtime.Tests`):
   - `GpuSelection`: escolhe a placa que roda o jogo entre as do DXGI, sem adaptadores de software. Uma placa: ela. Preferência do Windows para o exe (`HKCU\Software\Microsoft\DirectX\UserGpuPreferences`, `GpuPreference=1` economia / `2` desempenho) decide entre integrada (menos memória dedicada) e dedicada (mais memória). Sem preferência: a dedicada, que é onde os drivers põem jogos. Empate fica com a primeira do DXGI (a da tela principal).
   - `DriverFrameRateLimiter`: usa o backend do fabricante da placa escolhida e **grava o "antes" em `driver-fps-limit.json` antes de qualquer escrita** (como o RTSS fazia). O "antes" só é lido na primeira mudança de cada jogo; "Sem limite" devolve o valor do usuário; o fim da sessão, o fechamento do Tako e a abertura seguinte a uma queda devolvem tudo. Sem conseguir ler o valor atual ou gravar o arquivo, nada é mudado.
   - Estados por fabricante (`NvidiaLimitState`, `AmdLimitState`, `IntelLimitState`) e a ordem segura do Chill (máximo nunca abaixo do mínimo em vigor).
   - `FpsLimits`: Sem limite, 30, 40, 45, 60, 90, 120, 144 e **Tela − 3** (taxa da tela principal − 3, para ficar dentro da faixa do VRR). 75 e 165 saíram; valores salvos viram o mais próximo.
   - `GameExecutables`: o exe vem do atalho (`LaunchTarget` terminado em `.exe`) ou, quando o jogo aparece, do processo acompanhado (`GameRunTracker.GameExecutablePaths`), pulando anticheat, relatório de falha e launchers.
3. **Backends Windows** (`ConsoleMode.Runtime.Windows/SessionTools`), P/Invoke com ponteiros de função, sem pacote nem DLL nossa. Cada biblioteca é carregada de `System32` com `NativeLibrary.TryLoad` só quando a Central precisa; ausente = "Driver … não encontrado", nunca exceção.
   - **NVIDIA**: sessão DRS → `FindApplicationByName`; sem perfil, cria "Tako · jogo.exe" com o exe. Restaurar: apaga o perfil criado (só se o nome for o do Tako), apaga a configuração se o perfil já existia sem limite próprio, ou volta o valor. IDs e estruturas do NVAPI SDK (github.com/NVIDIA/nvapi, **MIT**).
   - **AMD**: ADLX pela tabela de funções (a mesma ordem que os headers descrevem para C, `IADLX…Vtbl`): FRTC se a placa tiver, senão Chill com mínimo = máximo = limite. O ADLX não tem perfil por jogo, então o limite vale para a placa toda durante a sessão.
   - **Intel**: IGCL com `ctl_3d_feature_getset_t` (nome do exe, ou vazio = global). Com mais de uma placa Intel, aplica e restaura em todas.
4. **Sem elevação.** Não está documentado se o DRS da NVIDIA exige administrador para gravar; se `NvAPI_DRS_SaveSettings` responder `NVAPI_INVALID_USER_PRIVILEGE` (-137) ou o IGCL responder `CTL_RESULT_ERROR_INSUFFICIENT_PERMISSIONS`, o item vira "Indisponível sem permissão de administrador" até o Tako fechar. O Tako não pede UAC.
5. **Central de Controle**: a seção "Limite de FPS" não depende mais do RTSS; diz qual driver e se vale "só neste jogo" ou "em toda a placa". A primeira sondagem roda em segundo plano na abertura; antes dela o item mostra "verificando".

## Consequências

- O limite funciona sem instalar nada e não conflita com o Afterburner (o Tako não toca no RTSS para limitar).
- **Licenças**: NVAPI é MIT. Os SDKs do ADLX (AMD) e do IGCL (Intel) têm licenças próprias, não MIT; o Tako **não copia nem distribui** headers nem binários deles: as declarações foram escritas à mão com os números e layouts necessários para interoperar, e as bibliotecas vêm do driver do usuário. Fica para a revisão jurídica (LEGAL-001) confirmar esse uso.
- **Incertezas, sem hardware para validar** (checklist §13):
  - Se a mudança no DRS vale para um jogo já aberto ou só na próxima abertura. Por isso o limite é aplicado logo no início da sessão quando o exe é conhecido; em jogos da Steam o exe só aparece com o processo, e pode valer só na próxima vez.
  - Se `SaveSettings` exige administrador em drivers atuais.
  - Se o Chill aceita mínimo = máximo (se recusar o mínimo, o máximo ainda limita) e se FRTC existe nas Radeon recentes.
  - Layout das estruturas do IGCL (`ApplicationNameLength` sem o zero final) e se o limite global da Intel vale sem reiniciar o jogo.
  - Em notebooks híbridos sem preferência no Windows, o driver pode rodar o jogo na integrada; aí o limite vai para a placa errada.
- AMD global: se o Tako cair, outro jogo aberto antes da próxima abertura do Tako fica com o limite. A abertura seguinte devolve o valor.
- A F1 ([ADR-0017](0017-contador-de-fps-com-presentmon.md)) removeu o RTSS do Tako, junto com `IFpsOverlayService` (`SetFpsLimit`/`CanLimitFps`) e o backup `rtss-fps-limit.txt`.
