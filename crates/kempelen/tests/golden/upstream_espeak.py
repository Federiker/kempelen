"""Reference-capture helper: phonemize through upstream eSpeak NG (a GPL library) over stdio.

Development tool only. Nothing here ships with kempelen; it exists so the golden corpus can be
captured from the real eSpeak NG and compared with the Rust crate and with replacement engines.

    python -I upstream_espeak.py LANGUAGE [LIBRARY DATA_PARENT]

Needs `pip install espeakng-loader` (bundles espeak-ng.dll/.so/.dylib and its data) unless
LIBRARY and DATA_PARENT (the directory containing `espeak-ng-data`) are given.
Protocol: one JSON string per stdin line in, one JSON string per line out; clauses are
joined with a single space. The first output line is the library version banner.
"""

import ctypes
import json
import os
import sys

IPA = 0x02
UTF8 = 1
SYNCHRONOUS = 2


def load(argv):
    if len(argv) >= 4:
        library, data_parent = argv[2], argv[3]
    else:
        import espeakng_loader

        library = espeakng_loader.get_library_path()
        data_parent = os.path.dirname(espeakng_loader.get_data_path())
    lib = ctypes.CDLL(library)
    lib.espeak_Initialize.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_char_p, ctypes.c_int]
    lib.espeak_SetVoiceByName.argtypes = [ctypes.c_char_p]
    lib.espeak_TextToPhonemes.argtypes = [ctypes.POINTER(ctypes.c_void_p), ctypes.c_int, ctypes.c_int]
    lib.espeak_TextToPhonemes.restype = ctypes.c_char_p
    lib.espeak_Info.argtypes = [ctypes.c_void_p]
    lib.espeak_Info.restype = ctypes.c_char_p
    if lib.espeak_Initialize(SYNCHRONOUS, 0, data_parent.encode(), 0) < 0:
        raise SystemExit("espeak_Initialize failed")
    return lib


def phonemize(lib, text):
    buffer = ctypes.create_string_buffer(text.encode("utf-8"))
    pointer = ctypes.c_void_p(ctypes.addressof(buffer))
    clauses = []
    while pointer.value:
        chunk = lib.espeak_TextToPhonemes(ctypes.byref(pointer), UTF8, IPA)
        if chunk:
            clauses.append(chunk.decode("utf-8"))
    return " ".join(clauses)


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    sys.stdin.reconfigure(encoding="utf-8")
    lib = load(sys.argv)
    voice = {"no": "nb"}.get(sys.argv[1], sys.argv[1])  # eSpeak calls Norwegian Bokmål "nb"
    if lib.espeak_SetVoiceByName(voice.encode()) != 0:
        raise SystemExit(f"unknown eSpeak voice {sys.argv[1]!r}")
    print(json.dumps(lib.espeak_Info(None).decode()), flush=True)
    for line in sys.stdin:
        print(json.dumps(phonemize(lib, json.loads(line))), flush=True)


if __name__ == "__main__":
    main()
