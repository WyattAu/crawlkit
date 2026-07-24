#!/bin/bash
# crawlkit community launch script

set -euo pipefail

echo "============================================"
echo " crawlkit community launch"
echo "============================================"
echo ""

# 1. Create GitHub release
echo "Creating GitHub release..."
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
gh release create "v${VERSION}" \
    --title "crawlkit v${VERSION}" \
    --notes-file docs/FINAL_RELEASE_NOTES.md

# 2. Update documentation
echo "Updating documentation..."
cd web && npm run build && cd ..

# 3. Deploy documentation
echo "Deploying documentation..."
git add -A
git commit -m "docs: update for v${VERSION} release"
git push origin main

# 4. Announce on social media
echo ""
echo "Social media announcements:"
echo "- Twitter: Post about v${VERSION} release"
echo "- LinkedIn: Share release announcement"
echo "- Reddit: Post to r/rust and r/programming"
echo "- Hacker News: Submit release story"
echo ""

# 5. Send email newsletter
echo "Email newsletter:"
echo "- Send to subscriber list"
echo "- Include release highlights"
echo "- Include getting started guide"
echo ""

echo "============================================"
echo " Community launch complete!"
echo "============================================"
