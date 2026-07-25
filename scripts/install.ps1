$ErrorActionPreference = "Stop"

$RepoUrl = if ($env:BRANP_REPO_URL) { $env:BRANP_REPO_URL } else { "https://github.com/BrandonPacewic/branp" }
$GitRef = if ($env:BRANP_INSTALL_REF) { $env:BRANP_INSTALL_REF } else { "" }

function Die($Message) {
    Write-Error $Message
    exit 1
}

function Need-Command($Name) {
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        Die "$Name is required but was not found in PATH"
    }
}

Need-Command git
Need-Command cargo
Need-Command rustc

try {
    rustc --version | Out-Null
} catch {
    Die "rustc is installed but could not run"
}

try {
    cargo --version | Out-Null
} catch {
    Die "cargo is installed but could not run"
}

$TempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("branp-install-" + [System.Guid]::NewGuid().ToString("N"))
$RepoPath = Join-Path $TempRoot "branp"

try {
    New-Item -ItemType Directory -Path $TempRoot | Out-Null

    Write-Host "Installing branp from source..."
    Write-Host "Repository: $RepoUrl"

    git clone --depth 1 $RepoUrl $RepoPath

    if ($GitRef) {
        Write-Host "Checking out ref: $GitRef"
        git -C $RepoPath fetch --depth 1 origin $GitRef
        git -C $RepoPath checkout FETCH_HEAD
    }

    cargo install --path $RepoPath --locked

    if ($env:CARGO_INSTALL_ROOT) {
        Write-Host "Installed bp to $(Join-Path $env:CARGO_INSTALL_ROOT "bin")"
    } elseif ($Bp = Get-Command bp -ErrorAction SilentlyContinue) {
        Write-Host "Installed bp to $($Bp.Source)"
    } else {
        $CargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $HOME ".cargo" }
        Write-Host "Installed bp to $(Join-Path $CargoHome "bin\bp.exe")"
    }

    Write-Host "Run 'bp --version' to verify the install."
} finally {
    if (Test-Path $TempRoot) {
        Remove-Item -Recurse -Force $TempRoot
    }
}
