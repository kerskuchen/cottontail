@echo off

IF EXIST shipping_windows rmdir /s /q shipping_windows
mkdir shipping_windows

IF EXIST temp rmdir /s /q temp
mkdir temp

cargo build --release
cargo run --release --package ct_assetbaker

magick convert assets_executable/launcher_icon/512.png -resize 256x256 temp/256.png
magick convert assets_executable/launcher_icon/512.png -resize 128x128 temp/128.png
magick convert assets_executable/launcher_icon/512.png -resize 64x64 temp/64.png
magick convert assets_executable/launcher_icon/512.png -resize 48x48 temp/48.png
magick convert assets_executable/launcher_icon/512.png -resize 32x32 temp/32.png
magick convert assets_executable/launcher_icon/512.png -resize 16x16 temp/16.png

REM NOTE: This also works beautifully for transparent pngs
magick convert temp/16.png temp/32.png temp/48.png temp/64.png temp/128.png temp/256.png temp/icon.ico

ResourceHacker.exe -open assets_executable/versioninfo.rc -save temp/versioninfo.res -action compile 
ResourceHacker.exe -open target/release/launcher.exe -save temp/launcher_tmp1.exe -action add -res temp/versioninfo.res 
ResourceHacker.exe -open temp/launcher_tmp1.exe -save temp/launcher_tmp2.exe -action add -res temp/icon.ico -mask ICONGROUP,MAINICON, 

copy ".\temp\launcher_tmp2.exe" ".\shipping_windows\{{project_name}}.exe"
robocopy "resources" "shipping_windows\resources" /s /e 

rmdir /s /q "temp"

pause