#!/bin/sh
set -eu
project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
npm ci --prefix "$project_root/tools/mermaid" --no-audit --no-fund
node "$project_root/tools/mermaid/node_modules/puppeteer/install.mjs"
node "$project_root/tools/mermaid/node_modules/@mermaid-js/mermaid-cli/src/cli.js" --version
