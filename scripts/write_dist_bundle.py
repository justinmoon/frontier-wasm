#!/usr/bin/env python3
from pathlib import Path
import zipfile

dist = Path("dist")
dist.mkdir(parents=True, exist_ok=True)

readme = dist / "README.md"
readme.write_text(
    "# Frontier Counter Demo Bundle\n\n"
    "Thanks for taking the Vello counter demo for a spin! The bundle contains:\n\n"
    "- `frontier-wasm-host`: native host binary\n"
    "- `counter_component.wasm`: release-built guest component\n\n"
    "## Run the Demo\n\n"
    "```sh\n"
    "./frontier-wasm-host --component counter_component.wasm\n"
    "```\n\n"
    "The window opens showing the counter demo. Use the mouse wheel or +/- keys to change the value.\n"
    "Press `R` to restart the component if something goes wrong.\n\n"
    "Happy hacking!\n",
    encoding="utf-8",
)

archive = dist / "frontier-demo.zip"
if archive.exists():
    archive.unlink()

with zipfile.ZipFile(archive, "w") as archive_file:
    for name in ("frontier-wasm-host", "counter_component.wasm", "README.md"):
        archive_file.write(dist / name, arcname=name)
