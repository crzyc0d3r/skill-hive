import fs from "node:fs";

const rustPath = "src-tauri/src/lib.rs";
const dtsPath = "src/lib/ipc/ipc.d.ts";

const known = {
  get_settings: { args: "{ key: string }", result: "string | null" },
  set_settings: { args: "{ key: string; value: string }", result: "null" },
  log_startup_event: { args: "{ label: string; elapsedMs: number }", result: "null" },
  list_base_entries: { args: "Record<string, never>", result: "string[]" },
};

export function commandNames(source) {
  const names = [];
  const re = /#\[tauri::command\]\s*(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z0-9_]+)/g;
  let match;
  while ((match = re.exec(source))) names.push(match[1]);
  return names;
}

export function render(names) {
  const fields = names.map((name) => {
    const spec = known[name] ?? { args: "Record<string, unknown>", result: "unknown" };
    return `  ${name}: { args: ${spec.args}; result: ${spec.result} };`;
  });
  return `export interface IpcCommands {\n${fields.join("\n")}\n}\n`;
}

function differs(rust, dts) {
  const rendered = render(commandNames(rust));
  return { rendered, match: rendered.trim() === dts.trim() };
}

const selfTest = process.argv.includes("--self-test");
const check = process.argv.includes("--check");

if (selfTest) {
  const rust = "#[tauri::command]\nfn get_settings() {}\n#[tauri::command]\nfn extra_widget() {}\n";
  const dts = "export interface IpcCommands {\n  get_settings: { args: { key: string }; result: string | null };\n}\n";
  const result = differs(rust, dts);
  if (result.match || !result.rendered.includes("extra_widget")) {
    console.error("self-test failed: a command missing from ipc.d.ts must be reported");
    process.exit(1);
  }
  console.log("self-test: deliberate command/type mismatch detected");
}

if (!selfTest || check) {
  const rust = fs.readFileSync(rustPath, "utf8");
  const names = commandNames(rust);
  const rendered = render(names);
  if (names.length === 0) {
    console.error(`no #[tauri::command] fns in ${rustPath}`);
    process.exit(1);
  }
  if (check) {
    const current = fs.readFileSync(dtsPath, "utf8");
    if (current.trim() !== rendered.trim()) {
      console.error(`${dtsPath} is out of date with ${rustPath}`);
      console.error(rendered);
      process.exit(1);
    }
    console.log(`${dtsPath} matches ${names.join(", ")}`);
  } else if (!selfTest) {
    fs.writeFileSync(dtsPath, rendered);
    console.log(`wrote ${dtsPath}`);
  }
}
