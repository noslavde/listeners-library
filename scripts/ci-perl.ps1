# CI setup for the pinned Windows Perl distribution used by bundled OpenSSL.
# Verifies the downloaded archive before extraction and exposes the selected
# executable to later native build steps.

$ErrorActionPreference = 'Stop'
# Official Strawberry Perl manifest:
# https://raw.githubusercontent.com/StrawberryPerl/strawberryperl.com/gh-pages/releases.json
$archive = Join-Path $env:RUNNER_TEMP 'strawberry-perl-5.42.3.1.zip'
$destination = Join-Path $env:RUNNER_TEMP 'audible-native-perl'
Invoke-WebRequest -Uri 'https://github.com/StrawberryPerl/Perl-Dist-Strawberry/releases/download/SP_54231_64bit/strawberry-perl-5.42.3.1-64bit-portable.zip' -OutFile $archive
$expected = '6a081a811781c30aca51dbc036afd93092af91e3297901f02c17043795a10690'
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected) {
    throw 'Native Perl download failed checksum verification.'
}
Expand-Archive -LiteralPath $archive -DestinationPath $destination
$perl = Join-Path $destination 'perl/bin/perl.exe'
if (!(Test-Path -LiteralPath $perl)) { throw 'Verified native Perl executable was not found.' }
"OPENSSL_SRC_PERL=$perl" | Add-Content -LiteralPath $env:GITHUB_ENV
