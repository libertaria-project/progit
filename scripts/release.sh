#!/usr/bin/env bash
set -euo pipefail

# ProGit Release Script
# Handles the full release workflow: main → stable with tagging
# Usage: ./scripts/release.sh [--dry-run] [--yes|-y]
#   --yes / -y : skip the interactive confirmation (for non-interactive CI)

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

INTEGRATION_BRANCH="main"
RELEASE_BRANCH="stable"
AUR_REPO_URL="ssh://aur@aur.archlinux.org/progit-bin.git"
AUR_REPO_DIR="${AUR_REPO_DIR:-$HOME/.cache/progit-aur/progit-bin}"
AUR_REPO_BRANCH="${AUR_REPO_BRANCH:-master}"
SKIP_AUR_UPDATE="${SKIP_AUR_UPDATE:-false}"
AUR_UPDATE_STRICT="${AUR_UPDATE_STRICT:-false}"
AUR_ONLY_TAGGED_RELEASE="${AUR_ONLY_TAGGED_RELEASE:-true}"

# Path helpers
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"

DRY_RUN=false
ASSUME_YES=false
for arg in "$@"; do
    case "$arg" in
        --dry-run) DRY_RUN=true; echo -e "${YELLOW}🔍 DRY RUN MODE${NC}" ;;
        -y|--yes)  ASSUME_YES=true ;;
    esac
done

# confirm "prompt" — success if the user agrees or --yes/-y was passed.
confirm() {
    [[ "$ASSUME_YES" == true ]] && return 0
    read -p "$1 (y/N) " -n 1 -r
    echo
    [[ $REPLY =~ ^[Yy]$ ]]
}

is_true() {
    case "${1,,}" in
        1|true|yes|on)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

is_tagged_release() {
    local tag="$1"
    git describe --tags --exact-match "$tag" >/dev/null 2>&1
}

update_aur_package() {
    if is_true "$SKIP_AUR_UPDATE"; then
        echo -e "${YELLOW}⏭️  SKIP_AUR_UPDATE=true, skipping progit-bin update.${NC}"
        return 0
    fi

    if ! command -v makepkg >/dev/null 2>&1; then
        echo -e "${RED}❌ makepkg not found, cannot update AUR package.${NC}"
        if is_true "$AUR_UPDATE_STRICT"; then
            return 1
        fi
        echo -e "${YELLOW}⚠️  Continuing without AUR publish.${NC}"
        return 0
    fi

    local pkgver_aur
    pkgver_aur="${VERSION//-/_}"
    echo -e "${BLUE}📦 Syncing AUR package from ${AUR_REPO_URL}...${NC}"

    if [[ -d "${AUR_REPO_DIR}/.git" ]]; then
        if ! git -C "$AUR_REPO_DIR" fetch origin "$AUR_REPO_BRANCH"; then
            echo -e "${RED}❌ Failed to fetch AUR branch ${AUR_REPO_BRANCH}.${NC}"
            if is_true "$AUR_UPDATE_STRICT"; then
                return 1
            fi
            echo -e "${YELLOW}⚠️  Continuing without AUR publish.${NC}"
            return 0
        fi

        if ! git -C "$AUR_REPO_DIR" checkout "$AUR_REPO_BRANCH"; then
            if git -C "$AUR_REPO_DIR" rev-parse --verify "origin/$AUR_REPO_BRANCH" >/dev/null 2>&1; then
                git -C "$AUR_REPO_DIR" checkout -b "$AUR_REPO_BRANCH" "origin/$AUR_REPO_BRANCH"
            else
                git -C "$AUR_REPO_DIR" checkout -B "$AUR_REPO_BRANCH"
            fi
        fi

        if git -C "$AUR_REPO_DIR" rev-parse --verify "origin/$AUR_REPO_BRANCH" >/dev/null 2>&1; then
            git -C "$AUR_REPO_DIR" reset --hard "origin/$AUR_REPO_BRANCH"
        fi
    elif [[ -d "$AUR_REPO_DIR" ]]; then
        rm -rf "$AUR_REPO_DIR"
    fi

    if [[ ! -d "$AUR_REPO_DIR/.git" ]]; then
        mkdir -p "$(dirname "$AUR_REPO_DIR")"
        if ! git clone "$AUR_REPO_URL" "$AUR_REPO_DIR"; then
            echo -e "${RED}❌ Failed to clone AUR repository.${NC}"
            if is_true "$AUR_UPDATE_STRICT"; then
                return 1
            fi
            echo -e "${YELLOW}⚠️  Continuing without AUR publish.${NC}"
            return 0
        fi
    fi

    local aur_template
    if [[ -f "$REPO_ROOT/aur/PKGBUILD" ]]; then
        aur_template="$REPO_ROOT/aur/PKGBUILD"
    elif [[ -f "$REPO_ROOT/../aur/PKGBUILD" ]]; then
        aur_template="$REPO_ROOT/../aur/PKGBUILD"
    else
        echo -e "${RED}❌ Missing PKGBUILD source; expected it in progit/aur or ../aur.${NC}"
        if is_true "$AUR_UPDATE_STRICT"; then
            return 1
        fi
        echo -e "${YELLOW}⚠️  Continuing without AUR publish.${NC}"
        return 0
    fi

    cp "$aur_template" "$AUR_REPO_DIR/PKGBUILD"
    sed -i "s/^pkgver=.*/pkgver=${pkgver_aur}/" "$AUR_REPO_DIR/PKGBUILD"
    makepkg --printsrcinfo -p "$AUR_REPO_DIR/PKGBUILD" > "$AUR_REPO_DIR/.SRCINFO"

    if git -C "$AUR_REPO_DIR" diff --quiet -- PKGBUILD .SRCINFO; then
        echo -e "${GREEN}✅ AUR recipe already up to date; no publish needed.${NC}"
        return 0
    fi

    git -C "$AUR_REPO_DIR" add PKGBUILD .SRCINFO
    git -C "$AUR_REPO_DIR" commit -m "Update progit-bin for v${VERSION}"
    if ! git -C "$AUR_REPO_DIR" push origin "$AUR_REPO_BRANCH"; then
        echo -e "${RED}❌ Failed to push AUR package recipe.${NC}"
        if is_true "$AUR_UPDATE_STRICT"; then
            return 1
        fi
        echo -e "${YELLOW}⚠️  AUR publish failed, release still published from Forgejo.${NC}"
        return 0
    fi

    echo -e "${GREEN}✅ Published progit-bin to AUR (v${VERSION}).${NC}"
    return 0
}

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
    echo -e "  6. Link target/release/prog to ~/bin/prog"
    echo -e "  7. Update progit-bin AUR package (unless SKIP_AUR_UPDATE=true)"
    exit 0
