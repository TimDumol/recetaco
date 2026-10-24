#!/bin/sh
set -eu
cd "$(git rev-parse --show-toplevel)"
python3 -m venv .security-venv
.security-venv/bin/python -m pip install foxguard==0.14.1 pre-commit==4.5.1
.security-venv/bin/pre-commit install --install-hooks
printf '%s\n' 'Foxguard installed in .security-venv/bin; pre-commit hook enabled.'
