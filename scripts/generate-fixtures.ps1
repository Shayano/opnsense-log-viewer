# generate-fixtures.ps1
# Generates 30GB of synthetic OPNsense firewall logs for testing
# - 10GB RFC3164 format
# - 10GB RFC5424 format
# - 10GB CSV filterlog format

param(
    [int]$SizeMB = 10240  # Default: 10GB per format (30GB total)
)

$ErrorActionPreference = "Stop"

Write-Host "=== OPNsense Log Fixture Generator ===" -ForegroundColor Cyan
Write-Host "Generating 3 x ${SizeMB}MB = $($SizeMB * 3 / 1024)GB total" -ForegroundColor Cyan
Write-Host ""

$fixturesDir = Join-Path $PSScriptRoot "..\tests\fixtures"
if (-not (Test-Path $fixturesDir)) {
    New-Item -ItemType Directory -Path $fixturesDir | Out-Null
}

# Realistic data distributions
$actions = @("pass", "block", "reject")
$actionWeights = @(60, 35, 5)  # 60% pass, 35% block, 5% reject

$protocols = @("tcp", "udp", "icmp")
$protocolWeights = @(70, 25, 5)  # 70% TCP, 25% UDP, 5% ICMP
$protocolNumbers = @(6, 17, 1)  # TCP=6, UDP=17, ICMP=1

$interfaces = @("vtnet0", "vtnet1", "em0", "ix0")

$commonPorts = @(443, 80, 22, 53, 3389, 8080, 8443, 25, 110, 143)
$portWeights = @(25, 20, 5, 5, 2, 2, 2, 2, 2, 2)  # Weighted distribution

# Helper function to get weighted random item
function Get-WeightedRandom {
    param(
        [array]$Items,
        [array]$Weights
    )

    $total = ($Weights | Measure-Object -Sum).Sum
    $random = Get-Random -Minimum 0 -Maximum $total
    $cumulative = 0

    for ($i = 0; $i -lt $Items.Count; $i++) {
        $cumulative += $Weights[$i]
        if ($random -lt $cumulative) {
            return $Items[$i]
        }
    }

    return $Items[-1]
}

# Helper function to generate random IP
function Get-RandomIP {
    param([bool]$Private = $false)

    if ($Private -or ((Get-Random -Minimum 0 -Maximum 100) -lt 50)) {
        # RFC1918 private ranges
        $range = Get-Random -Minimum 0 -Maximum 3
        switch ($range) {
            0 { return "192.168.$((Get-Random -Minimum 0 -Maximum 256)).$((Get-Random -Minimum 1 -Maximum 255))" }
            1 { return "10.$((Get-Random -Minimum 0 -Maximum 256)).$((Get-Random -Minimum 0 -Maximum 256)).$((Get-Random -Minimum 1 -Maximum 255))" }
            2 { return "172.$((Get-Random -Minimum 16 -Maximum 32)).$((Get-Random -Minimum 0 -Maximum 256)).$((Get-Random -Minimum 1 -Maximum 255))" }
        }
    }

    # Public IP (avoiding reserved ranges)
    return "$((Get-Random -Minimum 1 -Maximum 224)).$((Get-Random -Minimum 0 -Maximum 256)).$((Get-Random -Minimum 0 -Maximum 256)).$((Get-Random -Minimum 1 -Maximum 255))"
}

# Helper function to get random port
function Get-RandomPort {
    if ((Get-Random -Minimum 0 -Maximum 100) -lt 55) {
        # 55% common ports
        return Get-WeightedRandom -Items $commonPorts -Weights $portWeights
    }
    # 45% random high ports
    return Get-Random -Minimum 1024 -Maximum 65535
}

# 1. Generate RFC3164 format (10GB)
Write-Host "[1/3] Generating RFC3164 format (${SizeMB}MB)..." -ForegroundColor Yellow
$rfc3164Path = Join-Path $fixturesDir "rfc3164_${SizeMB}mb.log"
$bytesPerLine = 200
$lineCount = ($SizeMB * 1024 * 1024) / $bytesPerLine

$streamWriter = [System.IO.StreamWriter]::new($rfc3164Path, $false, [System.Text.Encoding]::UTF8, 65536)

try {
    $timestamp = Get-Date
    for ($i = 0; $i -lt $lineCount; $i++) {
        if ($i % 100000 -eq 0) {
            Write-Progress -Activity "Generating RFC3164" -Status "$([int]($i / $lineCount * 100))% complete" -PercentComplete ($i / $lineCount * 100)
            $timestamp = $timestamp.AddSeconds(1)
        }

        $action = Get-WeightedRandom -Items $actions -Weights $actionWeights
        $protocol = Get-WeightedRandom -Items $protocols -Weights $protocolWeights
        $protocolNum = Get-WeightedRandom -Items $protocolNumbers -Weights $protocolWeights
        $interface = $interfaces[(Get-Random -Minimum 0 -Maximum $interfaces.Count)]
        $sourceIP = Get-RandomIP
        $destIP = Get-RandomIP
        $sourcePort = Get-RandomPort
        $destPort = Get-RandomPort

        $priority = 134  # local0.info
        $month = $timestamp.ToString("MMM")
        $day = $timestamp.Day
        $time = $timestamp.ToString("HH:mm:ss")

        # RFC3164 format: <priority>Month Day Time hostname filterlog: CSV_DATA
        $line = "<$priority>$month $day $time fw1 filterlog: 5,,,$(1000000000 + $i),$interface,match,$action,in,4,0x0,,64,$(Get-Random -Minimum 1000 -Maximum 65535),0,none,$protocolNum,$protocol,60,$sourceIP,$destIP,$sourcePort,$destPort,0,S,$(Get-Random -Minimum 1000000000 -Maximum 9999999999),,64240,,mss;sackOK;TS"

        $streamWriter.WriteLine($line)
    }
} finally {
    $streamWriter.Close()
}

