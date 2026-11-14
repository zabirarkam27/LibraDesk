$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
Set-Location -LiteralPath $workspace
$config = Get-Content -LiteralPath 'src-tauri/tauri.conf.json' -Raw | ConvertFrom-Json
$version = $config.version
$tag = 'v' + $version
$repo = 'zabirarkam27/LibraDesk'
$json = gh api ("repos/" + $repo + "/releases/tags/" + $tag)
if ($LASTEXITCODE -ne 0) { throw 'Release API failed' }
$release = $json | ConvertFrom-Json
if ($release.draft -or $release.prerelease) { throw 'Expected public non-prerelease release' }
$filename = 'LibraDesk_' + $version + '_x64-setup.exe'
$asset = @($release.assets | Where-Object name -eq $filename)
$checksums = @($release.assets | Where-Object name -eq 'SHA256SUMS.txt')
if ($asset.Count -ne 1 -or $checksums.Count -ne 1) { throw 'Missing release assets' }
$setup = Join-Path $workspace ('src-tauri/target/release/bundle/nsis/' + $filename)
$hash = (Get-FileHash -LiteralPath $setup -Algorithm SHA256).Hash.ToLowerInvariant()
if ($asset[0].digest -ne ('sha256:' + $hash) -or $asset[0].size -ne (Get-Item -LiteralPath $setup).Length) { throw 'Published installer differs from audited local installer' }
$response = Invoke-WebRequest -Uri $asset[0].browser_download_url -Method Head -UseBasicParsing
if ($response.StatusCode -ne 200 -or [long](@($response.Headers['Content-Length'])[0]) -ne $asset[0].size) { throw 'Public setup download unavailable or wrong size' }
$checksumResponse = Invoke-WebRequest -Uri $checksums[0].browser_download_url -UseBasicParsing
$content = if ($checksumResponse.Content -is [byte[]]) { [Text.Encoding]::UTF8.GetString($checksumResponse.Content) } else { [string]$checksumResponse.Content }
if ($content.Trim() -ne ($hash + '  ' + $filename)) { throw 'Published checksum text mismatch' }
$latest = gh api ("repos/" + $repo + "/releases/latest") --jq .tag_name
if ($LASTEXITCODE -ne 0 -or $latest -ne $tag) { throw 'Latest release does not match' }
$readme = Invoke-WebRequest -Uri ("https://raw.githubusercontent.com/" + $repo + "/main/README.md") -UseBasicParsing
if ([string]$readme.Content -notmatch [regex]::Escape($filename)) { throw 'Public README lacks setup link' }
$report = [ordered]@{version=$version;tag=$tag;releaseUrl=$release.html_url;sourceCommit=$release.target_commitish;draft=$false;prerelease=$false;latest=$true;installerUrl=$asset[0].browser_download_url;installerBytes=$asset[0].size;sha256=$hash;githubAssetDigestMatches=$true;publicInstallerHttpStatus=[int]$response.StatusCode;publicChecksumHttpStatus=[int]$checksumResponse.StatusCode;checksumTextMatches=$true;publicReadmeVerified=$true}
$report | ConvertTo-Json | Set-Content -LiteralPath ('docs/qa/published-' + $version + '.json') -Encoding utf8
$report | ConvertTo-Json
