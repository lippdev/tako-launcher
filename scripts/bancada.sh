#!/usr/bin/env bash
# Bancada compartilhada: dois agentes trabalhando no mesmo PC (docs/marco-3/jornada-paralela.md).
# Só um por vez compila, testa, abre o Tako ou integra na develop; o outro espera na fila.
#
#   scripts/bancada.sh run <quem> <comando...>   roda o comando com a trava
#   scripts/bancada.sh ci <quem>                 CI local (o mesmo do rust.yml) na worktree atual
#   scripts/bancada.sh integrar <quem> <msg>     rebase na develop, CI local e merge --no-ff na develop
#   scripts/bancada.sh status                    quem está com a trava
set -uo pipefail

LOCK="${TAKO_BANCADA_LOCK:-$HOME/.tako-bancada.lock}"
STALE_SECS=5400 # 90 min: só uma trava de agente que caiu dura tanto
MAIN="$(git worktree list --porcelain | sed -n '1s/^worktree //p')"
# Um target só para as worktrees: o disco é curto e as dependências compilam uma vez.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$MAIN/target}"

say() { echo "bancada: $*" >&2; }

acquire() {
  local who="$1" waited=0 since
  until mkdir "$LOCK" 2>/dev/null; do
    # A trava pode ser liberada entre o mkdir e o stat: arquivo sumido é só tentar de novo.
    if since="$(stat -c %Y "$LOCK/owner" 2>/dev/null)" && [ $(($(date +%s) - since)) -gt $STALE_SECS ]; then
      say "trava velha ($(cat "$LOCK/owner" 2>/dev/null)), liberando"
      rm -rf "$LOCK"
      continue
    fi
    [ $((waited % 120)) -eq 0 ] && say "esperando: $(cat "$LOCK/owner" 2>/dev/null || echo '?')"
    sleep 5
    waited=$((waited + 5))
  done
  echo "$who desde $(date +%H:%M:%S): ${*:2}" >"$LOCK/owner"
  trap 'rm -rf "$LOCK"' EXIT INT TERM
  own_target
}

# O cargo decide o que recompilar pela data dos arquivos, não pelo caminho: no target/ dividido, a worktree Y
# reaproveita o que a X compilou se os arquivos da Y forem mais antigos que essa compilação. Vale para o código,
# para os build scripts (o do tako-core embute os catálogos e o do tako-ui o tema padrão, por caminho absoluto) e
# para os include_str!. Quando a worktree muda, os arquivos dela passam a ter a data de agora e o cargo recompila
# os crates do Tako (as dependências de fora continuam no cache).
own_target() {
  local here last mark="$CARGO_TARGET_DIR/.bancada-worktree"
  here="$(git rev-parse --show-toplevel)"
  last="$(cat "$mark" 2>/dev/null)"
  [ "$last" = "$here" ] && return
  say "o target/ estava com ${last:-?}; marcando os arquivos de $here como novos"
  (cd "$here" && git ls-files -z | xargs -0 touch -c --) || return 1
  mkdir -p "$CARGO_TARGET_DIR" && echo "$here" >"$mark"
}

ci() {
  cargo fmt --all --check || return 1
  cargo clippy --workspace --all-targets --locked -- -D warnings || return 1
  cargo test --workspace --locked || return 1
  cargo build -p tako --release --locked || return 1
  say "CI local verde em $(git rev-parse --abbrev-ref HEAD)"
}

integrar() {
  local msg="$1" branch
  branch="$(git rev-parse --abbrev-ref HEAD)"
  case "$branch" in develop | main | HEAD) say "integre de uma branch de trabalho, não de $branch"; return 1 ;; esac
  [ -f "$msg" ] || { say "arquivo de mensagem não existe: $msg"; return 1; }
  [ -z "$(git status --porcelain)" ] || { say "worktree com mudanças não commitadas"; return 1; }
  [ "$(git -C "$MAIN" rev-parse --abbrev-ref HEAD)" = develop ] && [ -z "$(git -C "$MAIN" status --porcelain)" ] ||
    { say "o checkout principal ($MAIN) precisa estar limpo na develop"; return 1; }

  if ! git rebase develop; then
    git rebase --abort
    say "conflito no rebase com a develop: resolva fora da trava (git rebase develop) e rode integrar de novo"
    return 2
  fi
  ci || { say "CI local vermelho: nada foi integrado"; return 1; }
  git -C "$MAIN" merge --no-ff -F "$msg" "$branch" || { git -C "$MAIN" merge --abort; return 1; }
  git switch --detach develop && git branch -d "$branch"
  say "integrado: $(git -C "$MAIN" log -1 --format='%h %s')"
}

cmd="${1:-status}"
shift || true
case "$cmd" in
  run) acquire "$1" "${@:2}"; "${@:2}" ;;
  ci) acquire "$1" ci; ci ;;
  integrar) acquire "$1" integrar "$2"; integrar "$2" ;;
  status) cat "$LOCK/owner" 2>/dev/null || echo "livre" ;;
  *) say "comando desconhecido: $cmd"; exit 64 ;;
esac
