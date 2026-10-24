#!/usr/bin/env python3
"""Repeatable Linux RSS smoke measurement; creates an isolated temporary database."""
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import time
import urllib.request

root = Path(__file__).resolve().parents[1]
binary = root / "target/release/recetaco"
port = 18082
base = f"http://127.0.0.1:{port}"

def request(path, body=None):
    raw = None if body is None else json.dumps(body).encode()
    req = urllib.request.Request(base + path, data=raw, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=5) as res:
        return json.load(res)

def memory(pid):
    data = dict(line.split(":", 1) for line in Path(f"/proc/{pid}/status").read_text().splitlines() if ":" in line)
    return {name: round(int(data[name].split()[0]) / 1024, 2) for name in ("VmRSS", "VmHWM")}

with tempfile.TemporaryDirectory(prefix="recetaco-memory-") as directory:
    env = dict(os.environ, RECETACO_BIND=f"127.0.0.1:{port}", RECETACO_DB=str(Path(directory) / "test.db"), RECETACO_TOKEN="", RECETACO_WEB_DIR=str(root / "web/dist"))
    for name in ("RECETACO_OIDC_ISSUER", "RECETACO_OIDC_CLIENT_ID", "RECETACO_OIDC_ANDROID_CLIENT_ID", "RECETACO_OIDC_AUDIENCE"):
        env.pop(name, None)
    server = subprocess.Popen([str(binary)], env=env, stdout=subprocess.DEVNULL)
    try:
        for _ in range(100):
            if server.poll() is not None:
                raise RuntimeError("Server exited before measurement")
            try:
                request("/health")
                break
            except OSError:
                time.sleep(0.05)
        else:
            raise RuntimeError("Server did not become ready")
        initial = memory(server.pid)
        recipe_ids = []
        for n in range(100):
            r = request("/api/recipes", {"title": f"Recipe {n}", "servings": 2, "instructions": "Cook gently. Serve warm.", "ingredients": [{"name": f"Ingredient {i}", "quantity": 100, "unit": "g"} for i in range(5)]})
            recipe_ids.append(r["id"])
        for recipe_id in recipe_ids:
            request(f"/api/recipes/{recipe_id}/cooks", {"cooked_on": "2026-10-24", "rating": 4, "went_well": "Texture", "to_improve": "Less salt", "notes": "Used stock"})
        shopping_list = request("/api/lists", {"name": "This week"})
        for n in range(7):
            request("/api/meals", {"recipe_id": recipe_ids[n], "date": f"2026-10-{24+n}", "servings": 4, "slot": "dinner"})
            request(f'/api/lists/{shopping_list["id"]}/recipes/{recipe_ids[n]}', {"servings": 4})
        for _ in range(100):
            request("/api/recipes")
            request(f'/api/lists/{shopping_list["id"]}')
        print(json.dumps({"platform": "Linux x86_64", "units": "MiB", "scope": "Rust server only; release build; 100 recipes x 5 ingredients, 7 meals, 100 cook logs, 1 shopping list, 200 read requests", "initial": initial, "after_workload": memory(server.pid)}, indent=2))
    finally:
        server.send_signal(signal.SIGINT)
        server.wait(timeout=5)
