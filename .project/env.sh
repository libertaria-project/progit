# ProGit project environment
# Source before running prog: `source .project/env.sh`
# Or add to your shell rc via direnv / .envrc

export FORGEJO_URL="https://git.sovereign-society.org"
export FORGEJO_TOKEN="$(cat ~/.secrets/forgejo-deploy.token 2>/dev/null | grep VIRGIL_DEPLOY | cut -d= -f2)"

# Sync target (used by prog sync)
export PROGIT_SYNC_URL="https://git.sovereign-society.org"
