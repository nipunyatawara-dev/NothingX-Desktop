$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
cargo build --release --locked
if ($LASTEXITCODE -ne 0) { throw 'Rust build failed' }
python scripts/package-source.py
if ($LASTEXITCODE -ne 0) { throw 'Source packaging failed' }
python scripts/package-windows.py target/release/nothingx-desktop.exe
if ($LASTEXITCODE -ne 0) { throw 'Windows packaging failed' }
Get-FileHash -Algorithm SHA256 dist/NothingX-Desktop-windows-x64.zip, dist/NothingX-Desktop-source.zip | Format-Table
