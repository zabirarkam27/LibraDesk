param([switch]$IncludeReleaseSmoke)
$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
Set-Location -LiteralPath $workspace
$expectedVersion = (Get-Content -LiteralPath 'src-tauri/tauri.conf.json' -Raw | ConvertFrom-Json).version
$installer = (Resolve-Path -LiteralPath ("src-tauri/target/release/bundle/nsis/LibraDesk_" + $expectedVersion + "_x64-setup.exe")).Path
if (Test-Path -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\LibraDesk') { throw 'An existing LibraDesk installation must be reviewed before this isolated installer test.' }
$qa = Join-Path $workspace ('src-tauri\target\installer-qa-' + [guid]::NewGuid().ToString('N'))
$install = Join-Path $qa 'installed'
New-Item -ItemType Directory -Path $qa | Out-Null
$source = Join-Path $qa 'original.pdf'
[IO.File]::WriteAllText($source, "%PDF-1.7" + [Environment]::NewLine + "Installer preservation fixture")
$sourceHash = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash
$process = Start-Process -FilePath $installer -ArgumentList @('/S','/NS',("/D=" + $install)) -Wait -PassThru -WindowStyle Hidden
if ($process.ExitCode -ne 0) { throw "Installer returned $($process.ExitCode)" }
$exe = Join-Path $install 'libradesk.exe'
$uninstall = Join-Path $install 'uninstall.exe'
if (!(Test-Path -LiteralPath $exe) -or !(Test-Path -LiteralPath $uninstall)) { throw 'Installed executable or uninstaller missing' }
$version = (Get-Item -LiteralPath $exe).VersionInfo.ProductVersion
if ($version -ne $expectedVersion) { throw "Wrong installed version $version" }
node scripts/check-installed-binary.mjs $exe
if ($LASTEXITCODE -ne 0) { throw 'Installed binary differs from the expected NSIS release binary' }
$markerDir = Join-Path $install 'database'
New-Item -ItemType Directory -Path $markerDir | Out-Null
$marker = Join-Path $markerDir 'preserve-user-data.txt'
[IO.File]::WriteAllText($marker,'User data preservation fixture')
if ($IncludeReleaseSmoke) {
    node scripts/smoke-release.mjs $exe
    if ($LASTEXITCODE -ne 0) { throw 'Installed release smoke test failed' }
}
$process = Start-Process -FilePath $uninstall -ArgumentList '/S' -Wait -PassThru -WindowStyle Hidden
if ($process.ExitCode -ne 0) { throw "Uninstaller returned $($process.ExitCode)" }
for ($attempt = 0; $attempt -lt 50 -and (Test-Path -LiteralPath $exe); $attempt++) { Start-Sleep -Milliseconds 100 }
if (Test-Path -LiteralPath $exe) { throw 'Uninstaller retained application executable' }
if (!(Test-Path -LiteralPath $marker)) { throw 'Uninstaller removed user data fixture' }
if ((Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash -ne $sourceHash) { throw 'Original PDF changed' }
if (Test-Path -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\LibraDesk') { throw 'Uninstall registry entry retained' }
$report = [ordered]@{ version=$version;installExitCode=0;uninstallExitCode=0;installedBinaryMatches=$true;sourcePdfPreserved=$true;userDataSubfolderPreserved=$true;uninstallRegistryRemoved=$true;testDirectory=$qa;scope='Clean current-user install directory on existing Windows account; isolated native profile tested separately' }
$report | ConvertTo-Json | Set-Content -LiteralPath 'src-tauri/target/installer-qa.json' -Encoding utf8
$report | ConvertTo-Json
