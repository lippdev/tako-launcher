# Spike MVP-007: integração oficial com o FSE (Xbox mode)

- Status: pesquisa concluída; experimento no Windows pendente
- Data: 2026-10-14
- Branch: `spike/MVP-007-fse`
- Riscos do PRD: "FSE não permitir launcher de terceiros em PCs comuns" (crítico), H1, gate técnico do épico MVP-B

## Pergunta

O Tako consegue se registrar e iniciar como app principal ("home app") do Full Screen Experience do Windows 11 por um caminho oficialmente suportado? Em quais builds e dispositivos?

## Resumo

1. **O mecanismo existe e é o mesmo para qualquer app**: um pacote MSIX com a extensão `windows.gamingApp` e a *custom capability* `Microsoft.appCategory.gamingHome_8wekyb3d8bbwe`. Apps com isso aparecem em Configurações > Jogos > Xbox mode > Escolher app inicial.
2. **A capability pertence à Microsoft** (sufixo `_8wekyb3d8bbwe`). Em produção, ela só é concedida com um SCCD **assinado pela Microsoft** para o nosso Package Family Name e certificado. Sem isso, só funciona com o catálogo de desenvolvimento (`FFFF`), que exige **Modo de Desenvolvedor**.
3. **Nenhum projeto de terceiros encontrado tem SCCD assinado.** AnyFSE, Playnite-FSE-Launcher e OmniConsole usam `FFFF` e mandam o usuário ligar o Modo de Desenvolvedor e confiar num certificado.
4. **OEMs podem escolher um launcher de terceiros** na imagem do Windows (`GamingHomeApp` no unattend). É um caminho de parceria com fabricante, não algo que o usuário final ou nós possamos ativar.
5. **Disponibilidade**: renomeado para **Xbox mode** em 2026. Exige Windows 11 24H2+; em handhelds desde out/2025; em desktops, notebooks e tablets em rollout gradual desde 30/04/2026, em "mercados selecionados" não divulgados, via Controlled Feature Rollout.
6. **Não há API documentada** para detectar se o modo está ativo ou qual é o home app atual. Ferramentas da comunidade usam sinais não documentados (registro `DeviceForm`, processo `XboxPcApp.exe`).

**Conclusão provisória para H1**: integração nativa com o FSE **não é distribuível hoje** sem aprovação da Microsoft. O gate técnico do PRD já prevê isso: o MVP sai como **prévia Fullscreen** (MVP-010), e a integração FSE vira um trilho de parceria.

## Como o registro funciona

Manifesto (trecho; versão completa em `packaging/fse-dev/AppxManifest.xml`):

```xml
<uap3:Extension Category="windows.appExtension">
  <uap3:AppExtension Name="windows.gamingApp" Id="App" ... PublicFolder="Public" />
</uap3:Extension>
...
<uap4:CustomCapability Name="Microsoft.appCategory.gamingHome_8wekyb3d8bbwe" />
```

SCCD na raiz do pacote. Em desenvolvimento:

```xml
<AuthorizedEntities AllowAny="true" />
<Catalog>FFFF</Catalog>
```

Em produção, o dono da capability gera um SCCD com `<AuthorizedEntity AppPackageFamilyName="..." CertificateSignatureHash="..."/>` e um catálogo assinado. O processo da Microsoft para custom capabilities pede o PFN (de uma conta da Microsoft Store), o nome da capability e o hash SHA-256 do certificado de assinatura. **Não encontramos um canal público** para pedir a `gamingHome`.

## Comportamento do Xbox mode que afeta a arquitetura

Da documentação de suporte da Microsoft (KB5070297):

- Uma janela por vez; apps rodam em tela cheia; minimizar/maximizar não mudam o estado da janela.
- Entrada e saída por Win+F11, Game Bar ou Task View. "Home" e "Library" da Game Bar voltam ao home app.
- No modo de inicialização (handhelds), o Windows reduz processos em segundo plano. **Apps de inicialização só sobem ao sair para o desktop**, a menos que o usuário marque "Iniciar ao fazer logon" em Configurações > Apps > Inicialização. Isso afeta como o `Launcher.Host` se inicia (MVP-006).

## Implicações para o projeto

- **ADR-003** (FSE oficial onde suportado, com fallback Fullscreen) continua válida, mas "onde suportado" passa a significar "quando a Microsoft autorizar o nosso pacote". Até lá, só Fullscreen convencional.
- **Empacotamento**: para ser elegível no futuro, o launcher precisa ter identidade de pacote (MSIX, ou pacote esparso com localização externa para o instalador Win32). Isso deve entrar na decisão do MVP-032 (instalador e portable).
- **Não recomendado como padrão**: pedir ao usuário que ligue o Modo de Desenvolvedor e confie num certificado para usar o modo FSE. É um rebaixamento de segurança da máquina inteira e vai contra os princípios do PRD (§10, Segurança). Se a equipe quiser oferecer isso, deve ser opt-in, fora do instalador padrão e com aviso explícito.
- **MVP-008 (detecção)**: sem API oficial, a detecção deve combinar sinais e responder "suportado", "indisponível" ou "indeterminado", nunca afirmar suporte sem certeza. Sinais candidatos: build ≥ 26100, pacote do Xbox app instalado, pacotes com `windows.gamingApp` (via `PackageManager`) e, se confirmado no experimento, onde o Windows guarda o home app escolhido.
- **Ação para os mantenedores**: procurar a Microsoft (programa de parceiros do Xbox / Windows gaming) para pedir a capability. Sem essa resposta, o MVP-009 ("demonstrar inicialização e retorno em ambiente com suporte oficial") não tem como ser cumprido em produção.

