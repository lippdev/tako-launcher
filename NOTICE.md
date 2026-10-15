# Avisos de terceiros

O Tako é distribuído sob a AGPL-3.0-only (ver `LICENSE`). Ele inclui ou adapta os componentes abaixo, cada um sob a
sua própria licença.

## Intel PresentMon 2.6.0

O contador de FPS (ADR-0017) usa o `PresentMon.exe` oficial da Intel, distribuído sem alterações na pasta
`PresentMon` ao lado do `Tako.exe`. Fonte: <https://github.com/GameTechDev/PresentMon> (release v2.6.0,
`PresentMon-2.6.0-x64.exe`, SHA-256 `b2a706bc6ad475749e3b7e3409263aa1e6906d45bdcf993f6dbc0f660188f1af`).

```
Copyright (C) 2017-2024 Intel Corporation

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"),
to deal in the Software without restriction, including without limitation
the rights to use, copy, modify, merge, publish, distribute, sublicense,
and/or sell copies of the Software, and to permit persons to whom
the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included
in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL
THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES
OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE
OR OTHER DEALINGS IN THE SOFTWARE.
```

## CapFrameX

Partes do código do contador de FPS foram adaptadas do CapFrameX (<https://github.com/CXWorld/CapFrameX>): os
argumentos do PresentMon em tempo real e a leitura do stdout (`CapFrameX.PresentMonInterface`) e o cálculo de FPS
médio, percentil e "1% low" (`CapFrameX.Statistics.NetStandard`). Os arquivos adaptados indicam a origem no topo:
`src/ConsoleMode.Runtime/SessionTools/PresentMon.cs`, `src/ConsoleMode.Runtime/SessionTools/FrameStats.cs` e
`src/ConsoleMode.Runtime.Windows/SessionTools/PresentMonFpsCounter.cs`.

```
MIT License

Copyright (c) 2018-2026 DevTechProfile

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## PyWebOSTV (manifesto de registro das TVs LG)

O registro nas TVs LG (`src/ConsoleMode.Runtime/Tv/WebOsProtocol.cs`) usa o manifesto assinado do "LG Remote App"
(bloco `signed` e a assinatura) como publicado pelo PyWebOSTV (<https://github.com/supersaiyanmode/PyWebOSTV>,
`pywebostv/connection.py`). O código do Tako é próprio; só esses dados do protocolo vêm de lá. O manifesto tem origem
no LG Connect SDK (Apache-2.0).

```
MIT License

Copyright (c) 2017 Srivatsan Iyer

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Serviços de dados de jogos

O Tako não distribui dados nem imagens de jogos: eles são baixados em tempo de execução para o cache local do usuário.

- **Steam Store** (Valve): descrição, gêneros, lançamento, empresas, nota do Metacritic e artes, pela API pública da
  loja.
- **IGDB** (<https://www.igdb.com>, Twitch Interactive): dados e imagens fornecidos pela IGDB.com, usada só quando o
  usuário informa a própria chave da Twitch (ADR-0021), conforme os termos de uso da API da Twitch/IGDB. A IGDB não
  endossa o Tako. A atribuição aparece em Configurações › Dados dos jogos (IGDB).
- **ScreenScraper** (<https://www.screenscraper.fr>): dados e mídias de jogos de console fornecidos por ScreenScraper.fr,
  banco de dados comunitário, usado só para ROMs quando o usuário informa a própria conta e a versão do Tako traz as
  credenciais do software (ADR-0022). Obrigado aos colaboradores do ScreenScraper. Os créditos aparecem em
  Configurações › Dados de ROMs (ScreenScraper).
