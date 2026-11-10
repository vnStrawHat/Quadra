// The Windows install helper. All paths come through the environment: cmd.exe parses a batch
// file before running it, so a path inlined here could inject commands. It avoids PowerShell,
// which some environments block by policy, and runs without a console: `timeout` fails there, so
// a loopback ping is the one-second sleep. /R overwrites read-only files.
module update

const windows_update_script = r'@echo off
setlocal DisableDelayedExpansion
set "pid=%ONETERM_UPDATER_PID%"
set "log=%ONETERM_UPDATE_LOG%"

call :log "helper started; install=%ONETERM_INSTALL_DIR% package=%ONETERM_PACKAGE_DIR%"
call :log "waiting for OneTerm (pid %pid%) to exit"

:wait_for_exit
tasklist /FI "PID eq %pid%" 2>NUL | find "%pid%" >NUL
if not errorlevel 1 (
    ping -n 2 127.0.0.1 >NUL 2>NUL
    goto wait_for_exit
)
call :log "OneTerm exited; backing up the current install"

>>"%log%" 2>&1 xcopy "%ONETERM_INSTALL_DIR%\*" "%ONETERM_BACKUP_DIR%\" /E /I /H /R /Y
if errorlevel 2 (
    call :log "ERROR: backup xcopy failed (errorlevel %errorlevel%); aborting without changes"
    exit /B 1
)
call :log "installing the new package over the install directory"
>>"%log%" 2>&1 xcopy "%ONETERM_PACKAGE_DIR%\*" "%ONETERM_INSTALL_DIR%\" /E /I /H /R /Y
if errorlevel 2 (
    call :log "ERROR: package xcopy failed (errorlevel %errorlevel%); rolling back"
    goto restore_backup
)
rem Reset errorlevel before start: a non-fatal xcopy errorlevel 1 must not roll back a launch.
(call )
call :log "launching the new build: %ONETERM_INSTALL_DIR%\%ONETERM_EXE_NAME%"
start "" "%ONETERM_INSTALL_DIR%\%ONETERM_EXE_NAME%"
if errorlevel 1 (
    call :log "ERROR: launch failed (errorlevel %errorlevel%); rolling back"
    goto restore_backup
)
call :log "update complete; cleaning up backup and staging directories"
cd /d "%~dp0" >NUL 2>NUL
rmdir /S /Q "%ONETERM_BACKUP_DIR%" >NUL 2>NUL
rmdir /S /Q "%ONETERM_STAGING_DIR%" >NUL 2>NUL
del "%ONETERM_SCRIPT_PATH%" >NUL 2>NUL
exit /B 0

:restore_backup
del /F /Q "%ONETERM_INSTALL_DIR%\*" >NUL 2>NUL
for /D %%D in ("%ONETERM_INSTALL_DIR%\*") do rmdir /S /Q "%%D" >NUL 2>NUL
>>"%log%" 2>&1 xcopy "%ONETERM_BACKUP_DIR%\*" "%ONETERM_INSTALL_DIR%\" /E /I /H /R /Y
call :log "rolled back to the previous install; backup kept at %ONETERM_BACKUP_DIR%"
exit /B 1

:log
>>"%log%" echo [%date% %time%] %~1
goto :eof
'
