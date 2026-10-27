//! kempelen's integration tests as one program: one link instead of one per file. Each file is a
//! module; nextest still runs every test in its own process.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::float_cmp,
    reason = "tests: their helpers may panic, and they measure audio"
)]

mod cuda;
mod golden;
mod offline;
mod parity;
mod speech;
mod synthesis;
