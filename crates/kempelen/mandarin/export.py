"""Regenerates the Mandarin data files from pypinyin and misaki (development only).

    uv venv misaki-venv && uv pip install --python misaki-venv/Scripts/python.exe "misaki[zh]"
    misaki-venv/Scripts/python.exe -I crates/tts/mandarin/export.py

Writes next to this script:
  characters.tsv  one reading (pypinyin's first, numbered tone, 5 = neutral) for each Han character
  phrases.tsv     each pypinyin phrase; a reading only when it differs from its characters' own
  syllables.tsv   misaki's IPA for each numbered syllable that appears above

pypinyin (MIT) supplies the readings and misaki (Apache-2.0) the sound table; see
crates/tts/src/phonemes/chinese/README.md for the licences. The Rust code reads these files, so
it never needs Python.
"""

import re
import sys
from pathlib import Path

from misaki.zh import ZHG2P
from pypinyin import Style, lazy_pinyin
from pypinyin.phrases_dict import phrases_dict
from pypinyin.pinyin_dict import pinyin_dict

HAN = re.compile("^[一-鿿]+$")
HERE = Path(__file__).resolve().parent


def reading(items):
    return lazy_pinyin(items, style=Style.TONE3, neutral_tone_with_five=True)


characters = {}
for code in sorted(pinyin_dict):
    character = chr(code)
    if HAN.match(character):
        characters[character] = reading([character])[0]

phrases = {}
for phrase in sorted(phrases_dict):
    if not HAN.match(phrase) or len(phrase) < 2:
        continue
    syllables = reading([phrase])
    if len(syllables) != len(phrase):
        print("skipped", phrase, syllables, file=sys.stderr)
        continue
    own = [characters.get(character) for character in phrase]
    phrases[phrase] = None if syllables == own else syllables

used = set(characters.values())
for syllables in phrases.values():
    used.update(syllables or [])

table = {}
for syllable in sorted(used):
    try:
        table[syllable] = ZHG2P.py2ipa(syllable).replace(chr(815), "")
    except Exception as error:  # a syllable misaki itself cannot pronounce
        print("unpronounceable", syllable, error, file=sys.stderr)

dropped = [c for c, s in characters.items() if s not in table]
for character in dropped:
    del characters[character]
for phrase in [p for p, s in phrases.items() if s and any(x not in table for x in s)]:
    del phrases[phrase]

with open(HERE / "characters.tsv", "w", encoding="utf8", newline="\n") as out:
    for character, syllable in characters.items():
        out.write(f"{character}\t{syllable}\n")
with open(HERE / "phrases.tsv", "w", encoding="utf8", newline="\n") as out:
    for phrase, syllables in phrases.items():
        out.write(phrase if syllables is None else f"{phrase}\t{' '.join(syllables)}")
        out.write("\n")
with open(HERE / "syllables.tsv", "w", encoding="utf8", newline="\n") as out:
    for syllable, ipa in table.items():
        out.write(f"{syllable}\t{ipa}\n")

print(len(characters), "characters,", len(phrases), "phrases,", len(table), "syllables")
