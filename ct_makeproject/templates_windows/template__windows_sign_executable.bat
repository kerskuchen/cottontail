@echo off

pushd shipping_windows
copy "{{windows_certificate_path}}" "{{project_company_name}}.cer"
signtool.exe sign /t http://timestamp.verisign.com/scripts/timstamp.dll /fd sha256 /s {{project_company_name}} /n {{project_company_name}} {{project_name}}.exe
del {{project_company_name}}.cer
REM NOTE: This will throw an error that it is not a root certificate
signtool.exe verify {{project_name}}.exe
popd

pause