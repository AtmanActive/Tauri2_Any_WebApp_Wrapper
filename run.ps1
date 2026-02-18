param([string]$Cmd)
$env:PATH = "C:\Users\dev\nodejs;C:\Users\dev\.cargo\bin;" + $env:PATH
Set-Location "C:\Users\dev\Documents\TaWebMidi"
Invoke-Expression $Cmd
