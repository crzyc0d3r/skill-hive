param(
  [Parameter(Mandatory = $true, Position = 0)]
  [string]$File
)

$ErrorActionPreference = "Stop"
$required = @(
  "AZURE_TENANT_ID",
  "AZURE_CLIENT_ID",
  "AZURE_CLIENT_SECRET",
  "AZURE_TRUSTED_SIGNING_ENDPOINT",
  "AZURE_CODE_SIGNING_ACCOUNT_NAME",
  "AZURE_CERTIFICATE_PROFILE_NAME"
)
foreach ($name in $required) {
  if (-not (Get-Item -Path "Env:$name" -ErrorAction SilentlyContinue) -or [string]::IsNullOrWhiteSpace((Get-Item "Env:$name").Value)) {
    Write-Error "Windows signing secret $name is missing"
    exit 1
  }
}

$metadata = Join-Path $env:RUNNER_TEMP "skill-hive-signing.json"
@{
  Endpoint = $env:AZURE_TRUSTED_SIGNING_ENDPOINT
  CodeSigningAccountName = $env:AZURE_CODE_SIGNING_ACCOUNT_NAME
  CertificateProfileName = $env:AZURE_CERTIFICATE_PROFILE_NAME
} | ConvertTo-Json | Set-Content -Path $metadata

$dlib = Join-Path $env:RUNNER_TEMP "Azure.CodeSigning.Dlib.dll"
if (-not (Test-Path $dlib)) {
  Write-Error "Azure.CodeSigning.Dlib.dll is not installed on the runner. Place it in RUNNER_TEMP before signCommand runs."
  exit 1
}

& signtool.exe sign /fd SHA256 /tr http://timestamp.acs.microsoft.com /td SHA256 /dlib $dlib /dmdf $metadata $File
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host "signed $File with Azure Trusted Signing"
