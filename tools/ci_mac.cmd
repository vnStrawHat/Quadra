@echo off
rem Runs the .sh of this name in Git Bash, for cmd.exe; GIT_BASH names another bash.exe.
setlocal
if not defined GIT_BASH set "GIT_BASH=%ProgramFiles%\Git\bin\bash.exe"
set "script=%~dpn0.sh"
"%GIT_BASH%" "%script:\=/%" %*
