#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────────────────────
# Catholic Compass – local SCA + SBOM runner
# Run from the repo root:  bash scripts/scan-local.sh
# ──────────────────────────────────────────────────────────────────────────────
set -euo pipefail

REPORT_DIR="security-reports"
mkdir -p "$REPORT_DIR"

echo "━━━ Catholic Compass Security Scan ━━━"
echo "Reports will be written to: $REPORT_DIR/"
echo

# ── 1. cargo-audit (Rust advisory DB) ────────────────────────────────────────
echo "[1/6] cargo-audit …"
if ! command -v cargo-audit &>/dev/null; then
  cargo install cargo-audit --locked
fi
(cd backend && cargo audit --json > "../$REPORT_DIR/rust-audit.json" 2>&1) || true
echo "      → $REPORT_DIR/rust-audit.json"

# ── 2. cargo-deny (licences + advisories) ────────────────────────────────────
echo "[2/6] cargo-deny …"
if ! command -v cargo-deny &>/dev/null; then
  cargo install cargo-deny --locked
fi
(cd backend && cargo deny check 2>&1 | tee "../$REPORT_DIR/cargo-deny.txt") || true
echo "      → $REPORT_DIR/cargo-deny.txt"

# ── 3. npm audit (JS deps) ───────────────────────────────────────────────────
echo "[3/6] npm audit …"
(cd frontend && npm audit --json > "../$REPORT_DIR/npm-audit.json" 2>&1) || true
echo "      → $REPORT_DIR/npm-audit.json"

# ── 4. syft – generate SBOMs ─────────────────────────────────────────────────
echo "[4/6] syft SBOM generation …"
if ! command -v syft &>/dev/null; then
  echo "      syft not found. Install: https://github.com/anchore/syft#installation"
  echo "      Skipping SBOM generation."
else
  syft backend/ -o cyclonedx-json="$REPORT_DIR/backend-sbom.cyclonedx.json"
  syft frontend/ -o cyclonedx-json="$REPORT_DIR/frontend-sbom.cyclonedx.json"
  echo "      → $REPORT_DIR/backend-sbom.cyclonedx.json"
  echo "      → $REPORT_DIR/frontend-sbom.cyclonedx.json"
fi

# ── 5. grype – vulnerability scan against SBOMs ──────────────────────────────
echo "[5/6] grype vulnerability scan …"
if ! command -v grype &>/dev/null; then
  echo "      grype not found. Install: https://github.com/anchore/grype#installation"
  echo "      Skipping grype scan."
else
  if [ -f "$REPORT_DIR/backend-sbom.cyclonedx.json" ]; then
    grype "sbom:$REPORT_DIR/backend-sbom.cyclonedx.json" \
      --fail-on high --output json \
      > "$REPORT_DIR/grype-rust.json" 2>&1 || true
    echo "      → $REPORT_DIR/grype-rust.json"
  fi
  if [ -f "$REPORT_DIR/frontend-sbom.cyclonedx.json" ]; then
    grype "sbom:$REPORT_DIR/frontend-sbom.cyclonedx.json" \
      --fail-on high --output json \
      > "$REPORT_DIR/grype-js.json" 2>&1 || true
    echo "      → $REPORT_DIR/grype-js.json"
  fi
fi

# ── 6. Summary ───────────────────────────────────────────────────────────────
echo
echo "[6/6] Summary"
echo "      Files in $REPORT_DIR/:"
ls -lh "$REPORT_DIR/" | tail -n +2

echo
echo "━━━ Scan complete ━━━"
echo "Open the JSON reports or pipe them through:"
echo "  jq '.vulnerabilities[] | select(.severity==\"High\" or .severity==\"Critical\")' \\"
echo "     $REPORT_DIR/grype-rust.json"