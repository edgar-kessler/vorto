// Prepares what the installer puts next to vorto.exe, in app/binaries: the voice engines, the
// Visual C++ runtime they link against and the third-party notices. Also checks that the
// executables only need DLLs every Windows 10 and 11 PC has, or that the installer ships.
// Run it in a Visual Studio developer environment (for dumpbin), as scripts\build.cmd does:
//   node scripts/stage.mjs <vorto-engine.exe> <vorto-engine-gpu.exe>   stage and check both
//   node scripts/stage.mjs --check <file.exe>                          only check imports
import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readdirSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);
const checkOnly = args[0] === "--check";
const exes = (checkOnly ? args.slice(1) : args).map((a) => resolve(a));
if (!exes.length || (!checkOnly && exes.length !== 2)) {
  throw new Error("Usage: node scripts/stage.mjs <vorto-engine.exe> <vorto-engine-gpu.exe> | --check <exe>");
}
for (const exe of exes) if (!existsSync(exe)) throw new Error(`Not found: ${exe}`);

const RUNTIME = ["vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll", "msvcp140_1.dll"];
// Graphics and machine learning libraries that only some PCs have: they must be delay-loaded.
const OPTIONAL = ["directml.dll", "d3d12.dll", "dxgi.dll", "vulkan-1.dll"];
// Part of every Windows 10 and 11 installation.
const SYSTEM = new Set([
  "advapi32.dll", "bcrypt.dll", "bcryptprimitives.dll", "cfgmgr32.dll", "comctl32.dll", "comdlg32.dll",
  "crypt32.dll", "dbghelp.dll", "dwmapi.dll", "gdi32.dll", "gdiplus.dll", "imm32.dll", "iphlpapi.dll",
  "kernel32.dll", "msimg32.dll", "ncrypt.dll", "ntdll.dll", "ole32.dll", "oleacc.dll", "oleaut32.dll",
  "powrprof.dll", "propsys.dll", "psapi.dll", "rpcrt4.dll", "secur32.dll", "setupapi.dll", "shcore.dll",
  "shell32.dll", "shlwapi.dll", "user32.dll", "userenv.dll", "uxtheme.dll", "version.dll", "winhttp.dll",
  "winmm.dll", "ws2_32.dll", "wtsapi32.dll", "mswsock.dll", "dnsapi.dll", "netapi32.dll", "winspool.drv",
  "mfplat.dll", "mf.dll", "mfreadwrite.dll", "avrt.dll", "mmdevapi.dll", "ksuser.dll", "d3d11.dll",
  "d2d1.dll", "dwrite.dll", "dcomp.dll", "windowscodecs.dll", "wininet.dll", "urlmon.dll", "sspicli.dll",
]);
const isSystem = (dll) => SYSTEM.has(dll) || dll.startsWith("api-ms-win-") || dll.startsWith("ext-ms-win-");

function imports(file) {
  const text = execFileSync("dumpbin", ["/nologo", "/imports", file], { encoding: "utf8", maxBuffer: 1 << 26 });
  const normal = new Set();
  const delayed = new Set();
  let section = null;
  for (const line of text.split(/\r?\n/)) {
    if (/following imports:/.test(line)) section = normal;
    else if (/following delay load imports:/.test(line)) section = delayed;
    else if (/^\s+Summary/.test(line)) section = null;
    const dll = line.match(/^ {4}(\S+\.(?:dll|drv))\s*$/i)?.[1];
    if (section && dll) section.add(dll.toLowerCase());
  }
  // Every Windows executable imports kernel32: finding nothing means the output wasn't understood.
  if (!normal.size) throw new Error(`Couldn't read the imports of ${file}`);
  return { normal, delayed };
}

function check(file) {
  const { normal, delayed } = imports(file);
  const problems = [];
  for (const dll of normal) {
    if (OPTIONAL.includes(dll)) problems.push(`${dll} must be delay-loaded`);
    else if (!isSystem(dll) && !RUNTIME.includes(dll)) problems.push(`${dll} is neither part of Windows nor shipped`);
  }
  for (const dll of delayed) {
    if (!isSystem(dll) && !OPTIONAL.includes(dll) && !RUNTIME.includes(dll)) problems.push(`${dll} (delay-loaded) is unknown`);
  }
  if (problems.length) throw new Error(`${file}:\n  ${problems.join("\n  ")}`);
  console.log(`${file}: imports ok (${normal.size} loaded, ${delayed.size} delay-loaded)`);
}

exes.forEach(check);
if (!checkOnly) {
  const out = join(root, "app", "binaries");
  rmSync(out, { recursive: true, force: true });
  mkdirSync(join(out, "runtime"), { recursive: true });
  // Tauri's externalBin names carry the target triple.
  copyFileSync(exes[0], join(out, "vorto-engine-x86_64-pc-windows-msvc.exe"));
  copyFileSync(exes[1], join(out, "vorto-engine-gpu-x86_64-pc-windows-msvc.exe"));

  // The newest Visual C++ runtime of any Visual Studio installation. It must be at least as new
  // as every toolset that compiled code in the engine, including the prebuilt ONNX Runtime.
  const vswhere = join(process.env["ProgramFiles(x86)"], "Microsoft Visual Studio", "Installer", "vswhere.exe");
  const installs = execFileSync(vswhere, ["-all", "-products", "*", "-property", "installationPath"], { encoding: "utf8" })
    .split(/\r?\n/)
    .filter(Boolean);
  const version = (v) => v.split(".").map(Number);
  const newer = (a, b) => {
    const [x, y] = [version(a), version(b)];
    for (let i = 0; i < Math.max(x.length, y.length); i++) if ((x[i] ?? 0) !== (y[i] ?? 0)) return (x[i] ?? 0) > (y[i] ?? 0);
    return false;
  };
  let best = null;
  for (const install of installs) {
    const redist = join(install, "VC", "Redist", "MSVC");
    if (!existsSync(redist)) continue;
    for (const v of readdirSync(redist).filter((d) => /^\d+(\.\d+)+$/.test(d))) {
      const x64 = join(redist, v, "x64");
      const crt = existsSync(x64) && readdirSync(x64).find((d) => /^Microsoft\.VC14\d\.CRT$/.test(d));
      if (crt && (!best || newer(v, best.version))) best = { version: v, dir: join(x64, crt) };
    }
  }
  if (!best) throw new Error("No Visual C++ runtime found. Install Visual Studio with the C++ workload.");
  if (newer("14.44", best.version)) throw new Error(`Visual C++ runtime ${best.version} is older than 14.44. Update Visual Studio.`);
  for (const dll of RUNTIME) copyFileSync(join(best.dir, dll), join(out, "runtime", dll));
  console.log(`Staged the engines and Visual C++ runtime ${best.version} in ${out}`);

  execFileSync(process.execPath, [join(root, "scripts", "notices.mjs"), join(out, "THIRD-PARTY-NOTICES.txt")], {
    cwd: root,
    stdio: "inherit",
  });
}
