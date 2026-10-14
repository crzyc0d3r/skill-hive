import fs from "node:fs";

const config = JSON.parse(fs.readFileSync("src-tauri/tauri.conf.json", "utf8"));
const caps = JSON.parse(fs.readFileSync("src-tauri/capabilities/default.json", "utf8"));
const problems = [];

if (config.productName !== "Skill Hive") problems.push("productName must stay Skill Hive");
if (config.identifier !== "com.skillhive.desktop") problems.push("identifier must stay com.skillhive.desktop");
if (String(config.plugins?.updater?.pubkey ?? "").includes("386AFFED")) {
  problems.push("updater public key still matches the previous placeholder");
}
if (config.bundle?.createUpdaterArtifacts !== false) {
  problems.push("committed config must leave updater artifacts off until the signing key is present");
}
const csp = String(config.app?.security?.csp ?? "");
if (!csp.includes("default-src 'self'") || /connect-src[^;]*https:/.test(csp)) {
  problems.push("CSP must keep connect-src off the network");
}
if ((caps.permissions ?? []).some((item) => String(item).includes("shell"))) {
  problems.push("the webview capability must not grant shell");
}
const deb = config.bundle?.linux?.deb?.depends ?? [];
for (const dep of ["libwebkit2gtk-4.1-0", "libayatana-appindicator3-1"]) {
  if (!deb.includes(dep)) problems.push(`deb depends missing ${dep}`);
}
const rpm = config.bundle?.linux?.rpm?.depends ?? [];
for (const dep of ["webkit2gtk4.1", "libayatana-appindicator-gtk3"]) {
  if (!rpm.includes(dep)) problems.push(`rpm depends missing ${dep}`);
}

if (problems.length) {
  console.error(problems.join("\n"));
  process.exit(1);
}
console.log("hardening checks passed for CSP, capabilities, identity, and Linux dependencies");
