# Security review

| Control | Where |
|---|---|
| CSP has no network `connect-src` | `src-tauri/tauri.conf.json`. `node scripts/check-hardening.mjs` |
| Webview capabilities omit `shell` | `src-tauri/capabilities/default.json`. The same script |
| Product name and identifier are Skill Hive's | `Skill Hive`, `com.skillhive.desktop` |
| Updater public key is not the previous placeholder | key id `8FC6E5C30C36CA69` |
| Secret scrub | `crates/hive-core/src/util/log_sanitize.rs` covers `sk-ant-`, `sk-or-`, and `Bearer` |
| Path guard | `sanitize_rejects_traversal_payloads` in `crates/hive-core/src/util/path_guard.rs` |
| Updater signature | `a_tampered_latest_json_signature_is_rejected` uses `minisign-verify` |
| Audits | `cargo audit`, `cargo deny`, and `npm audit` stay in CI |

No high-severity exception was added for this phase.
