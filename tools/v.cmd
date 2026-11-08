@echo off
rem Runs tools/v.sh with Git Bash from cmd (plain `bash` there is often WSL, see v.sh).
"%ProgramFiles%\Git\bin\bash.exe" "%~dp0v.sh" %*
