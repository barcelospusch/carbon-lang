$ErrorActionPreference = 'Stop'
if (-not [Environment]::Is64BitOperatingSystem) { throw 'Este pacote exige Windows x64.' }
$projectRoot = Split-Path -Parent $PSScriptRoot
$engineDirectory = Join-Path $projectRoot 'tools/tectonic'
New-Item -ItemType Directory -Force -Path $engineDirectory | Out-Null
$archivePath = Join-Path $engineDirectory 'tectonic.zip'
$downloadUrl = 'https://github.com/tectonic-typesetting/tectonic/releases/download/tectonic%400.17.0/tectonic-0.17.0-x86_64-pc-windows-msvc.zip'
Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath
$expectedSha256 = 'F61CE51F0B0ADE1015B7DE7EF368541C5424E9756ECBD0D7AF97D6D48030845F'
if ((Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash -ne $expectedSha256) {
    throw 'Checksum do pacote Tectonic não confere; instalação interrompida.'
}
Expand-Archive -LiteralPath $archivePath -DestinationPath $engineDirectory -Force
& (Join-Path $engineDirectory 'tectonic.exe') --version
if ($LASTEXITCODE -ne 0) { throw 'Não foi possível executar o Tectonic.' }
