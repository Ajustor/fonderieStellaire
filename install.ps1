# Installe la dernière version de Fonderie Stellaire (Windows).
#   irm https://raw.githubusercontent.com/Ajustor/fonderieStellaire/master/install.ps1 | iex
# Variables d'environnement facultatives : FONDERIE_VERSION (ex. v0.1.0), FONDERIE_DIR.
$ErrorActionPreference = 'Stop'

$Repo = 'Ajustor/fonderieStellaire'
$Bin = 'fonderie-stellaire.exe'
$Asset = 'fonderie-stellaire-x86_64-pc-windows-msvc.zip'
$Version = if ($env:FONDERIE_VERSION) { $env:FONDERIE_VERSION } else { 'latest' }
$InstallDir = if ($env:FONDERIE_DIR) { $env:FONDERIE_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\fonderie-stellaire' }

function Say($msg) { Write-Host "✦ $msg" -ForegroundColor Yellow }

if (-not [Environment]::Is64BitOperatingSystem) {
    throw 'Windows 64 bits requis.'
}

$Base = if ($Version -eq 'latest') {
    "https://github.com/$Repo/releases/latest/download"
} else {
    "https://github.com/$Repo/releases/download/$Version"
}

$Tmp = Join-Path ([IO.Path]::GetTempPath()) ("fonderie-" + [Guid]::NewGuid())
New-Item -ItemType Directory -Path $Tmp | Out-Null
try {
    Say "Téléchargement de $Asset…"
    $Zip = Join-Path $Tmp $Asset
    Invoke-WebRequest -UseBasicParsing -Uri "$Base/$Asset" -OutFile $Zip
    Invoke-WebRequest -UseBasicParsing -Uri "$Base/$Asset.sha256" -OutFile "$Zip.sha256"

    Say 'Vérification de l''intégrité…'
    $Expected = ((Get-Content "$Zip.sha256" -Raw).Trim() -split '\s+')[0].ToLower()
    $Actual = (Get-FileHash $Zip -Algorithm SHA256).Hash.ToLower()
    if ($Expected -ne $Actual) { throw 'Somme SHA-256 incorrecte.' }

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Expand-Archive -Path $Zip -DestinationPath $Tmp -Force
    Copy-Item (Join-Path $Tmp $Bin) (Join-Path $InstallDir $Bin) -Force
    # Fichier téléchargé : on retire la marque « provenant d'Internet ».
    Unblock-File (Join-Path $InstallDir $Bin)
    Say "Installé dans $InstallDir"

    $UserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (-not (($UserPath -split ';') -contains $InstallDir)) {
        $NewPath = if ([string]::IsNullOrEmpty($UserPath)) { $InstallDir } else { "$UserPath;$InstallDir" }
        [Environment]::SetEnvironmentVariable('Path', $NewPath, 'User')
        $env:Path = "$env:Path;$InstallDir"
        Say 'Dossier ajouté à ton PATH (ouvre un nouveau terminal pour en profiter).'
    }
    Say 'Lance le jeu avec : fonderie-stellaire   (de préférence dans Windows Terminal)'
}
finally {
    Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
}
