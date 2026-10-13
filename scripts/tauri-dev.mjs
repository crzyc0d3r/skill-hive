import { spawn } from "node:child_process";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";

const base = resolve(process.env.SKILL_HIVE_DEV_BASE ?? ".dev-base");
mkdirSync(base, { recursive: true });

const child = spawn(
  "npx",
  ["tauri", "dev", "--config", "src-tauri/tauri.dev.conf.json"],
  {
    stdio: "inherit",
    env: { ...process.env, SKILL_HIVE_BASE: base },
  },
);
child.on("exit", (code) => process.exit(code ?? 1));
