$ErrorActionPreference = "Stop"

function Stop-OhosEnv {
    param([string]$Message)

    [Console]::Error.WriteLine("error: $Message")
    throw "OpenHarmony Cargo environment setup failed."
}

if ([string]::IsNullOrWhiteSpace($env:OHOS_NATIVE_SDK)) {
    Stop-OhosEnv "set OHOS_NATIVE_SDK to the OpenHarmony native SDK directory."
}

if (-not (Test-Path -LiteralPath $env:OHOS_NATIVE_SDK -PathType Container -ErrorAction SilentlyContinue)) {
    Stop-OhosEnv "OHOS_NATIVE_SDK does not exist: $env:OHOS_NATIVE_SDK"
}

$sdk = (Resolve-Path -LiteralPath $env:OHOS_NATIVE_SDK -ErrorAction Stop).Path
$env:OHOS_NATIVE_SDK = $sdk
$clang = [System.IO.Path]::Combine($sdk, "llvm", "bin", "clang.exe")
$clangxx = [System.IO.Path]::Combine($sdk, "llvm", "bin", "clang++.exe")
$ar = [System.IO.Path]::Combine($sdk, "llvm", "bin", "llvm-ar.exe")
$libclang = [System.IO.Path]::Combine($sdk, "llvm", "bin", "libclang.dll")
$sysroot = [System.IO.Path]::Combine($sdk, "sysroot")
$cmakeToolchain = [System.IO.Path]::Combine($sdk, "build", "cmake", "ohos.toolchain.cmake")
$linker = [System.IO.Path]::Combine($PSScriptRoot, "ohos", "ohos-clang.cmd")

$requiredFiles = @($clang, $clangxx, $ar, $libclang, $cmakeToolchain)
foreach ($path in $requiredFiles) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf -ErrorAction SilentlyContinue)) {
        Stop-OhosEnv "required OpenHarmony SDK file is missing: $path"
    }
}

if (-not (Test-Path -LiteralPath $linker -PathType Leaf -ErrorAction SilentlyContinue)) {
    Stop-OhosEnv "required repository linker wrapper is missing: $linker"
}

if (-not (Test-Path -LiteralPath $sysroot -PathType Container -ErrorAction SilentlyContinue)) {
    Stop-OhosEnv "required OpenHarmony SDK sysroot is missing: $sysroot"
}

$target = "aarch64_unknown_linux_ohos"
$targetUpper = "AARCH64_UNKNOWN_LINUX_OHOS"
$commonFlags = "-target aarch64-linux-ohos --sysroot=`"$sysroot`" -D__MUSL__"

$env:CARGO_TARGET_AARCH64_UNKNOWN_LINUX_OHOS_LINKER = $linker
$env:AR_aarch64_unknown_linux_ohos = $ar
$env:CC_aarch64_unknown_linux_ohos = $clang
$env:CXX_aarch64_unknown_linux_ohos = $clangxx
$env:CC_SHELL_ESCAPED_FLAGS = "1"
Set-Item -Path "Env:CFLAGS_$target" -Value $commonFlags
Set-Item -Path "Env:CXXFLAGS_$target" -Value $commonFlags
Set-Item -Path "Env:CMAKE_TOOLCHAIN_FILE_$target" -Value $cmakeToolchain
$env:LIBCLANG_PATH = [System.IO.Path]::GetDirectoryName($libclang)
Set-Item -Path "Env:BINDGEN_EXTRA_CLANG_ARGS_$target" -Value $commonFlags

$separator = [char]0x1f
$env:CARGO_ENCODED_RUSTFLAGS = @(
    "-Clink-arg=-target",
    "-Clink-arg=aarch64-linux-ohos",
    "-Clink-arg=--sysroot=$sysroot",
    "-Clink-arg=-D__MUSL__"
) -join $separator

Write-Host "Configured OpenHarmony Cargo environment for $targetUpper from $sdk"
