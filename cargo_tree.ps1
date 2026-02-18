$env:PATH = "C:\Users\dev\nodejs;C:\Users\dev\.cargo\bin;" + $env:PATH
Set-Location "C:\Users\dev\Documents\TaWebMidi\src-tauri"
cargo tree -p webview2-com --depth 0 2>&1
Write-Output "---"
cargo tree -i webview2-com 2>&1
Write-Output "---"
cargo tree -p tauri -d 2>&1
