//! Shared support for fuzz targets and their deterministic regression tests.

pub mod pem_document;

#[cfg(fuzzing)]
pub mod incremental;
