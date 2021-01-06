@echo off
REM WARNING: This file was generated by `ct_makeproject` and should not be modified

REM Install `cargo-watch` if not found
where cargo-watch > nul 2> nul
IF %errorlevel% neq 0 (
    echo Installing `cargo-watch`
    cargo install cargo-watch
)
IF %errorlevel% neq 0 (
    echo ERROR: Could not install `cargo-watch`
    goto :error
)


if not exist assets mkdir assets
if not exist assets_copy mkdir assets_copy
if not exist assets_executable mkdir assets_executable

cargo watch --watch assets --watch assets_copy --watch assets_executable --watch cottontail/ct_assetbaker --exec "run --release --package ct_assetbaker"

goto :done 

REM ------------------------------------------------------------------------------------------------
:error
echo Failed with error #%errorlevel%.
pause
exit /b %errorlevel%

REM ------------------------------------------------------------------------------------------------
:done
echo DONE 