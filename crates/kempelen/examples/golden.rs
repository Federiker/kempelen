//! Maintains the phonemizer golden captures in `tmp/golden` (gitignored).
//!
//! `capture-upstream` records the reference output of a separately built eSpeak NG
//! (see `tests/golden/upstream_espeak.py`) into `espeak-upstream.json`, never committed (docs/jespersen.md).
//! `capture-jespersen` records what jespersen produces (`jespersen.json`), and
//! `report` scores one golden file against another: `report espeak-upstream jespersen`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a development tool: it may panic, and it measures audio"
)]

#[path = "../tests/golden/support.rs"]
mod support;

fn usage() -> anyhow::Error {
    anyhow::anyhow!(
        "usage: golden capture-jespersen | capture-upstream PYTHON [LIBRARY DATA_PARENT] | report REFERENCE CANDIDATE"
    )
}

fn main() -> anyhow::Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("capture-jespersen") => {
            let corpus = support::load_corpus()?;
            let golden = support::capture_jespersen(&corpus)?;
            let path = support::golden_path(support::JESPERSEN);
            std::fs::create_dir_all(path.parent().unwrap())?;
            std::fs::write(&path, serde_json::to_string_pretty(&golden)? + "\n")?;
            println!("Wrote {}", path.display());
            Ok(())
        }
        Some("capture-upstream") => {
            // golden capture-upstream PYTHON [LIBRARY DATA_PARENT]
            let args: Vec<String> = std::env::args().skip(2).collect();
            let python = std::path::PathBuf::from(args.first().ok_or_else(usage)?);
            let library = match (args.get(1), args.get(2)) {
                (Some(library), Some(data)) => Some((library.into(), data.into())),
                _ => None,
            };
            let library: Option<(std::path::PathBuf, std::path::PathBuf)> = library;
            let corpus = support::load_corpus()?;
            let (golden, version) = support::capture_upstream(
                &corpus,
                &python,
                library.as_ref().map(|(l, d)| (l.as_path(), d.as_path())),
            )?;
            let path = support::golden_path(support::ESPEAK_UPSTREAM);
            std::fs::create_dir_all(path.parent().unwrap())?;
            std::fs::write(
                &path,
                serde_json::to_string_pretty(&golden)?
                    + "
",
            )?;
            println!("Upstream eSpeak NG {version}: wrote {}", path.display());
            Ok(())
        }
        Some("report") => {
            // golden report REFERENCE CANDIDATE   (engine names = golden file stems)
            let reference = std::env::args().nth(2).ok_or_else(usage)?;
            let candidate = std::env::args().nth(3).ok_or_else(usage)?;
            print!(
                "{candidate} vs {reference}
{}",
                support::report(
                    &support::load_golden(&reference)?,
                    &support::load_golden(&candidate)?
                )
            );
            Ok(())
        }
        _ => Err(usage()),
    }
}
