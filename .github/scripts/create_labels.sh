#!/bin/bash
# Run this script once to create all project labels on GitHub.
# Requires GitHub CLI (gh) to be installed and authenticated.
# Usage: bash .github/create-labels.sh <owner/repo>

REPO=${1:?"Usage: $0 <owner/repo>"}

# List of default GitHub labels to remove
DEFAULT_LABELS=(
  "enhancement"
  "good first issue"
  "invalid"
)

echo "Removing default GitHub labels from $REPO..."

for label in "${DEFAULT_LABELS[@]}"; do
  gh label delete "$label" --repo "$REPO" --yes 2>/dev/null
done

echo "Creating new labels for $REPO..."

# Type
gh label create "bug" --repo "$REPO" --color "d73a4a" --description "Bug or unexpected behavior" --force
gh label create "fix" --repo "$REPO" --color "0e8a16" --description "Fix for a bug or regression" --force
gh label create "feature" --repo "$REPO" --color "a2eeef" --description "New feature or improvement" --force
gh label create "refactor" --repo "$REPO" --color "8a2be2" --description "Code changes that do not add features" --force
gh label create "chore" --repo "$REPO" --color "b2b2b2" --description "Maintenance and chores" --force
gh label create "dependencies" --repo "$REPO" --color "0366d6" --description "Dependencies updates" --force
gh label create "documentation" --repo "$REPO" --color "0366d6" --description "Documentation" --force
gh label create "question" --repo "$REPO" --color "5bc0de" --description "Further information is requested" --force
gh label create "wontfix" --repo "$REPO" --color "ff8c00" --description "Will not fix" --force
gh label create "duplicate" --repo "$REPO" --color "9b9b9b" --description "Issue or PR that already exists" --force
gh label create "help wanted" --repo "$REPO" --color "b31b1b" --description "Extra attention required from contributors" --force

# Priority
gh label create "priority: high" --repo "$REPO" --color "b60205" --description "Needs immediate attention" --force
gh label create "priority: medium" --repo "$REPO" --color "fbca04" --description "Normal priority" --force
gh label create "priority: low" --repo "$REPO" --color "0075ca" --description "Can be deferred" --force

# Status
gh label create "blocked" --repo "$REPO" --color "ee0701" --description "Blocked by another issue" --force
gh label create "needs discussion" --repo "$REPO" --color "cc317c" --description "Requires a decision first" --force

# Area
gh label create "area: core" --repo "$REPO" --color "6f42c1" --description "Related to core functionality" --force
gh label create "area: tui" --repo "$REPO" --color "6e6e6e" --description "Related to text-bases user interface" --force
gh label create "area: test" --repo "$REPO" --color "28a745" --description "Related to tests and CI" --force

echo "Done. All labels created for $REPO."