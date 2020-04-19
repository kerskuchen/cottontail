@echo off

IF EXIST shipping_windows rmdir /s /q shipping_windows
mkdir shipping_windows

IF EXIST temp rmdir /s /q temp
mkdir temp

cargo run --package ct_assetbaker && ^
cargo build --release --package launcher
if %errorlevel% neq 0 goto :error

ResourceHacker.exe -log temp/log1.txt -open resources_executable/versioninfo.rc -save temp/versioninfo.res -action compile 
if %errorlevel% neq 0 goto :error
ResourceHacker.exe -log temp/log2.txt -open target/release/launcher.exe -save temp/launcher_tmp1.exe -action add -res temp/versioninfo.res 
if %errorlevel% neq 0 goto :error
ResourceHacker.exe -log temp/log3.txt -open temp/launcher_tmp1.exe -save temp/launcher_tmp2.exe -action add -res resources_executable/launcher.ico -mask ICONGROUP,MAINICON,  
if %errorlevel% neq 0 goto :error

copy ".\temp\launcher_tmp2.exe" ".\shipping_windows\{{project_name}}.exe" > nul
if %errorlevel% neq 0 goto :error

REM NOTE: robocopy has success error code 1
robocopy "resources" "shipping_windows\resources" /s /e > nul
if %errorlevel% neq 1 goto :error

REM NOTE: rmdir has success error code 1
rmdir /s /q "temp"
if %errorlevel% neq 1 goto :error

goto :done

:error
echo Failed with error #%errorlevel%.
pause
exit /b %errorlevel%

:done
echo FINISHED BUILDING WINDOWS SHIPPING