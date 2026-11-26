<#
.SYNOPSIS
  Coletor continuo e passivo do boot da sonda PicoFW (BOOTFOLLOW): terminal aberto ate Ctrl+C.

.DESCRIPTION
  Requer o firmware com o comando BOOTFOLLOW (regravar a Pico apos o build).
  - Passivo: nunca pulsa XRESET nem puxa SW_UART/NSW5V (o aparelho nao e afetado).
  - Sem limite de 90 s: a sessao PIO fica aberta e o buffer DMA da sonda nao perde bytes.
  - Ciclos automaticos: XRESET real, wake (NSW5V sobe) ou banner "DDR CHECK: OK" zeram o
    contador (+0 ms); NSW5V caindo (standby) encerra o ciclo. Idempotente: anexar no meio de
    um boot nao importa, o relogio recomeca no proximo evento real.
  - Ctrl+C aqui envia 0x03 a sonda, que fecha o ciclo aberto, imprime o resumo e libera a serial.

.EXAMPLE
  .\scripts\run_bootfollow.ps1
  .\scripts\run_bootfollow.ps1 -Port COM10 -IdleMs 15000
#>
param(
    [string]$Port = "",
    [ValidateRange(1000, 120000)][int]$IdleMs = 8000   # silencio da UART que dispara o relatorio do ciclo
)

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$LogRoot  = Join-Path $RepoRoot "logs\boottrace"
New-Item -ItemType Directory -Force -Path $LogRoot | Out-Null

# --- Descoberta da porta (VID 2E8A) --------------------------------------
if (-not $Port) {
    $ExeCandidates = @(
        $env:PANADIGA_CLI_BIN,
        (Join-Path $RepoRoot "dist\panadiga.exe"),
        (Join-Path $RepoRoot "target\debug\panadiga.exe"),
        (Join-Path $RepoRoot "target\release\panadiga.exe")
    ) | Where-Object { $_ -and (Test-Path $_) }
    if ($ExeCandidates) {
        $m = [regex]::Match((& $ExeCandidates[0] pico list | Out-String), "Port:\s*(\S+)\s*\|[^\r\n]*VID:2E8A")
        if ($m.Success) { $Port = $m.Groups[1].Value }
    }
    if (-not $Port) { $Port = "COM10" }
}

$LogPath = Join-Path $LogRoot ("bootfollow_{0}.log" -f (Get-Date -Format "yyyyMMdd_HHmmss"))
Write-Host "Porta: $Port | idle_report: $IdleMs ms | Log: $LogPath"
Write-Host "Ctrl+C encerra (envia 0x03 a sonda e aguarda o resumo).`n"

$sp = New-Object System.IO.Ports.SerialPort $Port, 115200, ([System.IO.Ports.Parity]::None), 8, ([System.IO.Ports.StopBits]::One)
$sp.DtrEnable   = $true
$sp.ReadTimeout = 200
$writer = $null
$prevCtrlC = [Console]::TreatControlCAsInput
try {
    $sp.Open()
    [Console]::TreatControlCAsInput = $true
    $writer = New-Object System.IO.StreamWriter($LogPath, $true, [System.Text.Encoding]::UTF8)
    $writer.AutoFlush = $true

    $sp.DiscardInBuffer()
    $sp.Write("BOOTFOLLOW $IdleMs`r")

    $buf = ""
    $stopping = $false
    $stopDeadline = [DateTime]::MaxValue
    $finished = $false

    while (-not $finished) {
        # Ctrl+C -> 0x03 para a sonda (uma unica vez)
        while ([Console]::KeyAvailable) {
            $k = [Console]::ReadKey($true)
            if (($k.Modifiers -band [ConsoleModifiers]::Control) -and $k.Key -eq [ConsoleKey]::C -and -not $stopping) {
                $stopping = $true
                $sp.Write([byte[]](0x03), 0, 1)
                $stopDeadline = [DateTime]::UtcNow.AddSeconds(8)
                Write-Host "`n[ctrl+c] encerrando BOOTFOLLOW..." -ForegroundColor Yellow
            }
        }
        try { $buf += $sp.ReadExisting() } catch { }
        while (($i = $buf.IndexOfAny([char[]]"`r`n")) -ge 0) {
            $line = $buf.Substring(0, $i)
            $buf = $buf.Substring($i + 1)
            if ($line.Length -eq 0) { continue }
            $color = if ($line -like "``[FOLLOW``]*") { "Cyan" } else { $null }
            if ($color) { Write-Host $line -ForegroundColor $color } else { Write-Host $line }
            $writer.WriteLine($line)
            if ($line -match "^BOOTFOLLOW:\s*OK") { $finished = $true }
            if ($line -match "^BOOTFOLLOW:\s*ERROR|unknown command|ERR\s+ENOCMD") {
                Write-Host "A sonda nao reconhece BOOTFOLLOW: regrave o firmware novo (UF2)." -ForegroundColor Red
                $finished = $true
            }
        }
        if ($stopping -and [DateTime]::UtcNow -gt $stopDeadline) {
            Write-Host "[AVISO] sem confirmacao da sonda; encerrando localmente." -ForegroundColor Yellow
            break
        }
        Start-Sleep -Milliseconds 10
    }
}
finally {
    [Console]::TreatControlCAsInput = $prevCtrlC
    if ($writer) { $writer.Dispose() }
    if ($sp.IsOpen) { $sp.Close() }
    Write-Host "`nEncerrado. Log: $LogPath"
}