Write-Host "  ✓ RFC3164 generated: $rfc3164Path" -ForegroundColor Green

# 2. Generate RFC5424 format (10GB)
Write-Host "[2/3] Generating RFC5424 format (${SizeMB}MB)..." -ForegroundColor Yellow
$rfc5424Path = Join-Path $fixturesDir "rfc5424_${SizeMB}mb.log"

$streamWriter = [System.IO.StreamWriter]::new($rfc5424Path, $false, [System.Text.Encoding]::UTF8, 65536)

try {
    $timestamp = Get-Date
    for ($i = 0; $i -lt $lineCount; $i++) {
        if ($i % 100000 -eq 0) {
            Write-Progress -Activity "Generating RFC5424" -Status "$([int]($i / $lineCount * 100))% complete" -PercentComplete ($i / $lineCount * 100)
            $timestamp = $timestamp.AddSeconds(1)
        }

        $action = Get-WeightedRandom -Items $actions -Weights $actionWeights
        $protocol = Get-WeightedRandom -Items $protocols -Weights $protocolWeights
        $protocolNum = Get-WeightedRandom -Items $protocolNumbers -Weights $protocolWeights
        $interface = $interfaces[(Get-Random -Minimum 0 -Maximum $interfaces.Count)]
        $sourceIP = Get-RandomIP
        $destIP = Get-RandomIP
        $sourcePort = Get-RandomPort
        $destPort = Get-RandomPort

        $priority = 134
        $timestampISO = $timestamp.ToString("yyyy-MM-ddTHH:mm:ssZ")

        # RFC5424 format: <priority>VERSION TIMESTAMP HOSTNAME APP-NAME PROCID MSGID [STRUCTURED-DATA] MSG
        $line = "<$priority>1 $timestampISO fw1 filterlog - - [meta sequenceId=`"$(1000000000 + $i)`"] 5,,,$(1000000000 + $i),$interface,match,$action,in,4,0x0,,64,$(Get-Random -Minimum 1000 -Maximum 65535),0,none,$protocolNum,$protocol,60,$sourceIP,$destIP,$sourcePort,$destPort,0,S,$(Get-Random -Minimum 1000000000 -Maximum 9999999999),,64240,,mss;sackOK;TS"

        $streamWriter.WriteLine($line)
    }
} finally {
    $streamWriter.Close()
}

Write-Host "  ✓ RFC5424 generated: $rfc5424Path" -ForegroundColor Green

# 3. Generate CSV filterlog format (10GB)
Write-Host "[3/3] Generating CSV filterlog format (${SizeMB}MB)..." -ForegroundColor Yellow
$csvPath = Join-Path $fixturesDir "csv_filterlog_${SizeMB}mb.log"

$streamWriter = [System.IO.StreamWriter]::new($csvPath, $false, [System.Text.Encoding]::UTF8, 65536)

try {
    for ($i = 0; $i -lt $lineCount; $i++) {
        if ($i % 100000 -eq 0) {
            Write-Progress -Activity "Generating CSV" -Status "$([int]($i / $lineCount * 100))% complete" -PercentComplete ($i / $lineCount * 100)
        }

        $action = Get-WeightedRandom -Items $actions -Weights $actionWeights
        $protocol = Get-WeightedRandom -Items $protocols -Weights $protocolWeights
        $protocolNum = Get-WeightedRandom -Items $protocolNumbers -Weights $protocolWeights
        $interface = $interfaces[(Get-Random -Minimum 0 -Maximum $interfaces.Count)]
        $sourceIP = Get-RandomIP
        $destIP = Get-RandomIP
        $sourcePort = Get-RandomPort
        $destPort = Get-RandomPort

        # CSV filterlog format: rule,sub,anchor,tracker,interface,reason,action,dir,version,tos,ecn,ttl,id,offset,flags,proto_num,proto,length,src,dst,src_port,dst_port,data_length,tcp_flags,seq,ack,window,urg,options
        $line = "5,,,$(1000000000 + $i),$interface,match,$action,in,4,0x0,,64,$(Get-Random -Minimum 1000 -Maximum 65535),0,none,$protocolNum,$protocol,60,$sourceIP,$destIP,$sourcePort,$destPort,0,S,$(Get-Random -Minimum 1000000000 -Maximum 9999999999),,64240,,mss;sackOK;TS"

        $streamWriter.WriteLine($line)
    }
} finally {
    $streamWriter.Close()
}

Write-Host "  ✓ CSV filterlog generated: $csvPath" -ForegroundColor Green

Write-Host ""
Write-Host "=== Generation Complete ===" -ForegroundColor Cyan
Write-Host "Total size: $($SizeMB * 3 / 1024)GB" -ForegroundColor Green
Write-Host "Files created in: $fixturesDir" -ForegroundColor Green
Write-Host ""
Write-Host "⚠️  Remember: Add tests/fixtures/*.log to .gitignore" -ForegroundColor Yellow
