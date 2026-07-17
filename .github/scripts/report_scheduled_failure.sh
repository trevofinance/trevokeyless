#!/usr/bin/env bash
# Opens (or comments on) a GitHub issue for a failed *scheduled* workflow run.
#
# A scheduled run has no pull request or push to report a status against, so without this
# anything the weekly cadence finds — a new advisory, a fuzz crash, a cross-language
# divergence — sits unnoticed in the Actions tab. Reuses one issue per title rather than
# opening a new one every week, so a persistent failure accumulates comments on a single
# thread.
#
# Requires: GH_TOKEN, TITLE, BODY. Invoked from the scheduled workflows.
set -euo pipefail

body="Run: ${GITHUB_SERVER_URL}/${GITHUB_REPOSITORY}/actions/runs/${GITHUB_RUN_ID}

${BODY}"

number="$(gh issue list --state open --limit 100 --json number,title |
  jq -r --arg title "$TITLE" '.[] | select(.title == $title) | .number' | head -n 1)"

if [ -n "$number" ]; then
  gh issue comment "$number" --body "$body"
else
  gh issue create --title "$TITLE" --body "$body"
fi
