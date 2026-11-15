# Ghidra Script: Panasonic DMR-EH55 Symbol Importer (Backward-Compatibility Wrapper)
# @category Forensics.Panasonic
# @author Panadiga Engineering Team (Jython 2.7 + Python 3 compatible)
from __future__ import print_function

import os
import runpy

_target = os.path.join(os.path.dirname(os.path.abspath(__file__)), "ghidra_import_symbols_panadiga.py")
runpy.run_path(_target, init_globals={"currentProgram": globals().get("currentProgram")}, run_name="__main__")
