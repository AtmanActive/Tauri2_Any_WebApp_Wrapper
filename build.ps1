$env:PATH = "C:\Users\dev\nodejs;C:\Users\dev\.cargo\bin;" + $env:PATH
Set-Location "C:\Users\dev\Documents\TaWebMidi"
npx tauri build 2>&1
