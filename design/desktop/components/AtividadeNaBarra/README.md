# AtividadeNaBarra

O que o TAKO faz em segundo plano aparece no botão **Downloads** da barra de cima (ícone ↓ em círculo).

- **Contador**: bolinha de 19 na cor de destaque com o número de atividades, borda de 2 na cor da barra. Pula (escala 1,35 em 400 ms) quando muda; some quando não há nada.
- **Clique**: abre o popover (PainelDownloads) 6 abaixo, alinhado à direita: título DOWNLOADS e "n ativos", uma linha por atividade (capa 36×46, nome, "Baixando · 59,8 / 108,7 GB", barra de 3 na cor de destaque, Pausar/Continuar de 30). Sem nada: "Nenhum download ativo".
- Atualizar biblioteca, procurar atualização do TAKO, buscar capas e sincronizar a nuvem entram na mesma lista com um giro no lugar da capa e sem barra quando não têm %.
- O resultado vai para o Aviso (2,3 s): "Watch Dogs 2 está pronto para jogar", "Cyberpunk 2077 atualizado".
- Na lista de jogos, o jogo baixando mostra a % na cor de destaque e uma barra de 3 no pé do ícone.

Base: protótipo do mantenedor (`launcher.html`, out/2026), tema TAKO. Prévia gerada por `gen/rebrand.py` a partir do protótipo rodando, com arte de exemplo.
