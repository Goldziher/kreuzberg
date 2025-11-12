# Python Smoke App

Minimal script that imports `kreuzberg`, extracts text from a bundled fixture,
and asserts we get meaningful output.

## Run against PyPI

```bash
cd e2e/smoke/python
python -m venv .venv
source .venv/bin/activate
pip install --upgrade pip
pip install -r requirements-smoke.txt
python main.py
```

## Run against a local wheel

```bash
cd e2e/smoke/python
python -m venv .venv
source .venv/bin/activate
pip install --upgrade pip
pip install /path/to/kreuzberg-*-manylinux.whl
python main.py
```
