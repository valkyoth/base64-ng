//! Stable reporting and destination-contract categories.

/// Reporting category for the selected ordinary backend.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum BackendClass {
    /// Portable scalar execution.
    Scalar,
    /// Admitted accelerated execution.
    Accelerated,
    /// Redundantly checked backend execution.
    Checked,
}

impl BackendClass {
    /// Returns the stable lowercase reporting identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scalar => "scalar",
            Self::Accelerated => "accelerated",
            Self::Checked => "checked",
        }
    }
}

/// Reporting category for an operation's assurance boundary.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum AssuranceClass {
    /// Ordinary input-dependent operation.
    Ordinary,
    /// Ordinary operation with redundant backend checking.
    CheckedBackend,
    /// Constant-time-oriented bounded secret operation.
    Secret,
    /// Deployment-attested high-assurance operation.
    HighAssurance,
}

impl AssuranceClass {
    /// Returns the stable lowercase reporting identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ordinary => "ordinary",
            Self::CheckedBackend => "checked-backend",
            Self::Secret => "secret",
            Self::HighAssurance => "high-assurance",
        }
    }
}

/// Reporting scope for core and companion protocol behavior.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ProtocolScope {
    /// RFC 4648 core codec behavior.
    Core,
    /// Explicit compatibility behavior that is not strict RFC 4648.
    Compatibility,
    /// A separately named companion protocol.
    Companion,
}

impl ProtocolScope {
    /// Returns the stable lowercase reporting identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Compatibility => "compatibility",
            Self::Companion => "companion",
        }
    }
}

/// Destination mutation contract used by API and corpus metadata.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum Atomicity {
    /// Destination bytes remain unchanged on every returned error.
    Unchanged,
    /// A rewindable destination is restored to its entry length or position.
    Rollback,
    /// Exact reported prefixes remain committed.
    CommittedPrefix,
    /// A third-party sink may have irreversible or unknowable side effects.
    IrrevocableSink,
}

impl Atomicity {
    /// Returns the stable lowercase corpus/reporting identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unchanged => "unchanged",
            Self::Rollback => "rollback",
            Self::CommittedPrefix => "committed-prefix",
            Self::IrrevocableSink => "irrevocable-sink",
        }
    }
}
