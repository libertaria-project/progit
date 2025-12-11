#!/usr/bin/env bash
set -euo pipefail

# ProGit Version Bump Script
# Analyzes conventional commits and bumps version accordingly
# Usage: ./scripts/bump-version.sh [--dry-run]

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

DRY_RUN=false
if [[ "${1:-}" == "--dry-run" ]]; then
    DRY_RUN=true
    echo -e "${YELLOW}🔍 DRY RUN MODE${NC}"
fi

# Get the last tag, or use 0.1.0 if no tags exist
LAST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "v0.1.0")
echo -e "${BLUE}📌 Last tag: ${LAST_TAG}${NC}"

# Remove 'v' prefix if present
LAST_VERSION="${LAST_TAG#v}"

# Parse version components
IFS='.' read -r MAJOR MINOR PATCH <<< "$LAST_VERSION"

# Get commits since last tag
COMMITS=$(git log "${LAST_TAG}..HEAD" --pretty=format:"%s" 2>/dev/null || git log --pretty=format:"%s")

if [[ -z "$COMMITS" ]]; then
    echo -e "${YELLOW}⚠️  No new commits since ${LAST_TAG}${NC}"
    exit 0
fi

echo -e "${BLUE}📝 Analyzing commits since ${LAST_TAG}...${NC}"

# Determine version bump type
BUMP_TYPE="none"
HAS_FEAT=false
HAS_FIX=false
HAS_BREAKING=false

while IFS= read -r commit; do
    echo "  - $commit"
    
    if [[ "$commit" =~ ^feat(\(.+\))?!:|BREAKING[[:space:]]CHANGE ]]; then
        HAS_BREAKING=true
    elif [[ "$commit" =~ ^feat(\(.+\))?: ]]; then
        HAS_FEAT=true
    elif [[ "$commit" =~ ^fix(\(.+\))?: ]]; then
        HAS_FIX=true
    fi
done <<< "$COMMITS"

# Determine bump type (pre-1.0: breaking changes bump MINOR)
if [[ "$HAS_BREAKING" == true ]] || [[ "$HAS_FEAT" == true ]]; then
    BUMP_TYPE="minor"
    NEW_MINOR=$((MINOR + 1))
    NEW_PATCH=0
    NEW_VERSION="${MAJOR}.${NEW_MINOR}.${NEW_PATCH}"
elif [[ "$HAS_FIX" == true ]]; then
    BUMP_TYPE="patch"
    NEW_PATCH=$((PATCH + 1))
    NEW_VERSION="${MAJOR}.${MINOR}.${NEW_PATCH}"
else
    echo -e "${YELLOW}⚠️  No version-bumping commits found (feat/fix)${NC}"
    exit 0
fi

echo ""
echo -e "${GREEN}🚀 Version bump: ${LAST_VERSION} → ${NEW_VERSION} (${BUMP_TYPE})${NC}"

if [[ "$DRY_RUN" == true ]]; then
    echo -e "${YELLOW}🔍 Dry run - no changes made${NC}"
    exit 0
fi

# Update Cargo.toml
echo -e "${BLUE}📝 Updating Cargo.toml...${NC}"
sed -i "s/^version = \".*\"/version = \"${NEW_VERSION}\"/" Cargo.toml

# Verify the change
if grep -q "version = \"${NEW_VERSION}\"" Cargo.toml; then
    echo -e "${GREEN}✅ Cargo.toml updated successfully${NC}"
else
    echo -e "${RED}❌ Failed to update Cargo.toml${NC}"
    exit 1
fi

# Create commit
echo -e "${BLUE}📝 Creating version bump commit...${NC}"
git add Cargo.toml
git commit -m "chore: bump version to ${NEW_VERSION}"

echo -e "${GREEN}✅ Version bumped to ${NEW_VERSION}${NC}"
echo -e "${YELLOW}💡 Run './scripts/release.sh' to create a release${NC}"
