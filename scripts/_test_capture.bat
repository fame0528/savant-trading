@echo off
echo BEFORE_FORF: TOKEN_ROUTER_API_KEY defined? 
for /f "usebackq delims=" %%L in (`powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0load-env.ps1"`) do %%L
echo AFTER_FORF: TOKEN_ROUTER_API_KEY defined? 
if defined TOKEN_ROUTER_API_KEY (
    echo YES - length:
) else (
    echo NO - var not set
)
