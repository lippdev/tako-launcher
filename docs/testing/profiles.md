# Perfis locais (UX-004)

Configurações › Geral › Perfil: lista dos perfis (o em uso marcado), **Novo perfil**, e por perfil **Usar**,
**Renomear** e **Apagar**. O principal e o perfil em uso não podem ser apagados. Também tem **Perguntar quem
está jogando ao abrir**. Até 8 perfis, nomes de 1 a 32 caracteres, sem repetir.

Cada perfil é uma pasta de dados: o principal é a pasta de sempre (quem já usava o Tako não perde nada e nada
é migrado); os outros ficam em `profiles/<id>/`. Do perfil: banco (biblioteca, favoritos, status, tempo de jogo,
sessões, coleções, tags, configurações por jogo e do Tako), imagens escolhidas e temas. Da máquina: logs,
caches de imagens e metadados, a sessão de jogo (restauração de tela e áudio), atualizações, a trava de
instância única e o cofre do SO (chaves de API). A lista fica em `profiles.toml`, validada ao ler: arquivo
quebrado ou editado nunca aponta para fora de `profiles/`.

Trocar de perfil grava o perfil em uso e reabre o Tako com ele: com supervisor, a interface fecha com o motivo
`profile` e o supervisor a reabre (relendo os atalhos da Central do perfil novo); sem supervisor, abre outro
Tako e fecha este. Não troca com um jogo aberto. Um perfil novo começa vazio e passa pelo assistente da
primeira execução. "Quem está jogando?" aparece só numa abertura comum (não na volta de um jogo, na Central,
por link nem logo depois de uma troca).

## Teste manual pendente

- Criar um perfil, trocar para ele: o Tako reabre com o assistente; marcar favoritos e jogar um pouco; voltar
  ao principal: favoritos e tempo de jogo do principal intactos.
- Ligar "Perguntar quem está jogando", fechar e abrir o Tako: aparece a escolha; escolher o outro perfil não
  pergunta de novo na reabertura. Jogar e voltar: não pergunta.
- Atalho da Central diferente em cada perfil: depois da troca vale o do perfil novo (com supervisor).
- Tentar trocar com um jogo aberto (pela Central): aviso, nada muda.
- Apagar um perfil: some da lista e a pasta `profiles/<id>` é apagada; os jogos instalados continuam.
