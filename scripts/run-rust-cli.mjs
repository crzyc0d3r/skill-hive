import { existsSync } from "node:fs";
import { delimiter, dirname, join } from "node:path";
import { spawnCommandSync } from "./spawn-tool.mjs";

function canRun(command, args = ["--version"]) {
  const result = spawnCommandSync(command, args, { stdio: "ignore" });
  return result.status === 0;
}

function existingBinary(file) {
  if (existsSync(file)) return file;
  if (process.platform === "win32" && existsSync(`${file}.exe`)) return `${file}.exe`;
  return null;
}

function resolveCargo() {
  if (process.env.CARGO) {
    const fromEnv = existingBinary(process.env.CARGO);
    if (fromEnv) return fromEnv;
  }
  if (canRun("cargo")) return "cargo";
  const rustupCheck = spawnCommandSync("rustup", ["which", "rustc"], { encoding: "utf8" });
  if (rustupCheck.status === 0) {
    const rustcPath = rustupCheck.stdout.trim();
    if (rustcPath) {
      const cargoPath = existingBinary(join(dirname(rustcPath), "cargo"));
      if (cargoPath) return cargoPath;
    }
  }
  console.error("cargo not found. Install Rust or ensure cargo/rustup is on PATH.");
  process.exit(127);
}

const mode = process.argv[2];
const extraArgs = process.argv.slice(3);
const cargo = resolveCargo();
const baseArgs = ["--manifest-path", "crates/hive-cli/Cargo.toml", "--bin", "skill-hive-cli"];
const cargoArgs =
  mode === "cli"
    ? ["run", "--quiet", ...baseArgs, "--", ...extraArgs]
    : mode === "build"
      ? ["build", ...baseArgs]
      : mode === "install"
        ? ["install", "--path", "crates/hive-cli", "--bin", "skill-hive-cli", "--locked", "--force"]
        : null;

if (!cargoArgs) {
  console.error(`unknown mode: ${mode}`);
  process.exit(2);
}

const result = spawnCommandSync(cargo, cargoArgs, {
  stdio: "inherit",
  env: {
    ...process.env,
    PATH: `${dirname(cargo)}${delimiter}${process.env.PATH ?? ""}`,
  },
});

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}
process.exit(result.status ?? 1);
