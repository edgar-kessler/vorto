# Installs Vorto and the `vorto` command for this Windows user:
#   irm https://edgar-kessler.github.io/vorto/install.ps1 | iex
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$url = "https://github.com/edgar-kessler/vorto/releases/latest/download/Vorto-Setup.exe"
$setup = Join-Path $env:TEMP "Vorto-Setup-$PID.exe"
Write-Host "Downloading Vorto..."
Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $setup
try {
  Write-Host "Installing..."
  $run = Start-Process -FilePath $setup -ArgumentList "/S" -Wait -PassThru
  if ($run.ExitCode -ne 0) { throw "The installer stopped with code $($run.ExitCode)." }
} finally {
  Remove-Item $setup -ErrorAction SilentlyContinue
}

$dir = Join-Path $env:LOCALAPPDATA "Vorto"
$path = [Environment]::GetEnvironmentVariable("Path", "User")
if (($path -split ";") -notcontains $dir) {
  [Environment]::SetEnvironmentVariable("Path", ($path.TrimEnd(";") + ";" + $dir).TrimStart(";"), "User")
}
$env:Path += ";$dir"

Write-Host ""
Write-Host "Vorto is installed. Open a new terminal, then:"
Write-Host "  vorto models download parakeet-de   a voice model (see: vorto models)"
Write-Host "  vorto start                         only the recording pill"
Write-Host "  vorto start --window                with its window"
