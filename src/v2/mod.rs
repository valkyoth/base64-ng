//! Private module boundaries for the 2.0 implementation.
//!
//! Commit 4 establishes ownership only. Production behavior remains routed
//! through the 1.x modules until later numbered commits move each capability.

// Commit 5 establishes this internal value before Commit 6 wires it into the
// public specification model.
#[allow(dead_code)]
pub(crate) mod alphabet;
mod backend_health;
#[allow(dead_code)]
pub(crate) mod contracts;
#[allow(dead_code)]
pub(crate) mod incremental;
#[allow(dead_code)]
pub(crate) mod incremental_decoder;
#[allow(dead_code)]
mod lifecycle;
mod ordinary;
mod secret;
#[allow(dead_code)]
pub(crate) mod specifications;
#[allow(dead_code)]
mod wrapping;

#[cfg(test)]
mod alphabet_tests;
#[cfg(test)]
mod contract_tests;
#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod incremental_decoder_tests;
#[cfg(test)]
mod incremental_decoder_unpadded_tests;
#[cfg(test)]
mod incremental_encoder_tests;
#[cfg(test)]
mod rfc4648_oracle;
#[cfg(test)]
mod specification_tests;
#[cfg(test)]
mod wrapping_tests;
