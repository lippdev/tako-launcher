Versão de teste do Tako: launcher de jogos em tela cheia para Windows 11, feito para controle.

## Downloads
- **Tako-…-Setup-x64.exe** — instalador. Instala por usuário (sem pedir administrador), cria atalho no Menu Iniciar e aparece em "Aplicativos instalados" para desinstalar. Dados em `%LOCALAPPDATA%\Tako`.
- **Tako-…-Portable-x64.zip** — portable. Extraia em qualquer pasta (pendrive, HD externo) e rode `Tako.exe`; biblioteca e caches ficam em `Data\` ao lado do executável.
- **Tako-FseKit-…-win-x64.zip** — kit do experimento do Xbox mode (FSE, MVP-007). Siga o `LEIA-ME.txt` do kit.

Nenhum deles precisa do .NET instalado.

## O que testar
- Jogos da Steam aparecem sozinhos, com capa, fundo, descrição, gêneros e nota do Metacritic; "Jogar" abre pela Steam.
- "Adicionar jogo" (Início, Biblioteca ou Configurações) para jogos fora da Steam; Detalhes › Opções para corrigir correspondência, trocar capa ou remover.
- Controle Xbox ou PlayStation (USB/Bluetooth), teclado (Shift+Tab abre a Central de Controle) e mouse.
- Telas 2K/4K e ultrawide.

O Windows SmartScreen pode avisar que o app não é reconhecido (ainda não é assinado): "Mais informações" › "Executar assim mesmo".
