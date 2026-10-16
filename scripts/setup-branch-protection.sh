#!/usr/bin/env bash
# Protege main e develop conforme o CONTRIBUTING §1 e §3. Precisa ser rodado por um admin do repositório (@lippdev):
#   gh auth login            # conta lippdev
#   ./scripts/setup-branch-protection.sh
#
# Regras: nada entra sem PR; 1 aprovação de code owner; CI (build-test e pr-title) verde e branch atualizada;
# conversas resolvidas; sem force-push nem exclusão. Admins podem passar por cima em emergência (enforce_admins=false).
#
# Atenção: com isto ativo, a PR de um mantenedor só pode ser mergeada depois da aprovação do outro.
set -euo pipefail

REPO="${REPO:-lippdev/tako-launcher}"

for BRANCH in main develop; do
  echo "Protegendo $REPO@$BRANCH..."
  gh api -X PUT "repos/$REPO/branches/$BRANCH/protection" --input - <<'JSON'
{
  "required_status_checks": { "strict": true, "contexts": ["build-test", "pr-title"] },
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "required_approving_review_count": 1,
    "require_code_owner_reviews": true,
    "dismiss_stale_reviews": true
  },
  "restrictions": null,
  "required_conversation_resolution": true,
  "allow_force_pushes": false,
  "allow_deletions": false
}
JSON
done

echo "Pronto. Conferir em https://github.com/$REPO/settings/branches"
