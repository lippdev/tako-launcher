#!/usr/bin/env bash
# Decide a versão e se a release sai (usado pelo .github/workflows/release.yml; roda também no Mac/Linux à mão).
#
#   push na main          estável: a versão é a primeira seção "## [X.Y.Z]" do CHANGELOG.md. Se for pré-release ou se
#                         a tag já existir, não publica (o push segue normal; o motivo vai para o resumo do job).
#   Run workflow, develop alpha ou beta: X.Y.Z vem de "base_version" ou do topo do changelog (o patch sobe se X.Y.Z já
#                         saiu como estável) e o N de -alpha.N/-beta.N é o próximo depois das tags existentes.
#   Run workflow, main    canal "stable": refaz a estável do topo do changelog (útil se o push falhou no meio).
#   Outra branch          só sem "publish" (compila e guarda os arquivos como artefato, para testar o pipeline).
#
# Entrada (variáveis de ambiente): EVENT_NAME, REF, CHANNEL (stable|alpha|beta), BASE_VERSION, PUBLISH (true|false).
# Saída: build, publish, version, prerelease, tag e reason em $GITHUB_OUTPUT (ou na tela) e as notas em
# $NOTES_FILE (padrão release-notes.md): inglês do CHANGELOG.en-US.md + link para o português.
set -euo pipefail

EVENT_NAME="${EVENT_NAME:-workflow_dispatch}"
REF="${REF:-$(git symbolic-ref -q HEAD || echo detached)}"
CHANNEL="${CHANNEL:-stable}"
BASE_VERSION="${BASE_VERSION:-}"
PUBLISH="${PUBLISH:-false}"
NOTES_FILE="${NOTES_FILE:-release-notes.md}"
REPOSITORY="${GITHUB_REPOSITORY:-lippdev/tako-launcher}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PT="$ROOT/CHANGELOG.md"
EN="$ROOT/CHANGELOG.en-US.md"

out() { if [[ -n "${GITHUB_OUTPUT:-}" ]]; then echo "$1=$2" >>"$GITHUB_OUTPUT"; else echo "$1=$2"; fi; }
fail() { echo "::error::$1" >&2; exit 1; }
skip() {
  echo "::notice::$1"
  out build false; out publish false; out version ""; out prerelease false; out tag ""; out reason "$1"
  exit 0
}

# Seção "## [versão]" de um changelog, até a próxima "## ", sem linhas vazias nas pontas.
section() {
  awk -v want="## [$2]" '
    index($0, "## ") == 1 { if (on) exit; on = ($0 == want); next }
    on { lines[++n] = $0 }
    END {
      s = 1; while (s <= n && lines[s] ~ /^[[:space:]]*$/) s++
      e = n; while (e >= s && lines[e] ~ /^[[:space:]]*$/) e--
      for (i = s; i <= e; i++) print lines[i]
    }' "$1"
}

# Primeira versão do changelog que não é [Unreleased].
top_version() { grep -m1 -oE '^## \[[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?\]' "$1" | sed -E 's/^## \[(.*)\]$/\1/'; }

tag_exists() { [[ -n "$(git tag -l "v$1")" ]]; }

semver_re='^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$'
branch="${REF#refs/heads/}"
[[ "$PUBLISH" == "true" || "$EVENT_NAME" == "push" ]] && publish=true || publish=false

case "$CHANNEL" in
  stable)
    if [[ "$publish" == "true" && "$branch" != "main" ]]; then fail "A estável só sai da main (rodando em $branch)."; fi
    version="$(top_version "$PT")"
    [[ -n "$version" ]] || fail "CHANGELOG.md não tem nenhuma seção ## [X.Y.Z]."
    if [[ "$version" == *-* ]]; then
      [[ "$EVENT_NAME" == "push" ]] && skip "O topo do CHANGELOG.md é a pré-release $version; a main só publica estável. Adicione ## [X.Y.Z] para lançar."
      fail "O topo do CHANGELOG.md é a pré-release $version; o canal stable precisa de ## [X.Y.Z]."
    fi
    if tag_exists "$version"; then
      [[ "$publish" == "true" ]] && skip "A tag v$version já existe; nada a publicar."
    fi
    ;;
  alpha | beta)
    if [[ "$publish" == "true" && "$branch" != "develop" ]]; then fail "Alpha e beta só saem da develop (rodando em $branch)."; fi
    if [[ -n "$BASE_VERSION" ]]; then
      base="${BASE_VERSION#v}"
      [[ "$base" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "base_version inválida: '$BASE_VERSION' (use X.Y.Z)."
      tag_exists "$base" && fail "A estável v$base já saiu; escolha uma base_version maior."
    else
      top="$(top_version "$PT")"
      [[ -n "$top" ]] || fail "CHANGELOG.md não tem nenhuma seção ## [X.Y.Z]; informe base_version."
      base="${top%%-*}"
      if tag_exists "$base"; then
        IFS=. read -r major minor patch <<<"$base"
        base="$major.$minor.$((patch + 1))"
      fi
    fi
    # alpha < beta < rc no SemVer: uma alpha depois de uma beta da mesma base ficaria "mais velha" e o app não a oferece.
    if [[ "$CHANNEL" == "alpha" && -n "$(git tag -l "v$base-beta.*" "v$base-rc.*")" ]]; then
      fail "v$base já tem beta/rc; uma alpha nova seria mais antiga. Use beta ou outra base_version."
    fi
    last="$(git tag -l "v$base-$CHANNEL.*" | sed -nE "s/^v$base-$CHANNEL\.([0-9]+)$/\1/p" | sort -n | tail -1)"
    version="$base-$CHANNEL.$(( ${last:-0} + 1 ))"
    ;;
  *) fail "Canal inválido: $CHANNEL (use stable, alpha ou beta)." ;;
esac

[[ "$version" =~ $semver_re ]] || fail "Versão inválida: $version"
tag_exists "$version" && [[ "$publish" == "true" ]] && fail "A tag v$version já existe."

# Notas: a seção da própria versão; numa pré-release sem seção, a [Unreleased] (o que entrou desde a última).
notes_for() {
  local text
  text="$(section "$1" "$version")"
  if [[ -z "$text" && "$version" == *-* ]]; then text="$(section "$1" Unreleased)"; fi
  printf '%s' "$text"
}
en_notes="$(notes_for "$EN")"
pt_notes="$(notes_for "$PT")"
if [[ -z "$en_notes" || -z "$pt_notes" ]]; then
  msg="Os dois changelogs precisam de notas para $version (seção ## [$version]"
  [[ "$version" == *-* ]] && msg="$msg ou ## [Unreleased]"
  msg="$msg em CHANGELOG.md e CHANGELOG.en-US.md)."
  [[ "$publish" == "true" ]] && fail "$msg"
  echo "::warning::$msg"
fi

{
  printf '%s\n\n---\n\n' "${en_notes:-_No release notes._}"
  printf '🇧🇷 Notas em português: [CHANGELOG.md](https://github.com/%s/blob/v%s/CHANGELOG.md)\n' "$REPOSITORY" "$version"
} >"$NOTES_FILE"

[[ "$version" == *-* ]] && prerelease=true || prerelease=false
out build true
out publish "$publish"
out version "$version"
out prerelease "$prerelease"
out tag "v$version"
out reason ""
echo "Versão: $version (pré-release: $prerelease, publicar: $publish)"
