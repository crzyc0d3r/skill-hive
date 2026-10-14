import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import { applyBundle, decideSigning } from "./signing-mode.mjs";
import { verifyAssetNames } from "./verify-updater-assets.mjs";

const FIVE = [
  "Skill Hive_1.0.0_aarch64.dmg",
  "Skill Hive_1.0.0_aarch64.app.tar.gz",
  "Skill Hive_1.0.0_x64.dmg",
  "Skill Hive_1.0.0_x64.app.tar.gz",
  "Skill Hive_1.0.0_x64_en-US.msi",
  "Skill Hive_1.0.0_x64-setup.exe",
  "skill-hive_1.0.0_amd64.AppImage",
  "skill-hive_1.0.0_amd64.deb",
  "skill-hive-1.0.0-1.x86_64.rpm",
  "skill-hive_1.0.0_aarch64.AppImage",
  "skill-hive_1.0.0_arm64.deb",
  "skill-hive-1.0.0-1.aarch64.rpm",
  "skill-hive-cli-aarch64-apple-darwin",
  "skill-hive-cli-x86_64-apple-darwin",
  "skill-hive-cli-x86_64-unknown-linux-gnu",
  "skill-hive-cli-aarch64-unknown-linux-gnu",
  "skill-hive-cli-x86_64-pc-windows-msvc.exe",
];

const SIGS = [
  "Skill Hive_1.0.0_aarch64.app.tar.gz.sig",
  "Skill Hive_1.0.0_x64.app.tar.gz.sig",
  "Skill Hive_1.0.0_x64-setup.exe.sig",
  "skill-hive_1.0.0_amd64.AppImage.sig",
  "skill-hive_1.0.0_aarch64.AppImage.sig",
  "latest.json",
];

test("no secrets keeps every platform unsigned", () => {
  const decision = decideSigning({});
  assert.equal(decision.ok, true);
  assert.equal(decision.apple.mode, "unsigned");
  assert.equal(decision.windows.mode, "unsigned");
  assert.equal(decision.updater.mode, "unsigned");
  const bundle = applyBundle({ bundle: { createUpdaterArtifacts: false, macOS: { minimumSystemVersion: "10.15" } } }, decision);
  assert.equal(bundle.bundle.createUpdaterArtifacts, false);
  assert.equal(bundle.bundle.windows, undefined);
});

test("a complete App Store Connect set turns Apple signing on", () => {
  const env = {
    APPLE_CERTIFICATE: "cert",
    APPLE_CERTIFICATE_PASSWORD: "pw",
    APPLE_SIGNING_IDENTITY: "Developer ID Application: Skill Hive",
    APPLE_API_ISSUER: "issuer",
    APPLE_API_KEY: "KEYID",
    APPLE_API_PRIVATE_KEY: "p8",
  };
  const decision = decideSigning(env);
  assert.equal(decision.apple.mode, "sign");
  assert.equal(decision.apple.path, "api-key");
  const bundle = applyBundle({ bundle: { macOS: {} } }, decision);
  assert.equal(bundle.bundle.macOS.hardenedRuntime, true);
});

test("a partial Apple set fails loudly", () => {
  const decision = decideSigning({ APPLE_CERTIFICATE: "cert" });
  assert.equal(decision.ok, false);
  assert.ok(decision.partials.some((item) => item.startsWith("apple:")));
});

test("Azure Trusted Signing sets the Windows signCommand hook", () => {
  const env = {
    AZURE_TENANT_ID: "tenant",
    AZURE_CLIENT_ID: "client",
    AZURE_CLIENT_SECRET: "secret",
    AZURE_TRUSTED_SIGNING_ENDPOINT: "https://example.codesigning.azure.net",
    AZURE_CODE_SIGNING_ACCOUNT_NAME: "skill-hive",
    AZURE_CERTIFICATE_PROFILE_NAME: "skill-hive",
  };
  const decision = decideSigning(env);
  assert.equal(decision.windows.path, "azure-trusted-signing");
  const bundle = applyBundle({ bundle: {} }, decision);
  assert.match(bundle.bundle.windows.signCommand, /windows-sign\.ps1/);
});

test("an updater password without the private key is partial", () => {
  const decision = decideSigning({ TAURI_SIGNING_PRIVATE_KEY_PASSWORD: "pw" });
  assert.equal(decision.ok, false);
  assert.ok(decision.partials.includes("updater:TAURI_SIGNING_PRIVATE_KEY"));
});

test("the updater private key enables updater artifacts only", () => {
  const decision = decideSigning({ TAURI_SIGNING_PRIVATE_KEY: "minisign" });
  assert.equal(decision.updater.mode, "sign");
  assert.equal(decision.apple.mode, "unsigned");
  const bundle = applyBundle({ bundle: { createUpdaterArtifacts: false } }, decision);
  assert.equal(bundle.bundle.createUpdaterArtifacts, true);
});

test("unsigned verification accepts the five targets without signatures", () => {
  assert.deepEqual(verifyAssetNames(FIVE, { signed: false }), []);
});

test("a missing signature blocks publication", () => {
  const missing = verifyAssetNames([...FIVE, ...SIGS.filter((name) => !name.includes("aarch64.AppImage"))], { signed: true });
  assert.ok(missing.includes("linux-aarch64 sig"));
});

test("signed verification passes when every signature is present", () => {
  assert.deepEqual(verifyAssetNames([...FIVE, ...SIGS], { signed: true }), []);
});

test("the release workflow keeps unsigned builds and names the secret gates", () => {
  const workflow = fs.readFileSync(".github/workflows/release.yml", "utf8");
  for (const needle of [
    "ubuntu-24.04-arm",
    "verify-updater-assets",
    "publish-release",
    "releaseDraft: true",
    "timeout-minutes: 60",
    "APPLE_API_PRIVATE_KEY",
    "TAURI_SIGNING_PRIVATE_KEY",
    "AZURE_CERTIFICATE_PROFILE_NAME",
    "WINDOWS_CERTIFICATE",
    "Fail loudly when signing secrets are partial",
  ]) {
    assert.ok(workflow.includes(needle), needle);
  }
});
