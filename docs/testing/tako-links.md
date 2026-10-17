# Links externos (EXT-005)

O instalador Windows registra `tako://` para o usuário atual; o pacote Linux declara o handler no `.desktop`.
O portable não altera o registro: pode receber a URL como argumento único no terminal. No Linux, se o ambiente
não escolher o aplicativo automaticamente, `xdg-mime default tako.desktop x-scheme-handler/tako` seleciona-o.

| Link | Resultado |
|---|---|
| `tako://home` | Início |
| `tako://library` | Biblioteca inteira |
| `tako://library/installed` | Instalados |
| `tako://library/favorites` | Favoritos |
| `tako://settings` | Configurações |
| `tako://game/steam%3A570` | Detalhes do ID `steam:570` |
| `tako://play/steam%3A570` | Jogar pelo fluxo normal, com suas confirmações |

Também aceita `tako.exe --open-url "tako://settings"`. A chamada externa contém somente o link: não combine com
outras opções. IDs são os IDs internos da biblioteca, não nomes nem caminhos. `:` pode ser literal ou `%3A`.
Links não importam jogos, não mudam configurações, não aceitam argumentos de execução e não baixam arquivos.

Na versão publicada, um segundo processo encaminha o pedido ao supervisor e traz a interface para a frente.
Durante o jogo, o supervisor reabre a biblioteca mantendo a sessão; o fluxo normal impede iniciar outro jogo
sem confirmação. O pedido é consumido uma vez, sem repetir Jogar se a interface reiniciar. Durante a descoberta
da biblioteca, o pedido aguarda. Pedidos simultâneos usam a ativação existente: o último substitui o anterior.

## Teste manual pendente

- Windows instalado: abrir cada link pelo navegador/Executar, com Tako fechado, aberto e durante uma sessão.
- Autorizar o navegador a abrir o aplicativo e confirmar o foco da janela; testar caminho de instalação com espaços.
- Jogo inexistente: aviso, sem execução. Jogo com comandos pendentes: pedir o consentimento normal.
- Reabrir a interface após uma falha: não repetir o link de Jogar.
- Linux: instalar o pacote, verificar a associação no ambiente gráfico e o foco permitido pelo compositor.
- Desinstalar no Windows: remover a associação do Tako sem apagar os dados.

## Limitações

No macOS, declarar `CFBundleURLTypes` não basta: o toolkit precisa encaminhar eventos Apple `OpenURLs` ao app.
Esse caminho requer implementação/validação na plataforma; não declaramos uma associação que não funciona.
Com `--no-supervisor` (diagnóstico/desenvolvimento), o link inicial funciona, mas não há encaminhamento para
a janela já aberta. O registro/empacotamento dos SOs precisa de teste manual, além do CI Windows local.

## Imagem do SteamGridDB com um clique (ART-002)

Com o SGDBoop ligado no site (steamgriddb.com › Preferências), cada imagem ganha o botão "BOOP", que abre
`sgdb://boop/<tipo>/<número>`. O Tako trata esse link e a forma própria `tako://steamgriddb/<tipo>/<número>`
(`grid`, `hero`, `logo` ou `icon`; `/nonsteam` no fim é aceito). Pede o endereço da imagem ao SteamGridDB com a
chave do próprio usuário (a mesma do seletor de imagens) e só aceita imagem `https` do domínio do SteamGridDB.

- A imagem é de um app da Steam que está na biblioteca: aplica e fixa direto, com aviso.
- Senão: abre "Aplicar … em…" com os jogos da biblioteca, o de nome mais parecido primeiro (e o jogo aberto nos
  detalhes logo depois).
- Grade horizontal: aviso pedindo a capa vertical (o Tako só usa capa 600×900).
- Sem chave, chave recusada, sem rede ou imagem inexistente: aviso, nada muda.

A imagem fixada volta à automática pelo seletor de imagens, como as outras.

O instalador Windows só registra `sgdb://` se nenhum outro app tiver (o SGDBoop usa o mesmo), ou se já for do
Tako; a desinstalação só apaga a associação se ainda apontar para o Tako. O `.desktop` do Linux declara
`x-scheme-handler/sgdb`; para escolher o Tako: `xdg-mime default tako.desktop x-scheme-handler/sgdb`.

### Teste manual pendente

- Ligar o SGDBoop no site, clicar em BOOP numa capa de um jogo da Steam instalado: o Tako abre e aplica a capa.
- BOOP num fundo, logo e ícone; numa capa de jogo fora da Steam (escolher na lista); numa grade horizontal.
- **Confirmar que a API do botão aceita a chave pessoal do usuário.** Ela é a mesma do SGDBoop e não é
  documentada; se o SteamGridDB recusar (aviso de chave recusada com uma chave que funciona no seletor), o item
  precisa de um token de aplicativo cedido pelo SteamGridDB.
- Com o SGDBoop já instalado: o instalador não toma a associação; desinstalar o Tako não apaga a do SGDBoop.
