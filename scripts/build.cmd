@echo off
rem Release build of Vorto. Run it from any folder.
rem
rem   scripts\build.cmd              vorto.exe and both voice engines in %VORTO_TARGET%\release
rem   scripts\build.cmd --installer  the installer in target\release\bundle\nsis. With
rem                                  TAURI_SIGNING_PRIVATE_KEY and TAURI_SIGNING_PRIVATE_KEY_PASSWORD
rem                                  set, also its update signature (.sig).
rem   scripts\build.cmd --cpu        without the graphics card engine, so no Vulkan SDK is needed.
rem                                  Installers always include it.
rem
rem Needs Visual Studio 2022 or newer with the C++ workload, CMake, LLVM, Node.js with
rem `npm ci --prefix ui` done and, without --cpu, the Vulkan SDK. The graphics card engine is
rem built with Ninja (part of Visual Studio) in its own short target folder: whisper.cpp's Vulkan
rem shader build exceeds the 260-character path limit of MSBuild and cl.exe otherwise.
setlocal
set "INSTALLER="
set "GPU=1"
set "BAD="
for %%A in (%*) do (
  if /i "%%~A"=="--installer" (set "INSTALLER=1") else if /i "%%~A"=="--cpu" (set "GPU=") else (set "BAD=%%~A")
)
if defined BAD goto bad
if defined INSTALLER if not defined GPU goto cpu_installer

set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
for /f "usebackq delims=" %%I in (`"%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VSDIR=%%I"
if not defined VSDIR goto no_vs
call "%VSDIR%\VC\Auxiliary\Build\vcvars64.bat" >nul || exit /b 1
set "PATH=%USERPROFILE%\.cargo\bin;%ProgramFiles%\CMake\bin;%PATH%"
set CMAKE_GENERATOR=Ninja
set VSLANG=1033
if not defined VORTO_TARGET set "VORTO_TARGET=C:\vt"
if not defined VORTO_GPU_TARGET set "VORTO_GPU_TARGET=C:\vtg"

cd /d "%~dp0.."
call npm --prefix ui run build || exit /b 1
cargo build --release --locked -p vorto-engine --target-dir "%VORTO_TARGET%" || exit /b 1
if not defined GPU goto app
if not defined VULKAN_SDK for /d %%D in (C:\VulkanSDK\*) do set "VULKAN_SDK=%%D"
set "PATH=%VULKAN_SDK%\Bin;%PATH%"
cargo build --release --locked -p vorto-engine --no-default-features --features vulkan --target-dir "%VORTO_GPU_TARGET%" || exit /b 1
copy /y "%VORTO_GPU_TARGET%\release\vorto-engine.exe" "%VORTO_TARGET%\release\vorto-engine-gpu.exe" >nul || exit /b 1

:app
if defined INSTALLER goto installer
cargo build --release --locked -p vorto-app --target-dir "%VORTO_TARGET%" || exit /b 1
node scripts\stage.mjs --check "%VORTO_TARGET%\release\vorto.exe" || exit /b 1
echo.
echo Built vorto.exe and its voice engines in %VORTO_TARGET%\release
exit /b 0

:installer
node scripts\stage.mjs "%VORTO_TARGET%\release\vorto-engine.exe" "%VORTO_TARGET%\release\vorto-engine-gpu.exe" || exit /b 1
if exist target\release\bundle\nsis rmdir /s /q target\release\bundle\nsis
rem The UI is already built; the release config adds the engines, the runtime and the notices.
call ui\node_modules\.bin\tauri.cmd build --ci --config app\tauri.release.conf.json || exit /b 1
node scripts\stage.mjs --check target\release\vorto.exe || exit /b 1
if not defined TAURI_SIGNING_PRIVATE_KEY goto built
for %%F in (target\release\bundle\nsis\*-setup.exe) do call ui\node_modules\.bin\tauri.cmd signer sign "%%F"
dir /b target\release\bundle\nsis\*-setup.exe.sig >nul 2>&1 || exit /b 1
:built
echo.
echo Installer: target\release\bundle\nsis
exit /b 0

:bad
echo Unknown option: %BAD%
exit /b 1
:cpu_installer
echo Installers always include the graphics card engine; build them without --cpu.
exit /b 1
:no_vs
echo Visual Studio with the C++ workload was not found.
exit /b 1
