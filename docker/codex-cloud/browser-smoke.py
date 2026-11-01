#!/usr/bin/env python3
"""Compatibility entry point; the canonical smoke test lives under scripts/."""
import runpy,sys
from pathlib import Path
if '--instance' not in sys.argv:sys.argv.extend(['--instance','cloud'])
runpy.run_path(str(Path(__file__).resolve().parents[2]/'scripts/browser-smoke.py'),run_name='__main__')
