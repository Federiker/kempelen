# Errors

manuzio's speech crate returned `anyhow::Error` everywhere. Its TODO wants typed errors in libraries and
`anyhow` only at the boundaries, so kempelen has one error type (owner, 27 October 2026), and `anyhow`
only in its examples and tests.

## `kempelen::Error`

Every fallible function returns `kempelen::Result<T>`, that is `Result<T, kempelen::Error>`. The enum is
`#[non_exhaustive]`: match the variants you act on and keep a fallback arm.

| Variant | When |
| --- | --- |
| `InvalidSpeed(f32)` | The speed is not finite and greater than zero (checked before anything is loaded) |
| `UnsupportedLanguage(Language)` | Narration cannot pronounce the book language yet (Japanese, Ukrainian, Bulgarian) |
| `UnsupportedReading(String)` | `JespersenPhonemizer::new` got a reading code jespersen does not read |
| `NoSpeech` | The text holds nothing that can be spoken |
| `Asset { path, problem }` | A model or voice file is `Missing`, `Unreadable`, `Incomplete` (wrong size), fails its `Checksum`, or is `Unknown` to the manifest (`AssetProblem`) |
| `Download { path, message }` | `assets::stage` (feature `speech-download`): the manifest names an unsafe source, or a download failed, was larger than pinned or failed its checksum |
| `InvalidVoice(&str)` | A voice file is not a list of 256-wide style vectors |
| `NoStyle { styles, length }` | `load_voice` was asked for a sequence longer than the voice has styles for |
| `Phonemes { text, writing, source }` | jespersen could not read the words, or write their phonemes (`source` is jespersen's error) |
| `Phonemizer(Box<dyn Error>)` | Another `Phonemizer` (one of the caller's) failed |
| `Model(ort::Error)` | ONNX Runtime could not load the model or run it |
| `File { action, path, source }` | A file could not be created, read or written; the message names it |
| `WavTooLarge` | A WAV would be longer than its 32-bit header can say |
| `Interrupted(Box<dyn Error>)` | A callback of `synthesize_into` (`sink` or `before_chunk`) returned its own error, kept as it was |
| `Cuda(CudaError)` | The GPU runtime could not be installed, loaded or used |

The messages are manuzio's where it had one ("text produced no Kokoro phonemes", "Narration is not
available for Japanese books yet", "Cannot read \"…\""), so an application that showed them reads the
same. `Error::source` gives the cause where there is one (jespersen's error, the I/O error).

### Stopping a synthesis from a callback

`synthesize_into` calls `before_chunk(done, total)` before each model chunk and `sink(samples)` for each
block, and stops with the first error either returns. To stop for a reason of your own, return
`Error::interrupted(your_error)`, then recognise it afterwards:

```rust
let result = synthesizer.synthesize_into(text, &scratch, &mut sink, &mut |_, _| {
    if cancelled() {
        return Err(kempelen::Error::interrupted(Cancelled));
    }
    Ok(())
});
if let Err(kempelen::Error::Interrupted(error)) = &result
    && error.downcast_ref::<Cancelled>().is_some()
{
    // The user cancelled: not a failure.
}
```

## `kempelen::cuda::CudaError`

Inside `Error::Cuda`, and what `cuda::install` returns. A caller that gets one from
`Synthesizer::with_cuda` falls back to `Synthesizer::new` (the CPU).

| Variant | When |
| --- | --- |
| `Unsupported` | Not 64-bit Windows |
| `NotInstalled` | The runtime folder lacks this version's marker or one of its libraries |
| `NoDevice` | ONNX Runtime found no CUDA device |
| `NoApplicationFolder` | The running program's folder cannot be found |
| `SharedProviderMissing(PathBuf)` | `onnxruntime_providers_shared.dll` is not next to the program |
| `Library { library, message }` | A library of the runtime would not load |
| `Load(String)` | Registering the runtime failed earlier in this process, so it is not tried again |
| `Download { url, message }`, `Size(url)`, `Unpack { path, message }`, `File { action, path, message }` | Installing (feature `cuda-install`): a download failed or failed its checksum, an archive has another size than the pinned one, could not be unpacked or lacks a library, or a file could not be written or moved |

`CudaError` is `Clone` and `PartialEq` and carries messages rather than sources: a failed registration is
remembered for the whole process, and returned again to every later caller.
