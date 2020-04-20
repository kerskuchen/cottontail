@echo off

if exist shipping_windows rmdir /s /q shipping_windows
mkdir shipping_windows

if exist temp rmdir /s /q temp
mkdir temp

cargo run --package ct_assetbaker && cargo build --release --package launcher
if %errorlevel% neq 0 goto :error

REM NOTE: robocopy has success error code 1
robocopy "resources" "shipping_windows\resources" /s /e > nul
if %errorlevel% neq 1 goto :error

REM Check if we have resource hacker in %path%
where ResourceHacker.exe > nul 2> nul
if %errorlevel% neq 0 goto :noicon

ResourceHacker.exe -log temp/log1.txt -open resources_executable/versioninfo.rc -save temp/versioninfo.res -action compile 
if %errorlevel% neq 0 goto :error
ResourceHacker.exe -log temp/log2.txt -open target/release/launcher.exe -save temp/launcher_tmp1.exe -action add -res temp/versioninfo.res 
if %errorlevel% neq 0 goto :error
ResourceHacker.exe -log temp/log3.txt -open temp/launcher_tmp1.exe -save temp/launcher_tmp2.exe -action add -res resources_executable/launcher.ico -mask ICONGROUP,MAINICON,  
if %errorlevel% neq 0 goto :error

copy ".\temp\launcher_tmp2.exe" ".\shipping_windows\{{project_name}}.exe" > nul
if %errorlevel% neq 0 goto :error

goto :done

REM ------------------------------------------------------------------------------------------------
:noicon
echo ResourceHacker.exe not detected in PATH - Skipping embedding launcher icon and version info
copy ".\target\release\launcher.exe" ".\shipping_windows\{{project_name}}.exe" > nul
if %errorlevel% neq 0 goto :error
goto :done

REM ------------------------------------------------------------------------------------------------
:error
echo Failed with error #%errorlevel%.
pause
exit /b %errorlevel%

REM ------------------------------------------------------------------------------------------------
:done
rmdir /s /q "temp"
echo FINISHED BUILDING WINDOWS SHIPPING