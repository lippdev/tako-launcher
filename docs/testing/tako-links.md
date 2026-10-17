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
