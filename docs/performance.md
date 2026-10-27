# Performance

kempelen runs manuzio's synthesis unchanged, so it should be exactly as fast. This page keeps the numbers
that show it, and manuzio's own measurements from before the split.

## The benchmark

`mise run bench` (`cargo run --release -p kempelen --example bench`) is manuzio's `mnz-bench speech`:
the Introduction and Chapters I and II of manuzio's sample book (*The Wonderful Wizard of Oz*, public
domain), as manuzio's audiobook plans them (8 segments, 89 passages, `examples/bench/oz.json`), read by
`bf_emma` at speed 0.9 on the CPU, with the phonemes, the model and the levelling timed apart. It prints
the stage times, the peak memory (peak working set, Windows only) and a SHA-256 over every segment's WAV.

## kempelen against manuzio, 27 October 2026

The same text through manuzio's `mnz-tts` (the harness, [parity.md](parity.md)) and through kempelen,
alternately, twice each, release builds, the same staged model and voices. Machine: AMD Ryzen 7 5800H
(8 cores, 16 threads), 31 GB RAM, Windows 11, shared with another session (runs of the same code vary by
5–25 % on this laptop, so differences under about 10 % need repeated runs).

| | manuzio 1 | kempelen 1 | manuzio 2 | kempelen 2 |
| --- | --- | --- | --- | --- |
| phonemes alone (jespersen, 89 passages) | 0.02 s | 0.06 s | 0.03 s | 0.02 s |
| model load | 1.32 s | 3.37 s | 1.31 s | 1.17 s |
| phonemes and model | 305.45 s | 303.92 s | 270.40 s | 281.62 s |
| join chunks / levelling / WAV encode | 0.03 / 0.44 / 0.10 s | 0.02 / 0.18 / 0.05 s | 0.02 / 0.15 / 0.04 s | 0.02 / 0.22 / 0.05 s |
| **total** | 307.45 s | 307.65 s | 271.98 s | 283.15 s |
| audio | 1,082.9 s | 1,082.9 s | 1,082.9 s | 1,082.9 s |
| real time | 3.52× | 3.52× | 3.98× | 3.82× |
| peak memory | 1,378 MB | 1,377 MB | 1,378 MB | 1,376 MB |

Every run gave the same segments hash, `d47a2bcd…a248face`: the same samples. The differences in time
follow the machine's load, not the program (run 1 of each was slower, run 2 of each faster); the model is
more than 99 % of the time, and it is the same ONNX Runtime build, session settings and thread count. The
first model load of a run reads the 326 MB model from a cold disk cache, hence 3.37 s once.

## manuzio's earlier measurements (its docs/optimisation-notes.md)

Measured on the same laptop, 23 October 2026, with eSpeak NG then (1,089 s of audio for the same
chapters; jespersen now reads them in 1,083 s):

| | Time |
| --- | --- |
| baseline audiobook, CPU, ONNX Runtime's default threads (8) | 314.5 / 330.0 s (3.46× / 3.30×) |
| with every logical core (16 threads), the setting kempelen keeps | 284.6 s (3.83×) |
| final audiobook after streaming and the other slices | 289.3 s (3.77×), peak 1,109 MB |
| the same on CUDA (RTX 3060 Laptop at a 30 W power cap) | 155.9 s, peak 1,383 MB |
| cancelling inside a chapter body (before the next model chunk) | 0.6 s |

Why every logical core: about 10 % faster than ONNX Runtime's default of the physical cores. The thread
count changes the order in which Kokoro draws its noise, so the samples differ from one count to another
while sounding the same; at a given count they are reproducible. That is why the sample references hold
for one machine ([parity.md](parity.md)).

The audiobook's peak memory there (1.1 GB) is lower than this benchmark's (1.4 GB), which keeps each
segment's whole buffer to time the levelling apart; `synthesize_into` keeps memory flat: about 0.8 GB is
ONNX Runtime with the model loaded, and the samples do not add to it however long the text.
