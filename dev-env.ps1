# Build-Umgebung fuer lokale Entwicklung (Windows).
# Nutzung:  . .\dev-env.ps1   (dot-sourcen, dann bun tauri dev / cargo build)
#
# Hintergrund: Toolchain liegt teils portabel unter C:\dev\tools (kein Admin),
# target/ wird nach C:\h umgeleitet (.cargo/config.toml) wegen Windows MAX_PATH
# und OneDrive-Sync. ORT dynamisch wie in CI (build.yml), Vulkan wie in CI.

$env:Path = "$env:USERPROFILE\.cargo\bin;C:\dev\tools\cmake\bin;C:\VulkanSDK\1.4.309.0\Bin;" + $env:Path
$env:VULKAN_SDK = 'C:\VulkanSDK\1.4.309.0'
$env:CMAKE_PREFIX_PATH = 'C:/dev/tools/spirv-headers'
$env:ORT_LIB_LOCATION = 'C:\dev\tools\onnxruntime-win-x64-1.24.2\lib'
$env:ORT_PREFER_DYNAMIC_LINK = '1'
$env:CMAKE_POLICY_VERSION_MINIMUM = '3.5'

Write-Host "Build-Umgebung gesetzt (Rust $(& cargo --version | ForEach-Object { ($_ -split ' ')[1] }), CMake, Vulkan SDK, ORT 1.24.2)"
