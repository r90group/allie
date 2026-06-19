#!/bin/sh
set -eu

cargo fmt --check
cargo test --locked
npm run worker:smoke
npm run evidence:smoke
npm run release:smoke

test -f .allie/runs/v0-smoke/evidence.json
test -f .allie/runs/v0-smoke/report.html
test -f .allie/releases/v0-smoke/release-summary.json
test -f .allie/releases/v0-smoke/github-check.json
test -f .allie/releases/v0-smoke/release-report.html
