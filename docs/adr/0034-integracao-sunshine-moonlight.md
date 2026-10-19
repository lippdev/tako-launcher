# ADR-0034: Integração local com Sunshine/Apollo e cliente Moonlight

- Status: Aceita
- Data: 2026-10-19

## Contexto

STREAM-001 integra a biblioteca do Tako ao streaming local. O Sunshine e seu fork Apollo oferecem uma lista de aplicativos em `apps.json`; o Moonlight se conecta como cliente. A integração deve preservar configurações pessoais e não enviar dados para fora da máquina.

## Decisão

- Detectar Sunshine/Apollo instalado e em execução localmente nos três sistemas, sem rede.
- Exportar os jogos da biblioteca como aplicativos do Sunshine, gravando o comando do executável do Tako com `--open-url tako://play/<id>` em `apps.json`.
- Mostrar a prévia e pedir confirmação antes de gravar. Criar uma cópia de segurança do JSON original e recusar JSON inválido. Atualizações substituem apenas entradas reconhecidas pelo prefixo reservado `[Tako] ` e por um comando `tako://play/`; todas as demais entradas são mantidas.
- A sincronização automática após reexaminar a biblioteca é opcional, desligada por padrão, e só passa a valer depois da confirmação inicial do usuário.
- Moonlight funciona como cliente para conectar ao Sunshine/Apollo e iniciar os aplicativos exportados. O Tako não controla o cliente.
- Não usar API local nesta entrega; nenhuma credencial é necessária ou enviada.
- O Tako não solicita elevação de privilégios (UAC/sudo). Se a pasta do Sunshine estiver em local protegido (como `Program Files` no Windows) sem permissão de escrita para o usuário comum, o Tako exibe mensagem explicativa orientando o usuário a conceder permissão na pasta ou a executar o Tako como administrador.

## Consequências

A edição é local e pode exigir permissão de escrita dependendo da pasta de instalação/configuração do Sunshine. O Tako não pede elevação automática; em caso de falha de permissão no Windows ou na sandbox Flatpak, mensagens explicativas orientam o usuário. Os aplicativos exportados abrem o Tako, que inicia o jogo pelo fluxo normal. Sunshine documenta `apps.json` e os campos de aplicativo (`cmd`, `name`, `working-dir`) em sua [documentação de configuração](https://docs.lizardbyte.dev/projects/sunshine/master/md_docs_2configuration.html?lng=en-US) e [documentação de aplicativos](https://docs.lizardbyte.dev/projects/sunshine/master/md_docs_2getting__started.html).
