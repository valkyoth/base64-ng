//! Private module boundaries for the 2.0 implementation.
//!
//! Commit 4 establishes ownership only. Production behavior remains routed
//! through the 1.x modules until later numbered commits move each capability.

mod backend_health;
mod incremental;
mod ordinary;
mod secret;
mod specifications;

#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod rfc4648_oracle;
