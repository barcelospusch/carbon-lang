$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$runtimeDirectory = Join-Path $projectRoot 'tools/mermaid'
& npm.cmd ci --prefix $runtimeDirectory --no-audit --no-fund
if ($LASTEXITCODE -ne 0) { throw 'Falha ao instalar o Mermaid CLI.' }
# Explicit also for npm versions that block dependency lifecycle scripts.
& node (Join-Path $runtimeDirectory 'node_modules/puppeteer/install.mjs')
if ($LASTEXITCODE -ne 0) { throw 'Falha ao instalar o Chromium do Puppeteer.' }
& node (Join-Path $runtimeDirectory 'node_modules/@mermaid-js/mermaid-cli/src/cli.js') --version
if ($LASTEXITCODE -ne 0) { throw 'Não foi possível executar o Mermaid CLI.' }
