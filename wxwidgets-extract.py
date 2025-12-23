# Flatpak-specific workaround
import pathlib
import shutil
import zipfile

zip_path = pathlib.Path("wxWidgets-3.3.1.zip")
extract_root = pathlib.Path("wxwidgets-extract")

if extract_root.exists():
    shutil.rmtree(extract_root)
extract_root.mkdir(parents=True, exist_ok=True)

with zipfile.ZipFile(zip_path, "r") as zf:
    zf.extractall(extract_root)

candidates = list(extract_root.iterdir())
if len(candidates) == 1 and candidates[0].is_dir():
    source_dir = candidates[0]
else:
    source_dir = extract_root

if not (source_dir / "configure").exists():
    raise SystemExit("wxWidgets configure not found after extraction")

dest_dir = pathlib.Path("target/release/wxWidgets")
if dest_dir.exists():
    shutil.rmtree(dest_dir)

dest_dir.parent.mkdir(parents=True, exist_ok=True)
shutil.copytree(source_dir, dest_dir)