## Experimento no Windows (o que falta)

Objetivo: confirmar o mecanismo no hardware real, medir o ciclo de vida de um home app de terceiros e encontrar sinais de detecção. O experimento usa o caminho de desenvolvimento, que **não** é o de produção.

Ferramentas:
- `tools/Launcher.FseProbe`: sem argumentos, gera um relatório do ambiente; com `--home`, é a tela que o Windows abre como home app e grava o ciclo de vida em `Documentos\Tako-FseProbe\home.log`.
- `packaging/fse-dev/Register-FseProbe.ps1`: publica a probe e a registra como home app de desenvolvimento (sem MSIX nem certificado).

Sem o SDK do .NET: baixe `Tako-FseKit-…-win-x64.zip` na página de releases. O kit traz a probe compilada e o `LEIA-ME.txt` com estes mesmos passos (`.\probe\Launcher.FseProbe.exe` no lugar de `dotnet run`).

### Passo 1: relatório do ambiente (não muda nada no PC)

```powershell
dotnet run --project tools/Launcher.FseProbe
reg query HKCU /f GamingHomeApp /s
reg query HKLM\SOFTWARE /f GamingHomeApp /s
```

Anotar também se Configurações > Jogos mostra "Xbox mode" e quais opções aparecem.

### Passo 2: registrar a probe como home app (exige Modo de Desenvolvedor)

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\packaging\fse-dev\Register-FseProbe.ps1
```

Em Configurações > Jogos > Xbox mode, escolher "Tako FseProbe" como app inicial. Depois:

1. Entrar no Xbox mode (Win+F11). A probe abre em tela cheia?
2. Enter: abre o Bloco de Notas (o "jogo"). Fechar o Bloco de Notas: a probe volta ao primeiro plano?
3. Sair para o desktop (Win+F11) e entrar de novo: o `pid` no log muda (processo novo) ou continua (reaproveitado)? Aparecem `HEARTBEAT` entre a saída e a volta?
4. Task View (Win+Tab) e fechar a probe com X: o Windows reabre?
5. Rodar o Passo 1 de novo com a probe selecionada (para achar onde o Windows guarda o home app).

Mandar: saída do Passo 1 (antes e depois) e o `home.log`.

### Passo 3: limpeza

Voltar o app inicial para o Xbox app, rodar `.\packaging\fse-dev\Register-FseProbe.ps1 -Unregister` e desligar o Modo de Desenvolvedor.

### Matriz (preencher)

| Máquina | Build | Form factor | Xbox mode visível | Probe listada | Inicia como home | Retorno após jogo | Sair/entrar (pid) |
|---|---|---|---|---|---|---|---|
| PC do mantenedor | | | | | | | |

## Fontes

- Microsoft Support, [Windows Gaming: Full screen experience (KB5070297)](https://support.microsoft.com/en-gb/topic/windows-gaming-full-screen-experience-67fb8d12-5467-4a95-8adf-0a10789576ab)
- Microsoft Learn, [GamingHomeApp (unattend)](https://learn.microsoft.com/en-us/windows-hardware/customize/desktop/unattend/microsoft-windows-gaming-configuration-gaminghomeapp)
- Microsoft Learn, [Custom capabilities e SCCD (Hardware Support App)](https://learn.microsoft.com/en-us/windows-hardware/drivers/devapps/hardware-support-app--hsa--steps-for-app-developers)
- Windows Insider Blog, [Full screen experience expands to more Windows 11 PC form factors](https://blogs.windows.com/windows-insider/2025/11/21/full-screen-experience-expands-to-more-windows-11-pc-form-factors-for-windows-insiders/)
- Tom's Hardware, [Microsoft begins rolling out Xbox Mode to Windows 11 desktops and laptops](https://www.tomshardware.com/video-games/xbox/microsoft-begins-rolling-out-xbox-mode-to-windows-11-desktops-and-laptops)
- Windows Latest, [Xbox mode rollout expands (13/05/2026)](https://www.windowslatest.com/2026/05/13/microsoft-warns-windows-11s-xbox-mode-wont-show-up-yet-even-as-the-rollout-expands-to-more-users-today/)
- Projetos da comunidade (manifestos e SCCD lidos no código): [AnyFSE](https://github.com/ashpynov/AnyFSE), [Playnite-FSE-Launcher](https://github.com/leeson865/Playnite-FSE-Launcher), [OmniConsole](https://github.com/8bit2qubit/OmniConsole)
