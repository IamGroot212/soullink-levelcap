#!/usr/bin/env python3
"""Lädt Wachstumsraten für alle Gen-1..6-Pokémon (#1..721) aus der PokéAPI
und schreibt sie nach data/species_growth.json.

Einmal-Skript — Ergebnis wird ins Repo committed, danach baut build.rs daraus
die phf-Map zur Compile-Zeit.

Usage:
    python3 scripts/fetch_growth_rates.py
"""
from __future__ import annotations

import json
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

API = "https://pokeapi.co/api/v2/pokemon-species/{id}"
MAX_ID = 721  # bis inklusive Volcanion (Ende Gen 6)
OUT = Path(__file__).parent.parent / "data" / "species_growth.json"
USER_AGENT = "soullink-levelcap-build/0.1 (github.com/IamGroot212/soullink-levelcap)"


def fetch(species_id: int, retries: int = 3) -> str:
    url = API.format(id=species_id)
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    for attempt in range(retries):
        try:
            with urllib.request.urlopen(req, timeout=15) as resp:
                data = json.load(resp)
            return data["growth_rate"]["name"]
        except (urllib.error.URLError, KeyError) as exc:
            print(f"  retry {attempt + 1}/{retries} für #{species_id}: {exc}", file=sys.stderr)
            time.sleep(1 + attempt)
    raise SystemExit(f"PokéAPI gibt #{species_id} nach {retries} Versuchen nicht her")


def main() -> None:
    species: dict[str, str] = {}
    for sid in range(1, MAX_ID + 1):
        if sid % 25 == 0:
            print(f"... {sid}/{MAX_ID}", file=sys.stderr)
        species[str(sid)] = fetch(sid)
        time.sleep(0.05)  # PokéAPI ist freundlich, aber wir auch

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(
        json.dumps(
            {
                "_comment": "Auto-generated from PokéAPI by scripts/fetch_growth_rates.py",
                "_pokeapi_slugs": [
                    "erratic", "fast", "medium-fast",
                    "medium-slow", "slow", "fluctuating",
                ],
                "species": species,
            },
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )
    print(f"geschrieben: {OUT} ({len(species)} Einträge)")


if __name__ == "__main__":
    main()
