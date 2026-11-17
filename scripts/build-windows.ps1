$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
cargo build --release --locked --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { throw 'Rust build failed' }
python scripts/package-source.py
if ($LASTEXITCODE -ne 0) { throw 'Source packaging failed' }
python scripts/package-windows.py target/x86_64-pc-windows-msvc/release/nothingx-desktop.exe --installer
if ($LASTEXITCODE -ne 0) { throw 'Windows packaging failed' }
Get-FileHash -Algorithm SHA256 dist/*windows-x64*.exe, dist/*windows-x64*.zip | Format-Table
