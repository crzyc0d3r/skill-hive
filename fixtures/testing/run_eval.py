#!/usr/bin/env python3
"""Trigger judge shared with the Rust engine. No model calls."""

import json
import sys


def judge_stream(stream, command_name):
    tool_name = None
    buffer = ""
    decided = None
    for line in stream.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        event = value.get("event") if value.get("type") == "stream_event" else value
        if not isinstance(event, dict):
            event = {}
        event_type = event.get("type") or ""
        if event_type == "content_block_start":
            block = event.get("content_block") or {}
            if block.get("type") == "tool_use" and tool_name is None:
                name = block.get("name") or ""
                if name not in ("Skill", "Read"):
                    decided = False
                    break
                tool_name = name
        elif event_type == "content_block_delta":
            delta = event.get("delta") or {}
            if delta.get("type") == "input_json_delta":
                partial = delta.get("partial_json") or ""
                buffer += partial
                if tool_name is not None and command_name in buffer:
                    decided = True
                    break
        elif event_type in ("content_block_stop", "message_stop") and tool_name is not None:
            decided = command_name in buffer
            break
        if value.get("type") == "assistant":
            content = ((value.get("message") or {}).get("content")) or []
            for block in content:
                if block.get("type") == "tool_use" and tool_name is None:
                    name = block.get("name") or ""
                    blob = json.dumps(block, separators=(",", ":"))
                    if name not in ("Skill", "Read"):
                        return False
                    return command_name in blob
    return bool(decided)


def main():
    fixture = json.loads(open(sys.argv[1], encoding="utf-8").read())
    command_name = sys.argv[2]
    threshold = float(sys.argv[3])
    results = []
    for row in fixture["queries"]:
        streams = row.get("streams") or []
        hits = sum(1 for stream in streams if judge_stream(stream, command_name))
        runs = max(len(streams), 1)
        rate = hits / runs
        should = bool(row["should_trigger"])
        passed = rate >= threshold if should else rate < threshold
        results.append({"query": row["query"], "pass": passed, "trigger_rate": rate})
    json.dump({"results": results}, sys.stdout)


if __name__ == "__main__":
    main()
