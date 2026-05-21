#!/usr/bin/env bash
set -euo pipefail

# ProGit Release Script
# Handles the full release workflow: main → stable with tagging
# Usage: ./scripts/release.sh [--dry-run]

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

INTEGRATION_BRANCH="main"
RELEASE_BRANCH="stable"

DRY_RUN=false
if [[ "${1:-}" == "--dry-run" ]]; then
    DRY_RUN=true
    echo -e "${YELLOW}🔍 DRY RUN MODE${NC}"
fi

# Pre-flight checks
echo -e "${BLUE}🔍 Running pre-flight checks...${NC}"

# Check if we're in a git repo
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    echo -e "${RED}❌ Not in a git repository${NC}"
    exit 1
fi

# Check current branch
CURRENT_BRANCH=$(git branch --show-current)
if [[ "$CURRENT_BRANCH" != "$INTEGRATION_BRANCH" ]]; then
    echo -e "${RED}❌ Must be on '${INTEGRATION_BRANCH}' branch (currently on '${CURRENT_BRANCH}')${NC}"
    exit 1
fi

# Check release branch exists locally or on origin
if ! git show-ref --verify --quiet "refs/heads/${RELEASE_BRANCH}" \
    && ! git show-ref --verify --quiet "refs/remotes/origin/${RELEASE_BRANCH}"; then
    echo -e "${RED}❌ Release branch '${RELEASE_BRANCH}' not found locally or on origin${NC}"
    exit 1
fi

# Check for uncommitted changes
if [[ "$DRY_RUN" != true && -n $(git status --porcelain) ]]; then
    echo -e "${RED}❌ Working directory not clean. Commit or stash changes first.${NC}"
    git status --short
    exit 1
elif [[ "$DRY_RUN" == true && -n $(git status --porcelain) ]]; then
    echo -e "${YELLOW}⚠️  Dry run with uncommitted changes; real releases require a clean tree.${NC}"
fi

# Get current version from Cargo.toml
VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
if [[ -z "$VERSION" ]]; then
    echo -e "${RED}❌ Could not extract version from Cargo.toml${NC}"
    exit 1
fi

if git rev-parse -q --verify "refs/tags/v${VERSION}" > /dev/null; then
    if [[ "$DRY_RUN" == true ]]; then
        echo -e "${YELLOW}⚠️  Tag v${VERSION} already exists; real releases would stop here.${NC}"
    else
        echo -e "${RED}❌ Tag v${VERSION} already exists${NC}"
        exit 1
    fi
fi

echo -e "${GREEN}✅ Pre-flight checks passed${NC}"
echo -e "${BLUE}📦 Preparing release v${VERSION}${NC}"

# Run quality checks
echo -e "${BLUE}Running quality checks...${NC}"
if ! bash ./scripts/check.sh; then
    echo -e "${RED}Quality checks failed. Fix issues before releasing.${NC}"
    exit 1
fi
echo -e "${GREEN}Quality checks passed${NC}"

# Build release binary to verify
echo -e "${BLUE}🔨 Building release binary...${NC}"
if ! cargo build --release --quiet; then
    echo -e "${RED}❌ Release build failed${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Release build successful${NC}"

if [[ "$DRY_RUN" == true ]]; then
    echo -e "${YELLOW}🔍 Dry run complete - no changes made${NC}"
    echo -e "${BLUE}Would perform:${NC}"
    echo -e "  1. Checkout ${RELEASE_BRANCH}"
    echo -e "  2. Merge ${INTEGRATION_BRANCH} → ${RELEASE_BRANCH}"
    echo -e "  3. Tag v${VERSION} on ${RELEASE_BRANCH}"
    echo -e "  4. Push ${INTEGRATION_BRANCH}, ${RELEASE_BRANCH}, and tag"
    echo -e "  5. Return to ${INTEGRATION_BRANCH}"
    exit 0
fi

# Confirmation prompt
echo ""
echo -e "${YELLOW}⚠️  Ready to release v${VERSION}${NC}"
echo -e "${YELLOW}This will:${NC}"
echo -e "  1. Merge ${INTEGRATION_BRANCH} → ${RELEASE_BRANCH}"
echo -e "  2. Create tag v${VERSION} on ${RELEASE_BRANCH}"
echo -e "  3. Push ${INTEGRATION_BRANCH}, ${RELEASE_BRANCH}, and tag"
echo ""
read -p "Continue? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}❌ Release cancelled${NC}"
    exit 0
fi

# Perform release
echo -e "${BLUE}🚀 Starting release process...${NC}"

# Checkout stable
echo -e "${BLUE}📝 Switching to ${RELEASE_BRANCH} branch...${NC}"
if git show-ref --verify --quiet "refs/heads/${RELEASE_BRANCH}"; then
    git checkout "${RELEASE_BRANCH}"
else
    git checkout -b "${RELEASE_BRANCH}" "origin/${RELEASE_BRANCH}"
fi

# Merge main into stable
echo -e "${BLUE}📝 Merging ${INTEGRATION_BRANCH} → ${RELEASE_BRANCH}...${NC}"
if ! git merge "${INTEGRATION_BRANCH}" --no-ff -m "chore: release v${VERSION}"; then
    echo -e "${RED}❌ Merge failed. Resolve conflicts and try again.${NC}"
    git checkout "${INTEGRATION_BRANCH}"
    exit 1
fi

# Create annotated tag
echo -e "${BLUE}📝 Creating tag v${VERSION}...${NC}"
git tag -a "v${VERSION}" -m "Release v${VERSION}"

# Push to remote
echo -e "${BLUE}📝 Pushing to remote...${NC}"
git push origin "${INTEGRATION_BRANCH}"
git push origin "${RELEASE_BRANCH}"
git push origin "v${VERSION}"

# Return to main
echo -e "${BLUE}📝 Returning to ${INTEGRATION_BRANCH} branch...${NC}"
git checkout "${INTEGRATION_BRANCH}"

echo ""
echo -e "${GREEN}🎉 Release v${VERSION} completed successfully!${NC}"
echo -e "${BLUE}📦 Tag: v${VERSION}${NC}"
echo -e "${BLUE}🌿 Branch: ${RELEASE_BRANCH}${NC}"
echo ""
echo -e "${YELLOW}💡 Next steps:${NC}"
echo -e "  - Create GitHub/GitLab release notes"
echo -e "  - Build and publish binaries if needed"
echo -e "  - Update AUR package if applicable"
