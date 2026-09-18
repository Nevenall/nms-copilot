#!/usr/bin/env python3
"""Generate crates/nms-core/data/items.json, the item ID to display name table.

Source: the AssistantNMS API (https://api.nmsassistant.com), which serves data extracted from the game files by internal ID, free and without a key. `/ItemInfo/GameId` lists every ID it knows and `/ItemInfo/GameId/<ID>/en` returns the English name and group for one. See docs/reference/nms-game-notes.md for how it ranks against other sources.

IDs the API does not know (the corvette parts, `B_*`, as of 2026-09-15) are listed in scripts/item-ids-missing.txt after a run so the gap is visible; the tool shows the raw ID for them.

Usage:
    python3 scripts/gen-items.py                # regenerate from the API's full ID list
    python3 scripts/gen-items.py ID [ID ...]    # add or refresh only these IDs (with or without the caret)
"""

import json
import sys
import time
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

API = "https://api.nmsassistant.com"
ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "crates" / "nms-core" / "data" / "items.json"
MISSING = ROOT / "scripts" / "item-ids-missing.txt"
WORKERS = 8


def get(path, retries=4):
    req = urllib.request.Request(API + path, headers={"User-Agent": "nms-copilot gen-items"})
    for attempt in range(retries):
        try:
            with urllib.request.urlopen(req, timeout=30) as resp:
                return json.load(resp)
        except urllib.error.HTTPError as e:
            if 400 <= e.code < 500:
                return None
            if attempt == retries - 1:
                raise
            time.sleep(1.5 * (attempt + 1))
            last = e
        except (urllib.error.URLError, TimeoutError, json.JSONDecodeError) as e:
            if attempt == retries - 1:
                raise
            time.sleep(1.5 * (attempt + 1))
            last = e
    raise last


def fetch(game_id):
    data = get(f"/ItemInfo/GameId/{game_id}/en")
    if not isinstance(data, dict) or not data.get("name"):
        return game_id, None
    entry = {"name": data["name"]}
    if data.get("group"):
        entry["group"] = data["group"]
    return game_id, entry


def main(argv):
    table = {}
    if OUT.exists():
        table = json.loads(OUT.read_text(encoding="utf-8"))
    if argv:
        ids = [a.lstrip("^") for a in argv]
    else:
        ids = get("/ItemInfo/GameId")
    ids = sorted(set(ids))
    print(f"resolving {len(ids)} IDs", file=sys.stderr)
    missing = []
    done = 0
    with ThreadPoolExecutor(max_workers=WORKERS) as pool:
        for game_id, entry in pool.map(fetch, ids):
            done += 1
            if entry is None:
                missing.append(game_id)
                table.pop(game_id, None)
            else:
                table[game_id] = entry
            if done % 200 == 0:
                print(f"  {done}/{len(ids)}", file=sys.stderr)
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_bytes((json.dumps(dict(sorted(table.items())), ensure_ascii=False, indent=0) + "\n").encode("utf-8"))
    if missing:
        MISSING.write_bytes(("\n".join(missing) + "\n").encode("utf-8"))
    print(f"wrote {len(table)} entries to {OUT.relative_to(ROOT)}; {len(missing)} IDs unknown to the API", file=sys.stderr)


if __name__ == "__main__":
    main(sys.argv[1:])
