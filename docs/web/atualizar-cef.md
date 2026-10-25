# Procedimento de Atualização do Runtime do Chromium (CEF)

Este documento define o processo operacional para que **toda versão estável do Chromium Embedded Framework (CEF) com correção de segurança vire release do runtime do Tako em poucos dias**.

- **Referência normativa principal:** [ADR-0035](../adr/0035-interface-web-rust-e-chromium.md), seção "Custos e riscos" ("Atualizações de segurança: quem embute o Chromium responde por ele").
- **Diretrizes de produto e arquitetura:** [PRD.md](../PRD.md), especialmente §2 (D-11: instalador e atualizador por componentes; D-13: testes locais durante a migração) e §11 (Riscos: Chromium embutido desatualizado).
- **Plano de tarefas:** [2026-10-24-interface-web.md](../plans/2026-10-24-interface-web.md), itens **WEB-41** (automação de aviso e procedimento), **WEB-02c** (sandbox do CEF no Windows) e épico **INST** (INST-01 / INST-02).

---

## 1. Responsável e Prazos (SLA)

| Critério | Definição |
|---|---|
| **Quem faz** | Mantenedor do Tako ou engenheiro/agente de infraestrutura e release designado. |
| **Gatilho de entrada** | Notificação automática via issue fixa criada pelo workflow `.github/workflows/cef-check.yml`, ou execução periódica manual de `scripts/cef-check.ps1`. |
| **Prazo regular (SLA)** | Até **3 a 5 dias úteis** após a publicação da versão estável do CEF com correções de segurança no índice público do Spotify CDN e na crate `cef`. |
| **Prazo crítico / Emergência (0-day)** | Até **24 a 48 horas** em caso de CVE de alta gravidade com exploração ativa no motor Chromium (divulgada pelo [Chrome Releases](https://chromereleases.googleblog.com/search/label/Stable%20Updates)). |

---

## 2. Passo a Passo para Subir a Versão

### Passo 1: Recebimento do alerta e levantamento de notas de versão

1. O workflow `.github/workflows/cef-check.yml` roda semanalmente às segundas-feiras (ou sob demanda via `workflow_dispatch`) e, ao encontrar uma versão mais recente do CEF estável no Spotify CDN ou da crate `cef` no crates.io, abre ou atualiza a issue fixa **"Nova versão estável do CEF"**.
2. Também é possível rodar o script localmente a qualquer momento:
   ```powershell
   .\scripts\cef-check.ps1
   ```
3. Consultar as notas de lançamento e vulnerabilidades corrigidas:
   - **Índice oficial e notas de build do CEF:** [https://cef-builds.spotifycdn.com/index.html](https://cef-builds.spotifycdn.com/index.html)
   - **Atualizações de segurança do Chromium (Chrome Releases):** [https://chromereleases.googleblog.com/search/label/Stable%20Updates](https://chromereleases.googleblog.com/search/label/Stable%20Updates)
   - **Releases e notas da crate `cef-rs`:** [https://github.com/tauri-apps/cef-rs/releases](https://github.com/tauri-apps/cef-rs/releases)

---

### Passo 2: Atualização das dependências no workspace Rust

1. No `Cargo.toml` raiz do workspace, atualize a versão exata da crate `cef`:
   ```toml
   # Cargo.toml
   cef = { version = "=<nova_versão_da_crate>", default-features = false }
   ```
2. Atualize o `Cargo.lock` para resolver os metadados completos da nova versão:
   ```powershell
   cargo update -p cef --precise <nova_versão_completa>
   ```
   *Exemplo:* se a crate subiu para `156.2.0` com o runtime CEF `156.0.14`, a versão no lockfile será `156.2.0+156.0.14`.
3. Certifique-se de que a entrada `name = "cef"` no `Cargo.lock` reflita o novo par `binding+runtime`.

---

### Passo 3: Validação do `scripts/cef-setup.ps1` e cache compartilhado

1. O script `scripts/cef-setup.ps1` extrai a versão completa diretamente do `Cargo.lock` via expressão regular (`name = "cef"` / `version = "([^"+]+)\+([^"+]+)"`), garantindo que não seja necessário editar versões manuais nele.
2. Em um terminal PowerShell novo, execute o script em modo dot-source para preparar o cache compartilhado em `%LOCALAPPDATA%\tako-dev\`:
   ```powershell
   . .\scripts\cef-setup.ps1
   ```
   O script realizará:
   - Instalação da ferramenta `export-cef-dir` na versão correspondente à nova crate (`%LOCALAPPDATA%\tako-dev\cef-tools\<binding>`).
   - Download da distribuição minimal do CEF para Windows 64-bit e conferência do hash SHA-1 contra o índice oficial do Spotify CDN.
   - Extração do runtime em `%LOCALAPPDATA%\tako-dev\cef\<runtime>\cef_windows_x86_64`.
   - Exportação da variável de ambiente `CEF_PATH` e inclusão de CMake, Ninja e do runtime no `PATH`.
3. Execute o script de verificação para confirmar consistência interna e externa:
   ```powershell
   .\scripts\cef-check.ps1
   ```
   Deve exibir **"em dia"** e sair com código de retorno 0.

---

### Passo 4: Compilação do hospedeiro

> [!IMPORTANT]
> **Feche todas as instâncias do Tako** antes de compilar, executar o clippy ou rodar os testes. Se `libcef.dll` estiver em execução por algum processo, o sistema operacional travará a substituição do binário na pasta de saída.

1. Compile o Tako com a feature de interface web ativada:
   ```powershell
   cargo build -p tako --features web
   ```
2. O crate de ligação `cef-dll-sys` compilará o wrapper C++ contra os novos cabeçalhos e copiará os arquivos necessários (`libcef.dll`, `icudtl.dat`, `locales/`, `.pak`, etc.) para a pasta `target\debug\` ou `target\release\`.

---

### Passo 5: Testes locais de verificação de junção (PRD D-13)

Conforme estipulado no **PRD D-13**, durante a fase de migração os testes e validações de build rodam na máquina do desenvolvedor/agente antes de qualquer integração.

Execute a suíte de verificação local completa:
```powershell
. .\scripts\cef-setup.ps1

# 1. Formatação
cargo fmt --all -- --check

# 2. Linters estritos (sem e com a feature web)
cargo clippy -p tako-web -p tako --all-targets -- -D warnings
cargo clippy -p tako-web -p tako --all-targets --features tako/web -- -D warnings

# 3. Testes unitários e de integração de protocolo
cargo test -p tako-web -p tako-app

# 4. Testes nativos do hospedeiro CEF
cargo test -p tako-web --features cef

# 5. Validação de isolamento do binário sem Chromium
cargo build -p tako
cargo tree -p tako --edges normal,build | Select-String "cef"
# (Deve retornar zero ocorrências de cef ou cef-dll-sys sem a feature)

# 6. Build final com interface web
cargo build -p tako --features web

# 7. Checagem estática dos tipos TypeScript da interface
(cd web && npm run typecheck)
```

---

### Passo 6: Teste manual mínimo do hospedeiro CEF

O teste manual mínimo valida o ciclo de vida dos processos, a renderização dos esquemas customizados e o isolamento de memória:

1. Execute a página técnica de diagnóstico com o ciclo de reciclagem de processo ativado (D-10, WEB-02b):
   ```powershell
   . .\scripts\cef-setup.ps1
   $env:TAKO_WEB_TEST_ROOT = (Resolve-Path crates/tako-web/testpage).Path
   $env:TAKO_WEB_TEST_ART = Join-Path $env:TAKO_WEB_TEST_ROOT 'fixture.png'
   $env:TAKO_WEB_RECYCLE_MS = '20000'
   .\target\debug\tako.exe --web --data-dir "$PWD\target\web-cef-verify\manual"
   ```

2. **Itens a conferir visual e funcionalmente:**
   - **Janela:** abre na dimensão padrão 1600×900, com barra própria e título **Tako**.
   - **Renderização (`tako-app://`):** a página carrega corretamente; sem erros de inicialização V8 ou carregamento de recursos.
   - **Imagens (`tako-img://`):** a imagem de teste da fixture carrega e escala sem expor caminhos locais de arquivo.
   - **Ponte IPC:** comandos (`app.info`, `ui.state.set`, `library.scan`) completam o ciclo de ida e volta via JSON nos pipes de entrada/saída padrão.
   - **Interface descartável (D-10, ADR-0035):**
     - O processo do núcleo (`tako --web`) permanece vivo.
     - O processo filho (`tako --web-ui`) executa o CEF.
     - Ao decorrer o intervalo de reciclagem (`TAKO_WEB_RECYCLE_MS`), a janela fecha, o filho encerra e o sistema fica com **zero processos do Chromium** e sem a DLL `libcef.dll` carregada no processo do núcleo.
     - A interface reabre automaticamente restaurando o estado anterior (`ui.state`).
   - **Navegação externa:** links para domínios HTTP/HTTPS externos abrem no navegador padrão do sistema operacional, e não no interior da janela do CEF.

---

## 3. O que Conferir no Sandbox do Chromium (WEB-02c, Modelo Bootstrap)

No Windows, a segurança do runtime do Chromium depende da correta ativação do sandbox de processos restritos. Conforme a decisão do mantenedor em 25/10 ([WEB-02c](../plans/2026-10-24-interface-web.md) e [hospedeiro.md](hospedeiro.md)):

> A crate `cef` (v156) não expõe as funções `cef_sandbox_info_create/destroy` para Windows, e o pacote minimal oficial não contém a biblioteca estática `cef_sandbox.lib` (presente apenas na distribuição standard). Por isso, para o empacotador e instalador oficial (épico INST), o Tako adota o **modelo bootstrap oficial do CEF**: o executável da interface do Tako vira uma DLL carregada por um executável de inicialização `bootstrap.exe` compilado com `cef_sandbox.lib`.

Ao atualizar a versão do CEF, os seguintes pontos do sandbox devem ser conferidos:

1. **Hash de compatibilidade do sandbox (`sandbox_compat`):**
   - Verifique a chave `sandbox_compat` da versão Windows 64-bit no índice público do Spotify CDN (`https://cef-builds.spotifycdn.com/index.json`).
   - Se o valor mudar em relação à versão anterior, os tipos internos e contratos de inicialização do sandbox mudaram no upstream do Chromium.
2. **Biblioteca `cef_sandbox.lib` da distribuição standard:**
   - Obtenha a distribuição standard do novo CEF para coletar a biblioteca `cef_sandbox.lib` compilada pelo upstream na mesma toolchain MSVC.
   - O runtime minimal continua sendo utilizado no ambiente de desenvolvimento (`scripts/cef-setup.ps1`), mas o pacote de release exige a biblioteca de sandbox.
3. **Recompilação do `bootstrap.exe`:**
   - Recompile o executável de bootstrap contra a nova biblioteca `cef_sandbox.lib` e os novos cabeçalhos `cef_sandbox_win.h`.
4. **Verificação de injeção de parâmetros de linha de comando:**
   - Em release, confira que `--no-sandbox` **não** esteja presente nos argumentos de execução dos subprocessos (`tako.exe --type=...` ou `bootstrap.exe --type=...`).
   - Garanta que `Settings.command_line_args_disabled = 1` seja mantido para que usuários ou scripts externos não possam desativar o sandbox ou abrir portas de depuração remota em binários de produção.
5. **Comunicação por pipes sob o Sandbox:**
   - Valide que o processo de renderização (`--type=renderer`) e o processo utilitário continuem conseguindo se comunicar com o processo do navegador através dos descritores herdados dos pipes anônimos sem bloqueio pelas políticas de tokens restritos do Windows.

---

## 4. Release e Distribuição do Runtime do Chromium

Conforme o **PRD D-11** e o épico **INST** (INST-01 e INST-02):
- O Tako adota o **modelo de instalador e atualizador por componentes**. O núcleo em Rust, o runtime do Chromium (arquivos CEF) e o pacote de interface web (`web/dist/`) são componentes independentes.
- Isso permite que uma atualização de segurança do CEF seja distribuída para todos os usuários sem exigir o download completo do executável do Tako ou de novos pacotes de interface, economizando largura de banda e tempo de deploy.

### Procedimento de empacotamento da release do runtime:

1. **Montagem do arquivo de distribuição:**
   - Reúna a estrutura de arquivos do CEF extraída da distribuição oficial:
     - `libcef.dll`, `chrome_elf.dll`
     - Bibliotecas de renderização gráfica: `d3dcompiler_47.dll`, `dxcompiler.dll`, `dxil.dll`, `vk_swiftshader.dll`, `vulkan-1.dll`
     - Arquivos de recursos e tradução: `icudtl.dat`, `resources.pak`, `chrome_100_percent.pak`, `chrome_200_percent.pak`, pasta `locales/`
     - Binário de bootstrap com sandbox integrado (`bootstrap.exe` do WEB-02c)
   - Comprima o pacote no formato padrão:
     `tako-runtime-cef-windows-x64-<versão-cef>.tar.xz` (ou `.zip`).
2. **Geração do Hash SHA-256:**
   ```powershell
   Get-FileHash -Path .\tako-runtime-cef-windows-x64-<versão-cef>.tar.xz -Algorithm SHA256
   ```
3. **Publicação no GitHub Releases:**
   - Publique o pacote na tag de release de componentes correspondente (ex.: `runtime-cef-v<versão>`) no repositório.
4. **Atualização do manifesto de componentes (`updates.json`):**
   - Atualize a seção do componente de runtime com a nova versão, URL do GitHub Releases e o SHA-256 verificado.
   - O serviço de atualização do Tako (`tako_core::updates`, PRD INST-02 e WEB-33) detectará o novo runtime e o aplicará de forma atômica na inicialização ou encerramento do launcher.
