$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
Set-Location -LiteralPath $workspace
$config = Get-Content -LiteralPath 'src-tauri/tauri.conf.json' -Raw | ConvertFrom-Json
$version = $config.version
if ($config.bundle.windows.webviewInstallMode.type -ne 'offlineInstaller') { throw 'Expected bundled standalone runtime' }
$setup = (Resolve-Path -LiteralPath ("src-tauri/target/release/bundle/nsis/LibraDesk_" + $version + "_x64-setup.exe")).Path
$nsi = Get-Content -LiteralPath 'src-tauri/target/release/nsis/x64/installer.nsi' -Raw
if ($nsi -notmatch '!define INSTALLWEBVIEW2MODE "offlineInstaller"') { throw 'Generated NSIS mode mismatch' }
$match = [regex]::Match($nsi, '!define WEBVIEW2INSTALLERPATH "([^"]+)"')
if (!$match.Success) { throw 'Missing embedded runtime source' }
$runtime = $match.Groups[1].Value
$signature = Get-AuthenticodeSignature -LiteralPath $runtime
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'CN=Microsoft Corporation,') { throw 'Runtime must have a valid Microsoft signature' }
$listing = (& 7z l -ba -slt $setup) -join [Environment]::NewLine
if ($LASTEXITCODE -ne 0) { throw 'NSIS archive listing failed' }
if ($listing -notmatch 'MicrosoftEdgeWebView2RuntimeInstaller\.exe') { throw 'Runtime absent from setup payload' }
if ($listing -notmatch 'libradesk\.exe') { throw 'App absent from setup payload' }
if ($listing -match '\.(sqlite3|pdf)(\s|$)') { throw 'Unexpected library/source document in distribution' }
$hash = (Get-FileHash -LiteralPath $setup -Algorithm SHA256).Hash.ToLowerInvariant()
$filename = [IO.Path]::GetFileName($setup)
[IO.File]::WriteAllText((Join-Path ([IO.Path]::GetDirectoryName($setup)) 'SHA256SUMS.txt'),$hash + '  ' + $filename + [Environment]::NewLine,[Text.UTF8Encoding]::new($false))
$report = [ordered]@{version=$version;filename=$filename;bytes=(Get-Item -LiteralPath $setup).Length;sha256=$hash;webviewInstallMode='offlineInstaller';embeddedRuntimeBytes=(Get-Item -LiteralPath $runtime).Length;embeddedRuntimeSignature='Valid';embeddedRuntimeSigner='Microsoft Corporation';payloadInspected=$true;runtimeAbsentExecutionTested=$false}
$report | ConvertTo-Json | Set-Content -LiteralPath 'src-tauri/target/distribution-qa.json' -Encoding utf8
$report | ConvertTo-Json
