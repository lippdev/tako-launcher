# Roteiro manual — STREAM-001 Sunshine/Apollo e Moonlight

Execute em Windows, Linux e macOS. O Tako nunca envia a biblioteca para a rede: a exportação grava somente o `apps.json` local do Sunshine/Apollo.

## Preparação

1. Instale Sunshine ou Apollo e adicione manualmente um app de teste ao `apps.json`.
2. Adicione ao Tako pelo menos um jogo instalado que possa abrir; mantenha também uma entrada manual no `apps.json`.
3. Abra **Configurações › Integrações** e confira o estado instalado/em execução.

## Exportação

1. Escolha **Exportar jogos para o Sunshine**. Confira a lista e a contagem na prévia.
2. Feche a prévia sem confirmar; confirme que o `apps.json` não mudou.
3. Abra novamente e confirme. Confira que o Tako criou `apps.json.bak` (e `.bak.last` nas gravações seguintes) e preservou todas as entradas do usuário.
4. Confira no Sunshine real que o `cmd` gerado usa aspas duplas (`"caminho" --open-url "tako://play/<id>"`) em todas as plataformas (Linux, macOS, Windows) e que os apps abrem o jogo pela rota `tako://play/<id>` (o Sunshine separa argumentos via boost.process, que só reconhece aspas duplas, não aspas simples); verifique também caminhos com espaços.
5. Exporte de novo e confira que não há duplicatas.
6. Troque `apps.json` por JSON malformado, tente exportar e confirme que o arquivo e o backup anterior continuam intactos.

## Sincronização e cliente

1. Confirme que **Manter sincronizado ao reescanear** inicia desligado.
2. Ative a opção, confirme a prévia e reexamine a biblioteca; confira que jogos novos são exportados.
3. Desative a opção, reexamine e confirme que os apps não mudaram.
4. No Moonlight, conecte-se ao Sunshine/Apollo e abra um app exportado.

## Particularidades por plataforma

- **Windows:** faça o teste numa instalação cuja pasta de configuração permita gravação pelo usuário; confirme que a falha de permissão aparece sem perder o backup.
- **Linux:** valide a instalação nativa e a instalação Flatpak (`~/.var/app/dev.lizardbyte.app.Sunshine/config/sunshine/apps.json`); a lista padrão nativa fica em `~/.config/sunshine/apps.json`. Quando o Sunshine estiver em Flatpak, valide que o campo `cmd` das entradas geradas contém o prefixo `flatpak-spawn --host <comando>`. Se o Tako rodar como Flatpak ou AppImage, valide o comando gerado (`flatpak run <app-id>` ou `$APPIMAGE`) e o aviso de permissão (`sunshine_err_flatpak_permission`) caso a sandbox não possa gravar na pasta do Sunshine.
- **macOS:** valide Sunshine/Apollo em `/Applications` e a lista padrão em `~/.config/sunshine/apps.json`.

## Referência visual

A linha e o grupo seguem `tako-design/components/LinhaDeConfiguracao/README.md` (DeskRow, tokens de superfície, espaçamento e pílula). Capturas desktop devem ser conferidas em 1600×900 e 960×600.
