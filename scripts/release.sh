#!/usr/bin/env bash
set -euo pipefail

# ProGit Release Script
# Handles the full release workflow: develop → main with tagging
# Usage: ./scripts/release.sh [--dry-run]

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

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
if [[ "$CURRENT_BRANCH" != "develop" ]]; then
    echo -e "${RED}❌ Must be on 'develop' branch (currently on '${CURRENT_BRANCH}')${NC}"
    exit 1
fi

# Check for uncommitted changes
if [[ -n $(git status --porcelain) ]]; then
    echo -e "${RED}❌ Working directory not clean. Commit or stash changes first.${NC}"
    git status --short
    exit 1
fi

# Get current version from Cargo.toml
VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
if [[ -z "$VERSION" ]]; then
    echo -e "${RED}❌ Could not extract version from Cargo.toml${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Pre-flight checks passed${NC}"
echo -e "${BLUE}📦 Preparing release v${VERSION}${NC}"

# Run tests
echo -e "${BLUE}🧪 Running tests...${NC}"
if ! cargo test --quiet; then
    echo -e "${RED}❌ Tests failed. Fix issues before releasing.${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Tests passed${NC}"

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
    echo -e "  1. Checkout main"
    echo -e "  2. Merge develop → main"
    echo -e "  3. Tag v${VERSION}"
    echo -e "  4. Push main and tags"
    echo -e "  5. Return to develop"
    exit 0
fi

# Confirmation prompt
echo ""
echo -e "${YELLOW}⚠️  Ready to release v${VERSION}${NC}"
echo -e "${YELLOW}This will:${NC}"
echo -e "  1. Merge develop → main"
echo -e "  2. Create tag v${VERSION}"
echo -e "  3. Push to remote"
echo ""
read -p "Continue? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}❌ Release cancelled${NC}"
    exit 0
fi

# Perform release
echo -e "${BLUE}🚀 Starting release process...${NC}"

# Checkout main
echo -e "${BLUE}📝 Switching to main branch...${NC}"
git checkout main

# Merge develop into main
echo -e "${BLUE}📝 Merging develop → main...${NC}"
if ! git merge develop --no-ff -m "chore: release v${VERSION}"; then
    echo -e "${RED}❌ Merge failed. Resolve conflicts and try again.${NC}"
    git checkout develop
    exit 1
fi

# Create annotated tag
echo -e "${BLUE}📝 Creating tag v${VERSION}...${NC}"
git tag -a "v${VERSION}" -m "Release v${VERSION}"

# Push to remote
echo -e "${BLUE}📝 Pushing to remote...${NC}"
git push origin main
git push origin "v${VERSION}"

# Return to develop
echo -e "${BLUE}📝 Returning to develop branch...${NC}"
git checkout develop

# Merge main back to develop to keep them in sync
echo -e "${BLUE}📝 Syncing develop with main...${NC}"
git merge main --no-ff -m "chore: sync develop with main after v${VERSION} release"

echo ""
echo -e "${GREEN}🎉 Release v${VERSION} completed successfully!${NC}"
echo -e "${BLUE}📦 Tag: v${VERSION}${NC}"
echo -e "${BLUE}🌿 Branch: main${NC}"
echo ""
echo -e "${YELLOW}💡 Next steps:${NC}"
echo -e "  - Create GitHub/GitLab release notes"
echo -e "  - Build and publish binaries if needed"
echo -e "  - Update AUR package if applicable"