fi

# Confirmation prompt
echo ""
echo -e "${YELLOW}⚠️  Ready to release v${VERSION}${NC}"
echo -e "${YELLOW}This will:${NC}"
echo -e "  1. Merge ${INTEGRATION_BRANCH} → ${RELEASE_BRANCH}"
echo -e "  2. Create tag v${VERSION} on ${RELEASE_BRANCH}"
echo -e "  3. Push ${INTEGRATION_BRANCH}, ${RELEASE_BRANCH}, and tag"
echo -e "  4. Link target/release/prog to ~/bin/prog"
echo ""
if ! confirm "Continue?"; then
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

# Only publish to AUR for actual tagged releases unless explicitly overridden.
if is_true "$AUR_ONLY_TAGGED_RELEASE" && ! is_tagged_release "v${VERSION}"; then
    echo -e "${YELLOW}⏭️  Skipping AUR publish because HEAD does not point at v${VERSION} tag."
    echo -e "   Set AUR_ONLY_TAGGED_RELEASE=false to force update on non-tagged runs.${NC}"
else
    echo -e "${BLUE}📦 Syncing progit-bin AUR package...${NC}"
    if ! update_aur_package; then
        echo -e "${RED}❌ Release script cannot continue because AUR publish failed.${NC}"
        exit 1
    fi
fi

# Return to main
echo -e "${BLUE}📝 Returning to ${INTEGRATION_BRANCH} branch...${NC}"
git checkout "${INTEGRATION_BRANCH}"

echo -e "${BLUE}🔗 Linking release binary to ~/bin/prog...${NC}"
bash ./scripts/link-user-bin.sh target/release/prog

echo ""
echo -e "${GREEN}🎉 Release v${VERSION} completed successfully!${NC}"
echo -e "${BLUE}📦 Tag: v${VERSION}${NC}"
echo -e "${BLUE}🌿 Branch: ${RELEASE_BRANCH}${NC}"
echo ""
echo -e "${YELLOW}💡 Next steps:${NC}"
echo -e "  - Create GitHub/GitLab release notes"
echo -e "  - Build and publish binaries if needed"
