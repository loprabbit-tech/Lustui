$ErrorActionPreference = "Stop"

$installDir = "$env:LOCALAPPDATA\Lustui"
$exePath = "$installDir\lustui.exe"
$repoOwner = "loprabbit-tech"
$repoName = "Lustui"

Write-Host "LustUI のインストールを開始します..." -ForegroundColor Cyan

# 1. インストール先フォルダの作成
if (!(Test-Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir | Out-Null
}

# 2. 最新リリースの lustui.exe をダウンロード
$downloadUrl = "https://github.com/$repoOwner/$repoName/releases/latest/download/lustui.exe"
Write-Host "最新のバイナリを取得中: $downloadUrl" -ForegroundColor Yellow
Invoke-WebRequest -Uri $downloadUrl -OutFile $exePath

# 3. PATH環境変数の追加
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$installDir", "User")
    Write-Host "PATH環境変数を更新しました。" -ForegroundColor Green
}

# 4. .lustuiprj 拡張子の関連付け
Write-Host "ファイル関連付け (.lustuiprj) を設定中..." -ForegroundColor Yellow
$regPathExt = "HKCU:\Software\Classes\.lustuiprj"
$regPathType = "HKCU:\Software\Classes\LustUI.Project"

New-Item -Path $regPathExt -Force | Out-Null
Set-ItemProperty -Path $regPathExt -Name "(Default)" -Value "LustUI.Project"

New-Item -Path "$regPathType\shell\open\command" -Force | Out-Null
Set-ItemProperty -Path "$regPathType\shell\open\command" -Name "(Default)" -Value "`"$exePath`" `"%1`""

Write-Host "`nLustUI のインストールが完了しました！" -ForegroundColor Green
Write-Host "ターミナルを再起動すると 'lustui new <name>' コマンドが使用可能になります。" -ForegroundColor Cyan
