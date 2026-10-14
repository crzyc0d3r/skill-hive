from pathlib import Path

result = Path("automations/skill-hive-wiring/out/result.md").read_text()
audit = Path("docs/wiring-audit.md").read_text()
assert "ipc-ok" in result
assert "audit-ok" in result
assert "real IPC" in audit
assert "Not completed" in audit
print("result-ok")
