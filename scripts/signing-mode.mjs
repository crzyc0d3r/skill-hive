/**
 * Decide whether a Skill Hive release signs, stays unsigned, or fails.
 * A platform signs only when one complete secret set is present.
 * A partial set fails the job so tauri-action cannot fall back to an ad-hoc signature.
 */

export const APPLE_CLASSIC = [
  "APPLE_CERTIFICATE",
  "APPLE_CERTIFICATE_PASSWORD",
  "APPLE_SIGNING_IDENTITY",
  "APPLE_ID",
  "APPLE_PASSWORD",
  "APPLE_TEAM_ID",
];

export const APPLE_API = [
  "APPLE_CERTIFICATE",
  "APPLE_CERTIFICATE_PASSWORD",
  "APPLE_SIGNING_IDENTITY",
  "APPLE_API_ISSUER",
  "APPLE_API_KEY",
  "APPLE_API_PRIVATE_KEY",
];

export const WINDOWS_OV = ["WINDOWS_CERTIFICATE", "WINDOWS_CERTIFICATE_PASSWORD"];

export const WINDOWS_AZURE = [
  "AZURE_TENANT_ID",
  "AZURE_CLIENT_ID",
  "AZURE_CLIENT_SECRET",
  "AZURE_TRUSTED_SIGNING_ENDPOINT",
  "AZURE_CODE_SIGNING_ACCOUNT_NAME",
  "AZURE_CERTIFICATE_PROFILE_NAME",
];

export const UPDATER_KEY = "TAURI_SIGNING_PRIVATE_KEY";
export const UPDATER_PASSWORD = "TAURI_SIGNING_PRIVATE_KEY_PASSWORD";

export function secretPresent(env, name) {
  return typeof env[name] === "string" && env[name].trim() !== "";
}

function group(env, names) {
  const missing = names.filter((name) => !secretPresent(env, name));
  if (missing.length === names.length) return { mode: "unsigned", missing: [] };
  if (missing.length === 0) return { mode: "sign", missing: [] };
  return { mode: "partial", missing };
}

export function decideSigning(env) {
  const appleClassic = group(env, APPLE_CLASSIC);
  const appleApi = group(env, APPLE_API);
  const appleNames = [...new Set([...APPLE_CLASSIC, ...APPLE_API])];
  let apple;
  if (!appleNames.some((name) => secretPresent(env, name))) {
    apple = { mode: "unsigned", path: "none", missing: [] };
  } else if (appleApi.mode === "sign") {
    apple = { mode: "sign", path: "api-key", missing: [] };
  } else if (appleClassic.mode === "sign") {
    apple = { mode: "sign", path: "apple-id", missing: [] };
  } else {
    const missing = appleApi.missing.length <= appleClassic.missing.length ? appleApi.missing : appleClassic.missing;
    apple = { mode: "partial", path: "incomplete", missing };
  }

  const windowsOv = group(env, WINDOWS_OV);
  const windowsAzure = group(env, WINDOWS_AZURE);
  const windowsNames = [...new Set([...WINDOWS_OV, ...WINDOWS_AZURE])];
  let windows;
  if (!windowsNames.some((name) => secretPresent(env, name))) {
    windows = { mode: "unsigned", path: "none", missing: [] };
  } else if (windowsAzure.mode === "sign") {
    windows = { mode: "sign", path: "azure-trusted-signing", missing: [] };
  } else if (windowsOv.mode === "sign") {
    windows = { mode: "sign", path: "ov-ev", missing: [] };
  } else {
    const missing = windowsAzure.missing.length <= windowsOv.missing.length ? windowsAzure.missing : windowsOv.missing;
    windows = { mode: "partial", path: "incomplete", missing };
  }

  const key = secretPresent(env, UPDATER_KEY);
  const passwordOnly = secretPresent(env, UPDATER_PASSWORD) && !key;
  const updater = passwordOnly
    ? { mode: "partial", missing: [UPDATER_KEY] }
    : key
      ? { mode: "sign", missing: [] }
      : { mode: "unsigned", missing: [] };

  const partials = [
    ...apple.missing.map((name) => `apple:${name}`),
    ...windows.missing.map((name) => `windows:${name}`),
    ...updater.missing.map((name) => `updater:${name}`),
  ];
  return { apple, windows, updater, ok: partials.length === 0, partials };
}

export function applyBundle(config, decision) {
  const next = structuredClone(config);
  next.bundle = next.bundle ?? {};
  next.bundle.createUpdaterArtifacts = decision.updater.mode === "sign";
  if (decision.apple.mode === "sign") {
    next.bundle.macOS = { ...(next.bundle.macOS ?? {}), hardenedRuntime: true };
  }
  if (decision.windows.mode === "sign" && decision.windows.path === "azure-trusted-signing") {
    next.bundle.windows = {
      ...(next.bundle.windows ?? {}),
      signCommand: "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/windows-sign.ps1 %1",
    };
  }
  return next;
}
