#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]

//! `base64-ng` is a `no_std`-first Base64 encoder and decoder.
//!
//! This initial release provides strict scalar RFC 4648-style behavior and
//! caller-owned output buffers. Future SIMD fast paths, including AVX, NEON,
//! and wasm `simd128` candidates, will be required to match this scalar module
//! byte-for-byte.
//!
//! # Examples
//!
//! Encode and decode with caller-owned buffers:
//!
//! ```
//! use base64_ng::{STANDARD, checked_encoded_len};
//!
//! let input = b"hello";
//! const ENCODED_CAPACITY: usize = match checked_encoded_len(5, true) {
//!     Some(len) => len,
//!     None => panic!("encoded length overflow"),
//! };
//! let mut encoded = [0u8; ENCODED_CAPACITY];
//! let encoded_len = STANDARD.encode_slice(input, &mut encoded).unwrap();
//! assert_eq!(&encoded[..encoded_len], b"aGVsbG8=");
//!
//! let mut decoded = [0u8; 5];
//! let decoded_len = STANDARD.decode_slice(&encoded, &mut decoded).unwrap();
//! assert_eq!(&decoded[..decoded_len], input);
//! ```
//!
//! Use the URL-safe no-padding engine:
//!
//! ```
//! use base64_ng::URL_SAFE_NO_PAD;
//!
//! let mut encoded = [0u8; 3];
//! let encoded_len = URL_SAFE_NO_PAD.encode_slice(b"\xfb\xff", &mut encoded).unwrap();
//! assert_eq!(&encoded[..encoded_len], b"-_8");
//! ```

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "simd")]
mod simd;

/// Runtime backend reporting for security-sensitive deployments.
///
/// This module does not enable acceleration. It exposes the backend posture so
/// callers can log, assert, or audit whether execution is scalar-only or merely
/// detecting future SIMD candidates.
pub mod runtime {
    /// A backend that can be reported by `base64-ng`.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    #[non_exhaustive]
    pub enum Backend {
        /// The audited scalar backend.
        Scalar,
        /// An AVX-512 VBMI candidate was detected.
        Avx512Vbmi,
        /// An AVX2 candidate was detected.
        Avx2,
        /// An SSSE3/SSE4.1 candidate was detected.
        Ssse3Sse41,
        /// An ARM NEON candidate was detected.
        Neon,
        /// A wasm `simd128` candidate was detected.
        WasmSimd128,
    }

    impl Backend {
        /// Returns the stable lowercase identifier for this backend.
        ///
        /// ```
        /// assert_eq!(base64_ng::runtime::Backend::Scalar.as_str(), "scalar");
        /// ```
        #[must_use]
        pub const fn as_str(self) -> &'static str {
            match self {
                Self::Scalar => "scalar",
                Self::Avx512Vbmi => "avx512-vbmi",
                Self::Avx2 => "avx2",
                Self::Ssse3Sse41 => "ssse3-sse4.1",
                Self::Neon => "neon",
                Self::WasmSimd128 => "wasm-simd128",
            }
        }

        /// Returns the CPU features required before this backend may be used.
        ///
        /// The active backend is still scalar-only. This method exists so
        /// security logs can record exactly which future backend feature bundle
        /// was detected.
        ///
        /// ```
        /// assert_eq!(
        ///     base64_ng::runtime::Backend::Avx512Vbmi.required_cpu_features(),
        ///     ["avx512f", "avx512bw", "avx512vl", "avx512vbmi"],
        /// );
        /// ```
        #[must_use]
        pub const fn required_cpu_features(self) -> &'static [&'static str] {
            match self {
                Self::Scalar => &[],
                Self::Avx512Vbmi => &["avx512f", "avx512bw", "avx512vl", "avx512vbmi"],
                Self::Avx2 => &["avx2"],
                Self::Ssse3Sse41 => &["ssse3", "sse4.1"],
                Self::Neon => &["neon"],
                Self::WasmSimd128 => &["simd128"],
            }
        }
    }

    impl core::fmt::Display for Backend {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            formatter.write_str(self.as_str())
        }
    }

    /// Security posture for the active runtime backend.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    #[non_exhaustive]
    pub enum SecurityPosture {
        /// No accelerated backend is active.
        ScalarOnly,
        /// SIMD support may be detected, but execution still uses scalar.
        SimdCandidateScalarActive,
        /// A SIMD backend is active.
        Accelerated,
    }

    impl SecurityPosture {
        /// Returns the stable lowercase identifier for this security posture.
        ///
        /// ```
        /// assert_eq!(
        ///     base64_ng::runtime::SecurityPosture::ScalarOnly.as_str(),
        ///     "scalar-only",
        /// );
        /// ```
        #[must_use]
        pub const fn as_str(self) -> &'static str {
            match self {
                Self::ScalarOnly => "scalar-only",
                Self::SimdCandidateScalarActive => "simd-candidate-scalar-active",
                Self::Accelerated => "accelerated",
            }
        }
    }

    impl core::fmt::Display for SecurityPosture {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            formatter.write_str(self.as_str())
        }
    }

    /// Deployment policy for runtime backend assertions.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    #[non_exhaustive]
    pub enum BackendPolicy {
        /// Require encode/decode execution to use the scalar backend.
        ScalarExecutionOnly,
        /// Require the crate to be built without the `simd` feature.
        SimdFeatureDisabled,
        /// Require no SIMD candidate to be visible to this build and target.
        NoDetectedSimdCandidate,
        /// Require scalar execution, the `simd` feature disabled, no detected
        /// SIMD candidate, and the unsafe boundary enforced.
        HighAssuranceScalarOnly,
    }

    impl BackendPolicy {
        /// Returns the stable lowercase identifier for this policy.
        ///
        /// ```
        /// assert_eq!(
        ///     base64_ng::runtime::BackendPolicy::HighAssuranceScalarOnly.as_str(),
        ///     "high-assurance-scalar-only",
        /// );
        /// ```
        #[must_use]
        pub const fn as_str(self) -> &'static str {
            match self {
                Self::ScalarExecutionOnly => "scalar-execution-only",
                Self::SimdFeatureDisabled => "simd-feature-disabled",
                Self::NoDetectedSimdCandidate => "no-detected-simd-candidate",
                Self::HighAssuranceScalarOnly => "high-assurance-scalar-only",
            }
        }
    }

    impl core::fmt::Display for BackendPolicy {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            formatter.write_str(self.as_str())
        }
    }

    /// Runtime backend policy failure.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct BackendPolicyError {
        /// Policy that was requested.
        pub policy: BackendPolicy,
        /// Backend report observed when the policy failed.
        pub report: BackendReport,
    }

    impl core::fmt::Display for BackendPolicyError {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(
                formatter,
                "runtime backend policy `{}` was not satisfied ({})",
                self.policy, self.report,
            )
        }
    }

    #[cfg(feature = "std")]
    impl std::error::Error for BackendPolicyError {}

    /// Backend report for the current build and target.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct BackendReport {
        /// Backend currently used for encode/decode dispatch.
        pub active: Backend,
        /// Strongest backend candidate visible to the current build.
        pub candidate: Backend,
        /// Whether the `simd` feature is enabled in this build.
        pub simd_feature_enabled: bool,
        /// Whether an accelerated SIMD backend is active.
        pub accelerated_backend_active: bool,
        /// Whether unsafe code is confined to the dedicated SIMD boundary.
        pub unsafe_boundary_enforced: bool,
        /// Current security posture.
        pub security_posture: SecurityPosture,
    }

    /// Compact structured backend snapshot for logging and policy evidence.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct BackendSnapshot {
        /// Stable active backend identifier.
        pub active: &'static str,
        /// Stable detected candidate identifier.
        pub candidate: &'static str,
        /// CPU features required by the detected candidate.
        pub candidate_required_cpu_features: &'static [&'static str],
        /// Whether the `simd` feature is enabled in this build.
        pub simd_feature_enabled: bool,
        /// Whether an accelerated SIMD backend is active.
        pub accelerated_backend_active: bool,
        /// Whether unsafe code is confined to the dedicated SIMD boundary.
        pub unsafe_boundary_enforced: bool,
        /// Stable security posture identifier.
        pub security_posture: &'static str,
    }

    impl core::fmt::Display for BackendReport {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(
                formatter,
                "active={} candidate={} candidate_required_cpu_features=",
                self.active, self.candidate,
            )?;
            write_feature_list(formatter, self.candidate_required_cpu_features())?;
            write!(
                formatter,
                " simd_feature_enabled={} accelerated_backend_active={} unsafe_boundary_enforced={} security_posture={}",
                self.simd_feature_enabled,
                self.accelerated_backend_active,
                self.unsafe_boundary_enforced,
                self.security_posture,
            )
        }
    }

    impl BackendReport {
        /// Returns whether this report satisfies `policy`.
        ///
        /// ```
        /// let report = base64_ng::runtime::backend_report();
        ///
        /// assert!(
        ///     report.satisfies(base64_ng::runtime::BackendPolicy::ScalarExecutionOnly)
        /// );
        /// ```
        #[must_use]
        pub const fn satisfies(self, policy: BackendPolicy) -> bool {
            match policy {
                BackendPolicy::ScalarExecutionOnly => {
                    matches!(self.active, Backend::Scalar) && !self.accelerated_backend_active
                }
                BackendPolicy::SimdFeatureDisabled => !self.simd_feature_enabled,
                BackendPolicy::NoDetectedSimdCandidate => matches!(self.candidate, Backend::Scalar),
                BackendPolicy::HighAssuranceScalarOnly => {
                    matches!(self.active, Backend::Scalar)
                        && matches!(self.candidate, Backend::Scalar)
                        && !self.simd_feature_enabled
                        && !self.accelerated_backend_active
                        && self.unsafe_boundary_enforced
                }
            }
        }

        /// Returns the CPU features required by the detected candidate.
        ///
        /// ```
        /// let report = base64_ng::runtime::backend_report();
        ///
        /// assert_eq!(
        ///     report.candidate_required_cpu_features(),
        ///     report.candidate.required_cpu_features(),
        /// );
        /// ```
        #[must_use]
        pub const fn candidate_required_cpu_features(self) -> &'static [&'static str] {
            self.candidate.required_cpu_features()
        }

        /// Returns a compact structured snapshot with stable string values.
        ///
        /// ```
        /// let snapshot = base64_ng::runtime::backend_report().snapshot();
        ///
        /// assert_eq!(snapshot.active, "scalar");
        /// assert!(!snapshot.accelerated_backend_active);
        /// ```
        #[must_use]
        pub const fn snapshot(self) -> BackendSnapshot {
            BackendSnapshot {
                active: self.active.as_str(),
                candidate: self.candidate.as_str(),
                candidate_required_cpu_features: self.candidate_required_cpu_features(),
                simd_feature_enabled: self.simd_feature_enabled,
                accelerated_backend_active: self.accelerated_backend_active,
                unsafe_boundary_enforced: self.unsafe_boundary_enforced,
                security_posture: self.security_posture.as_str(),
            }
        }
    }

    /// Returns the runtime backend report for this build and target.
    ///
    /// ```
    /// let report = base64_ng::runtime::backend_report();
    ///
    /// assert_eq!(report.active, base64_ng::runtime::Backend::Scalar);
    /// assert!(!report.accelerated_backend_active);
    /// ```
    #[must_use]
    pub fn backend_report() -> BackendReport {
        let active = active_backend();
        let candidate = detected_candidate();
        let accelerated_backend_active = active != Backend::Scalar;
        let security_posture = if accelerated_backend_active {
            SecurityPosture::Accelerated
        } else if candidate != Backend::Scalar {
            SecurityPosture::SimdCandidateScalarActive
        } else {
            SecurityPosture::ScalarOnly
        };

        BackendReport {
            active,
            candidate,
            simd_feature_enabled: cfg!(feature = "simd"),
            accelerated_backend_active,
            unsafe_boundary_enforced: true,
            security_posture,
        }
    }

    /// Requires the current runtime backend report to satisfy `policy`.
    ///
    /// ```
    /// base64_ng::runtime::require_backend_policy(
    ///     base64_ng::runtime::BackendPolicy::ScalarExecutionOnly,
    /// )
    /// .unwrap();
    /// ```
    pub fn require_backend_policy(policy: BackendPolicy) -> Result<(), BackendPolicyError> {
        let report = backend_report();
        if report.satisfies(policy) {
            Ok(())
        } else {
            Err(BackendPolicyError { policy, report })
        }
    }

    fn write_feature_list(
        formatter: &mut core::fmt::Formatter<'_>,
        features: &[&str],
    ) -> core::fmt::Result {
        formatter.write_str("[")?;
        let mut index = 0;
        while index < features.len() {
            if index != 0 {
                formatter.write_str(",")?;
            }
            formatter.write_str(features[index])?;
            index += 1;
        }
        formatter.write_str("]")
    }

    #[cfg(feature = "simd")]
    fn active_backend() -> Backend {
        match super::simd::active_backend() {
            super::simd::ActiveBackend::Scalar => Backend::Scalar,
        }
    }

    #[cfg(not(feature = "simd"))]
    const fn active_backend() -> Backend {
        Backend::Scalar
    }

    #[cfg(feature = "simd")]
    fn detected_candidate() -> Backend {
        match super::simd::detected_candidate() {
            super::simd::Candidate::Scalar => Backend::Scalar,
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            super::simd::Candidate::Avx512Vbmi => Backend::Avx512Vbmi,
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            super::simd::Candidate::Avx2 => Backend::Avx2,
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            super::simd::Candidate::Ssse3Sse41 => Backend::Ssse3Sse41,
            #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
            super::simd::Candidate::Neon => Backend::Neon,
            #[cfg(target_arch = "wasm32")]
            super::simd::Candidate::WasmSimd128 => Backend::WasmSimd128,
        }
    }

    #[cfg(not(feature = "simd"))]
    const fn detected_candidate() -> Backend {
        Backend::Scalar
    }
}

#[cfg(feature = "stream")]
pub mod stream {
    //! Streaming Base64 wrappers for `std::io`.
    //!
    //! ```
    //! use std::io::{Read, Write};
    //! use base64_ng::{STANDARD, stream::{Decoder, DecoderReader, Encoder, EncoderReader}};
    //!
    //! let mut encoder = Encoder::new(Vec::new(), STANDARD);
    //! encoder.write_all(b"he").unwrap();
    //! encoder.write_all(b"llo").unwrap();
    //! let encoded = encoder.finish().unwrap();
    //! assert_eq!(encoded, b"aGVsbG8=");
    //!
    //! let mut reader = EncoderReader::new(&b"hello"[..], STANDARD);
    //! let mut encoded = String::new();
    //! reader.read_to_string(&mut encoded).unwrap();
    //! assert_eq!(encoded, "aGVsbG8=");
    //!
    //! let mut decoder = Decoder::new(Vec::new(), STANDARD);
    //! decoder.write_all(b"aGVs").unwrap();
    //! decoder.write_all(b"bG8=").unwrap();
    //! let decoded = decoder.finish().unwrap();
    //! assert_eq!(decoded, b"hello");
    //!
    //! let mut reader = DecoderReader::new(&b"aGVsbG8="[..], STANDARD);
    //! let mut decoded = Vec::new();
    //! reader.read_to_end(&mut decoded).unwrap();
    //! assert_eq!(decoded, b"hello");
    //! ```

    use super::{Alphabet, DecodeError, EncodeError, Engine};
    use std::io::{self, Read, Write};

    struct OutputQueue<const CAP: usize> {
        buffer: [u8; CAP],
        start: usize,
        len: usize,
    }

    impl<const CAP: usize> OutputQueue<CAP> {
        const fn new() -> Self {
            Self {
                buffer: [0; CAP],
                start: 0,
                len: 0,
            }
        }

        const fn is_empty(&self) -> bool {
            self.len == 0
        }

        const fn len(&self) -> usize {
            self.len
        }

        const fn capacity(&self) -> usize {
            self.len + self.available_capacity()
        }

        fn push_slice(&mut self, input: &[u8]) -> io::Result<()> {
            if input.len() > self.available_capacity() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "base64 stream output queue capacity exceeded",
                ));
            }

            let mut read = 0;
            while read < input.len() {
                let write = (self.start + self.len) % CAP;
                self.buffer[write] = input[read];
                self.len += 1;
                read += 1;
            }

            Ok(())
        }

        fn copy_front(&self, output: &mut [u8]) -> usize {
            let count = core::cmp::min(self.len, output.len());
            let first = core::cmp::min(count, CAP - self.start);
            output[..first].copy_from_slice(&self.buffer[self.start..self.start + first]);

            let second = count - first;
            if second > 0 {
                output[first..first + second].copy_from_slice(&self.buffer[..second]);
            }

            count
        }

        fn discard_front(&mut self, count: usize) {
            let count = core::cmp::min(count, self.len);
            let first = core::cmp::min(count, CAP - self.start);
            crate::wipe_bytes(&mut self.buffer[self.start..self.start + first]);

            let second = count - first;
            if second > 0 {
                crate::wipe_bytes(&mut self.buffer[..second]);
            }

            self.start = (self.start + count) % CAP;
            self.len -= count;
            if self.len == 0 {
                self.start = 0;
            }
        }

        fn pop_slice(&mut self, output: &mut [u8]) -> usize {
            let count = self.copy_front(output);
            self.discard_front(count);
            count
        }

        fn clear_all(&mut self) {
            crate::wipe_bytes(&mut self.buffer);
            self.start = 0;
            self.len = 0;
        }

        const fn available_capacity(&self) -> usize {
            CAP - self.len
        }
    }

    /// A streaming Base64 encoder for `std::io::Write`.
    ///
    /// Like any [`Write`] implementation, [`Write::write`] may accept only
    /// part of the provided input. Accepted input may be held as encoded
    /// output until [`Write::flush`], [`Self::try_finish`], [`Self::finish`],
    /// or a later write drains the wrapped writer. Use [`Write::write_all`]
    /// when the whole input slice must be consumed.
    pub struct Encoder<W, A, const PAD: bool>
    where
        A: Alphabet,
    {
        inner: Option<W>,
        engine: Engine<A, PAD>,
        pending: [u8; 2],
        pending_len: usize,
        output: OutputQueue<1024>,
        finalized: bool,
    }

    impl<W, A, const PAD: bool> Encoder<W, A, PAD>
    where
        A: Alphabet,
    {
        /// Creates a new streaming encoder.
        #[must_use]
        pub const fn new(inner: W, engine: Engine<A, PAD>) -> Self {
            Self {
                inner: Some(inner),
                engine,
                pending: [0; 2],
                pending_len: 0,
                output: OutputQueue::new(),
                finalized: false,
            }
        }

        /// Returns a shared reference to the wrapped writer.
        #[must_use]
        pub fn get_ref(&self) -> &W {
            self.inner_ref()
        }

        /// Returns a mutable reference to the wrapped writer.
        pub fn get_mut(&mut self) -> &mut W {
            self.inner_mut()
        }

        /// Returns the Base64 engine used by this adapter.
        #[must_use]
        pub const fn engine(&self) -> Engine<A, PAD> {
            self.engine
        }

        /// Returns whether this adapter uses padded Base64.
        #[must_use]
        pub const fn is_padded(&self) -> bool {
            PAD
        }

        /// Returns the number of raw input bytes currently buffered until a
        /// complete 3-byte Base64 encode quantum is available.
        #[must_use]
        pub const fn pending_len(&self) -> usize {
            self.pending_len
        }

        /// Returns whether this encoder currently holds a partial input
        /// quantum.
        #[must_use]
        pub const fn has_pending_input(&self) -> bool {
            self.pending_len != 0
        }

        /// Returns how many additional input bytes are needed to complete the
        /// currently buffered encode quantum.
        ///
        /// Returns `0` when no partial input quantum is buffered.
        #[must_use]
        pub const fn pending_input_needed_len(&self) -> usize {
            if self.has_pending_input() {
                3 - self.pending_len
            } else {
                0
            }
        }

        /// Returns the number of encoded bytes buffered for the wrapped
        /// writer after a previous write or flush could not fully drain them.
        #[must_use]
        pub const fn buffered_output_len(&self) -> usize {
            self.output.len()
        }

        /// Returns the maximum number of encoded bytes this adapter can buffer
        /// before returning bytes to the caller.
        #[must_use]
        pub const fn buffered_output_capacity(&self) -> usize {
            self.output.capacity()
        }

        /// Returns how many more encoded bytes can be buffered before this
        /// adapter must drain the wrapped writer.
        #[must_use]
        pub const fn buffered_output_remaining_capacity(&self) -> usize {
            self.output.available_capacity()
        }

        /// Returns whether this encoder has encoded output waiting to be
        /// written to the wrapped writer.
        #[must_use]
        pub const fn has_buffered_output(&self) -> bool {
            !self.output.is_empty()
        }

        /// Returns whether this encoder has been finalized.
        ///
        /// Once this returns `true`, later non-empty writes return an error.
        #[must_use]
        pub const fn is_finalized(&self) -> bool {
            self.finalized
        }

        /// Returns whether [`Self::try_into_inner`] can recover the wrapped
        /// writer without discarding pending input.
        #[must_use]
        pub const fn can_into_inner(&self) -> bool {
            !self.has_pending_input() && !self.has_buffered_output()
        }

        /// Consumes the encoder without flushing pending input.
        ///
        /// Prefer [`Self::finish`] when the encoded output must be complete.
        #[must_use]
        pub fn into_inner(mut self) -> W {
            self.take_inner()
        }

        /// Consumes the encoder only when no partial input quantum is buffered.
        ///
        /// This does not flush or finalize the wrapped writer. It is a checked
        /// alternative to [`Self::into_inner`] for callers that want to avoid
        /// accidentally discarding pending input bytes.
        #[allow(clippy::result_large_err)]
        pub fn try_into_inner(mut self) -> Result<W, Self> {
            if !self.can_into_inner() {
                return Err(self);
            }
            Ok(self.take_inner())
        }

        fn inner_ref(&self) -> &W {
            match &self.inner {
                Some(inner) => inner,
                None => unreachable!("stream encoder inner writer was already taken"),
            }
        }

        fn inner_mut(&mut self) -> &mut W {
            match &mut self.inner {
                Some(inner) => inner,
                None => unreachable!("stream encoder inner writer was already taken"),
            }
        }

        fn take_inner(&mut self) -> W {
            match self.inner.take() {
                Some(inner) => inner,
                None => unreachable!("stream encoder inner writer was already taken"),
            }
        }

        fn clear_pending(&mut self) {
            crate::wipe_bytes(&mut self.pending);
            self.pending_len = 0;
        }

        fn clear_output(&mut self) {
            self.output.clear_all();
        }
    }

    impl<W, A, const PAD: bool> Drop for Encoder<W, A, PAD>
    where
        A: Alphabet,
    {
        fn drop(&mut self) {
            self.clear_pending();
            self.clear_output();
        }
    }

    impl<W, A, const PAD: bool> core::fmt::Debug for Encoder<W, A, PAD>
    where
        A: Alphabet,
    {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            formatter
                .debug_struct("Encoder")
                .field("inner", &redacted_inner_state(self.inner.is_some()))
                .field("engine", &self.engine)
                .field("pending", &"<redacted>")
                .field("pending_len", &self.pending_len)
                .field("pending_input_needed_len", &self.pending_input_needed_len())
                .field("buffered_output_len", &self.output.len())
                .field("buffered_output_capacity", &self.output.capacity())
                .field(
                    "buffered_output_remaining_capacity",
                    &self.output.available_capacity(),
                )
                .field("can_into_inner", &self.can_into_inner())
                .field("finalized", &self.finalized)
                .finish()
        }
    }

    impl<W, A, const PAD: bool> Encoder<W, A, PAD>
    where
        W: Write,
        A: Alphabet,
    {
        /// Writes any pending input and flushes the wrapped writer without
        /// consuming this encoder.
        ///
        /// After this succeeds, [`Self::pending_len`] returns `0`, later
        /// writes are rejected, and [`Self::finish`] can still be used to
        /// recover the wrapped writer.
        /// This is useful when a caller needs to finalize a framed payload
        /// while keeping the stream adapter available for diagnostics or
        /// explicit recovery.
        pub fn try_finish(&mut self) -> io::Result<()> {
            if !self.finalized {
                self.queue_pending_final()?;
                self.finalized = true;
            }
            self.flush()
        }

        /// Writes any pending input, flushes the wrapped writer, and returns it.
        pub fn finish(mut self) -> io::Result<W> {
            self.try_finish()?;
            Ok(self.take_inner())
        }

        fn queue_pending_final(&mut self) -> io::Result<()> {
            if self.pending_len == 0 {
                return Ok(());
            }

            let mut pending = [0u8; 2];
            pending[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
            let pending_len = self.pending_len;
            let mut encoded = [0u8; 4];
            let result = self.queue_encoded_temp(&pending[..pending_len], &mut encoded);
            crate::wipe_bytes(&mut pending);
            result?;
            self.clear_pending();
            Ok(())
        }

        fn queue_encoded_temp(&mut self, input: &[u8], encoded: &mut [u8]) -> io::Result<()> {
            let written = match self.engine.encode_slice(input, encoded) {
                Ok(written) => written,
                Err(err) => {
                    crate::wipe_bytes(encoded);
                    return Err(encode_error_to_io(err));
                }
            };

            let result = self.output.push_slice(&encoded[..written]);
            crate::wipe_bytes(encoded);
            result
        }

        fn drain_output(&mut self) -> io::Result<()> {
            let mut chunk = [0u8; 1024];
            while !self.output.is_empty() {
                let pending = self.output.copy_front(&mut chunk);
                let result = self.inner_mut().write(&chunk[..pending]);
                crate::wipe_bytes(&mut chunk[..pending]);
                match result {
                    Ok(0) => {
                        return Err(io::Error::new(
                            io::ErrorKind::WriteZero,
                            "base64 stream encoder could not drain buffered output",
                        ));
                    }
                    Ok(written) => {
                        if written > pending {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "wrapped writer reported more bytes than provided",
                            ));
                        }
                        self.output.discard_front(written);
                    }
                    Err(err) => return Err(err),
                }
            }

            Ok(())
        }
    }

    impl<W, A, const PAD: bool> Write for Encoder<W, A, PAD>
    where
        W: Write,
        A: Alphabet,
    {
        fn write(&mut self, input: &[u8]) -> io::Result<usize> {
            if input.is_empty() {
                self.drain_output()?;
                return Ok(0);
            }
            self.drain_output()?;
            if self.finalized {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "base64 stream encoder received input after finalization",
                ));
            }

            let mut consumed = 0;
            if self.pending_len > 0 {
                let needed = 3 - self.pending_len;
                if input.len() < needed {
                    self.pending[self.pending_len..self.pending_len + input.len()]
                        .copy_from_slice(input);
                    self.pending_len += input.len();
                    return Ok(input.len());
                }

                let mut chunk = [0u8; 3];
                chunk[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
                chunk[self.pending_len..].copy_from_slice(&input[..needed]);

                let mut encoded = [0u8; 4];
                let result = self.queue_encoded_temp(&chunk, &mut encoded);
                crate::wipe_bytes(&mut chunk);
                result?;
                self.clear_pending();
                consumed += needed;
                return Ok(consumed);
            }

            let remaining = &input[consumed..];
            let full_len = remaining.len() / 3 * 3;
            if full_len > 0 {
                let mut take = core::cmp::min(full_len, 768);
                take -= take % 3;
                debug_assert!(take > 0);

                let mut encoded = [0u8; 1024];
                self.queue_encoded_temp(&remaining[..take], &mut encoded)?;
                return Ok(consumed + take);
            }

            let tail = &remaining[full_len..];
            self.pending[..tail.len()].copy_from_slice(tail);
            self.pending_len = tail.len();

            Ok(input.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            self.drain_output()?;
            self.inner_mut().flush()
        }
    }

    fn encode_error_to_io(err: EncodeError) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidInput, err)
    }

    /// A streaming Base64 decoder for `std::io::Write`.
    ///
    /// Like any [`Write`] implementation, [`Write::write`] may accept only
    /// part of the provided input. Accepted input may be held as decoded
    /// output until [`Write::flush`], [`Self::try_finish`], [`Self::finish`],
    /// or a later write drains the wrapped writer. Use [`Write::write_all`]
    /// when the whole input slice must be consumed.
    pub struct Decoder<W, A, const PAD: bool>
    where
        A: Alphabet,
    {
        inner: Option<W>,
        engine: Engine<A, PAD>,
        pending: [u8; 4],
        pending_len: usize,
        output: OutputQueue<1024>,
        finished: bool,
        finalized: bool,
    }

    impl<W, A, const PAD: bool> Decoder<W, A, PAD>
    where
        A: Alphabet,
    {
        /// Creates a new streaming decoder.
        #[must_use]
        pub const fn new(inner: W, engine: Engine<A, PAD>) -> Self {
            Self {
                inner: Some(inner),
                engine,
                pending: [0; 4],
                pending_len: 0,
                output: OutputQueue::new(),
                finished: false,
                finalized: false,
            }
        }

        /// Returns a shared reference to the wrapped writer.
        #[must_use]
        pub fn get_ref(&self) -> &W {
            self.inner_ref()
        }

        /// Returns a mutable reference to the wrapped writer.
        pub fn get_mut(&mut self) -> &mut W {
            self.inner_mut()
        }

        /// Returns the Base64 engine used by this adapter.
        #[must_use]
        pub const fn engine(&self) -> Engine<A, PAD> {
            self.engine
        }

        /// Returns whether this adapter uses padded Base64.
        #[must_use]
        pub const fn is_padded(&self) -> bool {
            PAD
        }

        /// Returns the number of encoded input bytes currently buffered until
        /// a complete 4-byte Base64 decode quantum is available.
        #[must_use]
        pub const fn pending_len(&self) -> usize {
            self.pending_len
        }

        /// Returns whether this decoder currently holds a partial input
        /// quantum.
        #[must_use]
        pub const fn has_pending_input(&self) -> bool {
            self.pending_len != 0
        }

        /// Returns how many additional input bytes are needed to complete the
        /// currently buffered decode quantum.
        ///
        /// Returns `0` when no partial input quantum is buffered.
        #[must_use]
        pub const fn pending_input_needed_len(&self) -> usize {
            if self.has_pending_input() {
                4 - self.pending_len
            } else {
                0
            }
        }

        /// Returns the number of decoded bytes buffered for the wrapped writer
        /// after a previous write or flush could not fully drain them.
        #[must_use]
        pub const fn buffered_output_len(&self) -> usize {
            self.output.len()
        }

        /// Returns the maximum number of decoded bytes this adapter can buffer
        /// before returning bytes to the caller.
        #[must_use]
        pub const fn buffered_output_capacity(&self) -> usize {
            self.output.capacity()
        }

        /// Returns how many more decoded bytes can be buffered before this
        /// adapter must drain the wrapped writer.
        #[must_use]
        pub const fn buffered_output_remaining_capacity(&self) -> usize {
            self.output.available_capacity()
        }

        /// Returns whether this decoder has decoded output waiting to be
        /// written to the wrapped writer.
        #[must_use]
        pub const fn has_buffered_output(&self) -> bool {
            !self.output.is_empty()
        }

        /// Returns whether this decoder has processed a terminal padded block.
        ///
        /// Once this returns `true`, later calls to [`Write::write`] with
        /// additional input return an error because strict Base64 does not
        /// permit trailing payload bytes after padding.
        #[must_use]
        pub const fn has_terminal_padding(&self) -> bool {
            self.finished
        }

        /// Returns whether this decoder has been finalized.
        ///
        /// Once this returns `true`, later non-empty writes return an error.
        #[must_use]
        pub const fn is_finalized(&self) -> bool {
            self.finalized
        }

        /// Returns whether [`Self::try_into_inner`] can recover the wrapped
        /// writer without discarding pending encoded input.
        #[must_use]
        pub const fn can_into_inner(&self) -> bool {
            !self.has_pending_input() && !self.has_buffered_output()
        }

        /// Consumes the decoder without flushing pending input.
        ///
        /// Prefer [`Self::finish`] when the decoded output must be complete.
        #[must_use]
        pub fn into_inner(mut self) -> W {
            self.take_inner()
        }

        /// Consumes the decoder only when no partial input quantum is buffered.
        ///
        /// This does not flush or finalize the wrapped writer. It is a checked
        /// alternative to [`Self::into_inner`] for callers that want to avoid
        /// accidentally discarding pending encoded input bytes.
        #[allow(clippy::result_large_err)]
        pub fn try_into_inner(mut self) -> Result<W, Self> {
            if !self.can_into_inner() {
                return Err(self);
            }
            Ok(self.take_inner())
        }

        fn inner_ref(&self) -> &W {
            match &self.inner {
                Some(inner) => inner,
                None => unreachable!("stream decoder inner writer was already taken"),
            }
        }

        fn inner_mut(&mut self) -> &mut W {
            match &mut self.inner {
                Some(inner) => inner,
                None => unreachable!("stream decoder inner writer was already taken"),
            }
        }

        fn take_inner(&mut self) -> W {
            match self.inner.take() {
                Some(inner) => inner,
                None => unreachable!("stream decoder inner writer was already taken"),
            }
        }

        fn clear_pending(&mut self) {
            crate::wipe_bytes(&mut self.pending);
            self.pending_len = 0;
        }

        fn clear_output(&mut self) {
            self.output.clear_all();
        }
    }

    impl<W, A, const PAD: bool> Drop for Decoder<W, A, PAD>
    where
        A: Alphabet,
    {
        fn drop(&mut self) {
            self.clear_pending();
            self.clear_output();
        }
    }

    impl<W, A, const PAD: bool> core::fmt::Debug for Decoder<W, A, PAD>
    where
        A: Alphabet,
    {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            formatter
                .debug_struct("Decoder")
                .field("inner", &redacted_inner_state(self.inner.is_some()))
                .field("engine", &self.engine)
                .field("pending", &"<redacted>")
                .field("pending_len", &self.pending_len)
                .field("pending_input_needed_len", &self.pending_input_needed_len())
                .field("buffered_output_len", &self.output.len())
                .field("buffered_output_capacity", &self.output.capacity())
                .field(
                    "buffered_output_remaining_capacity",
                    &self.output.available_capacity(),
                )
                .field("can_into_inner", &self.can_into_inner())
                .field("terminal_padding", &self.finished)
                .field("finalized", &self.finalized)
                .finish()
        }
    }

    impl<W, A, const PAD: bool> Decoder<W, A, PAD>
    where
        W: Write,
        A: Alphabet,
    {
        /// Validates any final pending input and flushes the wrapped writer
        /// without consuming this decoder.
        ///
        /// After this succeeds, [`Self::pending_len`] returns `0`, later
        /// writes are rejected, and [`Self::finish`] can still be used to
        /// recover the wrapped writer.
        /// If the final buffered input is malformed, an error is returned and
        /// the caller still owns the decoder for diagnostics or explicit
        /// recovery.
        pub fn try_finish(&mut self) -> io::Result<()> {
            if !self.finalized {
                self.queue_pending_final()?;
                self.finalized = true;
            }
            self.flush()
        }

        /// Validates final pending input, flushes the wrapped writer, and returns it.
        pub fn finish(mut self) -> io::Result<W> {
            self.try_finish()?;
            Ok(self.take_inner())
        }

        fn queue_pending_final(&mut self) -> io::Result<()> {
            if self.pending_len == 0 {
                return Ok(());
            }

            let mut pending = [0u8; 4];
            pending[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
            let pending_len = self.pending_len;
            let mut decoded = [0u8; 3];
            let result = self.queue_decoded_temp(&pending[..pending_len], &mut decoded);
            crate::wipe_bytes(&mut pending);
            result?;
            self.clear_pending();
            Ok(())
        }

        fn queue_full_quad(&mut self, mut input: [u8; 4]) -> io::Result<()> {
            let mut decoded = [0u8; 3];
            let result = self.queue_decoded_temp(&input, &mut decoded);
            crate::wipe_bytes(&mut input);
            let written = result?;
            if written < 3 {
                self.finished = true;
            }
            Ok(())
        }

        fn queue_decoded_temp(&mut self, input: &[u8], decoded: &mut [u8]) -> io::Result<usize> {
            let written = match self.engine.decode_slice(input, decoded) {
                Ok(written) => written,
                Err(err) => {
                    crate::wipe_bytes(decoded);
                    return Err(decode_error_to_io(err));
                }
            };

            let result = self.output.push_slice(&decoded[..written]);
            crate::wipe_bytes(decoded);
            result?;
            Ok(written)
        }

        fn drain_output(&mut self) -> io::Result<()> {
            let mut chunk = [0u8; 1024];
            while !self.output.is_empty() {
                let pending = self.output.copy_front(&mut chunk);
                let result = self.inner_mut().write(&chunk[..pending]);
                crate::wipe_bytes(&mut chunk[..pending]);
                match result {
                    Ok(0) => {
                        return Err(io::Error::new(
                            io::ErrorKind::WriteZero,
                            "base64 stream decoder could not drain buffered output",
                        ));
                    }
                    Ok(written) => {
                        if written > pending {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "wrapped writer reported more bytes than provided",
                            ));
                        }
                        self.output.discard_front(written);
                    }
                    Err(err) => return Err(err),
                }
            }

            Ok(())
        }
    }

    impl<W, A, const PAD: bool> Write for Decoder<W, A, PAD>
    where
        W: Write,
        A: Alphabet,
    {
        fn write(&mut self, input: &[u8]) -> io::Result<usize> {
            if input.is_empty() {
                self.drain_output()?;
                return Ok(0);
            }
            self.drain_output()?;
            if self.finalized {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "base64 stream decoder received input after finalization",
                ));
            }
            if self.finished {
                return Err(trailing_input_after_padding_error());
            }

            let mut consumed = 0;
            if self.pending_len > 0 {
                let needed = 4 - self.pending_len;
                if input.len() < needed {
                    self.pending[self.pending_len..self.pending_len + input.len()]
                        .copy_from_slice(input);
                    self.pending_len += input.len();
                    return Ok(input.len());
                }

                let mut quad = [0u8; 4];
                quad[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
                quad[self.pending_len..].copy_from_slice(&input[..needed]);
                let result = self.queue_full_quad(quad);
                crate::wipe_bytes(&mut quad);
                result?;
                self.clear_pending();
                consumed += needed;
                return Ok(consumed);
            }

            let remaining = &input[consumed..];
            let full_len = remaining.len() / 4 * 4;
            if full_len > 0 {
                let quad = [remaining[0], remaining[1], remaining[2], remaining[3]];
                let mut quad = quad;
                let result = self.queue_full_quad(quad);
                crate::wipe_bytes(&mut quad);
                result?;
                return Ok(consumed + 4);
            }

            let tail = &remaining[full_len..];
            self.pending[..tail.len()].copy_from_slice(tail);
            self.pending_len = tail.len();

            Ok(input.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            self.drain_output()?;
            self.inner_mut().flush()
        }
    }

    fn decode_error_to_io(err: DecodeError) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidInput, err)
    }

    fn trailing_input_after_padding_error() -> io::Error {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "base64 decoder received trailing input after padding",
        )
    }

    /// A streaming Base64 decoder for `std::io::Read`.
    ///
    /// For padded engines, this reader stops at the terminal padded Base64
    /// block and leaves later bytes unread in the wrapped reader. This preserves
    /// boundaries for callers that decode one Base64 payload from a larger
    /// stream.
    pub struct DecoderReader<R, A, const PAD: bool>
    where
        A: Alphabet,
    {
        inner: Option<R>,
        engine: Engine<A, PAD>,
        pending: [u8; 4],
        pending_len: usize,
        output: OutputQueue<3>,
        finished: bool,
        terminal_seen: bool,
    }

    impl<R, A, const PAD: bool> DecoderReader<R, A, PAD>
    where
        A: Alphabet,
    {
        /// Creates a new streaming decoder reader.
        #[must_use]
        pub fn new(inner: R, engine: Engine<A, PAD>) -> Self {
            Self {
                inner: Some(inner),
                engine,
                pending: [0; 4],
                pending_len: 0,
                output: OutputQueue::new(),
                finished: false,
                terminal_seen: false,
            }
        }

        /// Returns a shared reference to the wrapped reader.
        #[must_use]
        pub fn get_ref(&self) -> &R {
            self.inner_ref()
        }

        /// Returns a mutable reference to the wrapped reader.
        pub fn get_mut(&mut self) -> &mut R {
            self.inner_mut()
        }

        /// Returns the Base64 engine used by this adapter.
        #[must_use]
        pub const fn engine(&self) -> Engine<A, PAD> {
            self.engine
        }

        /// Returns whether this adapter uses padded Base64.
        #[must_use]
        pub const fn is_padded(&self) -> bool {
            PAD
        }

        /// Returns the number of encoded input bytes currently buffered until
        /// a complete 4-byte Base64 decode quantum is available.
        #[must_use]
        pub const fn pending_len(&self) -> usize {
            self.pending_len
        }

        /// Returns whether this decoder reader currently holds a partial input
        /// quantum.
        #[must_use]
        pub const fn has_pending_input(&self) -> bool {
            self.pending_len != 0
        }

        /// Returns how many additional encoded input bytes are needed to
        /// complete the currently buffered decode quantum.
        ///
        /// Returns `0` when no partial input quantum is buffered.
        #[must_use]
        pub const fn pending_input_needed_len(&self) -> usize {
            if self.has_pending_input() {
                4 - self.pending_len
            } else {
                0
            }
        }

        /// Returns the number of decoded bytes currently buffered and ready to
        /// be read before this adapter polls the wrapped reader again.
        #[must_use]
        pub const fn buffered_output_len(&self) -> usize {
            self.output.len()
        }

        /// Returns the maximum number of decoded bytes this adapter can buffer
        /// before returning bytes to the caller.
        #[must_use]
        pub const fn buffered_output_capacity(&self) -> usize {
            self.output.capacity()
        }

        /// Returns how many more decoded bytes can be buffered before this
        /// adapter must return bytes to the caller.
        #[must_use]
        pub const fn buffered_output_remaining_capacity(&self) -> usize {
            self.output.available_capacity()
        }

        /// Returns whether this decoder reader currently has decoded output
        /// waiting in its internal queue.
        #[must_use]
        pub const fn has_buffered_output(&self) -> bool {
            !self.output.is_empty()
        }

        /// Returns whether this decoder reader has seen terminal padding.
        ///
        /// For padded engines, this becomes `true` after the terminal padded
        /// block is decoded. The wrapped reader is then left positioned after
        /// that Base64 block so adjacent framed bytes can be read by the
        /// caller.
        #[must_use]
        pub const fn has_terminal_padding(&self) -> bool {
            self.terminal_seen
        }

        /// Returns whether this decoder reader has reached EOF or terminal
        /// padding in the wrapped reader.
        ///
        /// This may become `true` before [`Self::is_finished`] when decoded
        /// output is still buffered for the caller.
        #[must_use]
        pub const fn has_finished_input(&self) -> bool {
            self.finished
        }

        /// Returns whether this reader has reached EOF or terminal padding
        /// and has no decoded output buffered for the caller.
        #[must_use]
        pub const fn is_finished(&self) -> bool {
            self.finished && self.output.is_empty()
        }

        /// Returns whether [`Self::try_into_inner`] can recover the wrapped
        /// reader without discarding buffered decoded output.
        #[must_use]
        pub const fn can_into_inner(&self) -> bool {
            self.is_finished()
        }

        /// Consumes the decoder reader and returns the wrapped reader.
        #[must_use]
        pub fn into_inner(mut self) -> R {
            self.take_inner()
        }

        /// Consumes the decoder reader only after the Base64 payload is fully
        /// drained.
        ///
        /// For padded streams, terminal padding may leave adjacent framed bytes
        /// unread in the wrapped reader. This method succeeds only after all
        /// decoded output buffered by this adapter has been read, so recovering
        /// the wrapped reader does not silently discard decoded bytes.
        #[allow(clippy::result_large_err)]
        pub fn try_into_inner(mut self) -> Result<R, Self> {
            if !self.can_into_inner() {
                return Err(self);
            }
            Ok(self.take_inner())
        }

        fn inner_ref(&self) -> &R {
            match &self.inner {
                Some(inner) => inner,
                None => unreachable!("stream decoder reader inner reader was already taken"),
            }
        }

        fn inner_mut(&mut self) -> &mut R {
            match &mut self.inner {
                Some(inner) => inner,
                None => unreachable!("stream decoder reader inner reader was already taken"),
            }
        }

        fn take_inner(&mut self) -> R {
            match self.inner.take() {
                Some(inner) => inner,
                None => unreachable!("stream decoder reader inner reader was already taken"),
            }
        }

        fn clear_pending(&mut self) {
            crate::wipe_bytes(&mut self.pending);
            self.pending_len = 0;
        }
    }

    impl<R, A, const PAD: bool> Drop for DecoderReader<R, A, PAD>
    where
        A: Alphabet,
    {
        fn drop(&mut self) {
            self.clear_pending();
            self.output.clear_all();
        }
    }

    impl<R, A, const PAD: bool> core::fmt::Debug for DecoderReader<R, A, PAD>
    where
        A: Alphabet,
    {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            formatter
                .debug_struct("DecoderReader")
                .field("inner", &redacted_inner_state(self.inner.is_some()))
                .field("engine", &self.engine)
                .field("pending", &"<redacted>")
                .field("pending_len", &self.pending_len)
                .field("pending_input_needed_len", &self.pending_input_needed_len())
                .field("buffered_output_len", &self.output.len())
                .field("buffered_output_capacity", &self.output.capacity())
                .field(
                    "buffered_output_remaining_capacity",
                    &self.output.available_capacity(),
                )
                .field("can_into_inner", &self.can_into_inner())
                .field("finished", &self.finished)
                .field("terminal_padding", &self.terminal_seen)
                .finish()
        }
    }

    impl<R, A, const PAD: bool> Read for DecoderReader<R, A, PAD>
    where
        R: Read,
        A: Alphabet,
    {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            if output.is_empty() {
                return Ok(0);
            }

            while self.output.is_empty() && !self.finished {
                self.fill_output()?;
            }

            Ok(self.output.pop_slice(output))
        }
    }

    impl<R, A, const PAD: bool> DecoderReader<R, A, PAD>
    where
        R: Read,
        A: Alphabet,
    {
        fn fill_output(&mut self) -> io::Result<()> {
            if self.terminal_seen {
                self.finished = true;
                return Ok(());
            }

            let mut input = [0u8; 4];
            let available = 4 - self.pending_len;
            let read = self.inner_mut().read(&mut input[..available])?;
            if read == 0 {
                crate::wipe_bytes(&mut input);
                self.finished = true;
                self.push_final_pending()?;
                return Ok(());
            }

            self.pending[self.pending_len..self.pending_len + read].copy_from_slice(&input[..read]);
            crate::wipe_bytes(&mut input);
            self.pending_len += read;
            if self.pending_len < 4 {
                return Ok(());
            }

            let mut quad = self.pending;
            self.clear_pending();
            let result = self.push_decoded(&quad);
            crate::wipe_bytes(&mut quad);
            result?;
            if self.terminal_seen {
                self.finished = true;
            }
            Ok(())
        }

        fn push_final_pending(&mut self) -> io::Result<()> {
            if self.pending_len == 0 {
                return Ok(());
            }

            let mut pending = [0u8; 4];
            pending[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
            let pending_len = self.pending_len;
            self.clear_pending();
            let result = self.push_decoded(&pending[..pending_len]);
            crate::wipe_bytes(&mut pending);
            result
        }

        fn push_decoded(&mut self, input: &[u8]) -> io::Result<()> {
            let mut decoded = [0u8; 3];
            let written = match self.engine.decode_slice(input, &mut decoded) {
                Ok(written) => written,
                Err(err) => {
                    crate::wipe_bytes(&mut decoded);
                    return Err(decode_error_to_io(err));
                }
            };
            let result = self.output.push_slice(&decoded[..written]);
            crate::wipe_bytes(&mut decoded);
            result?;
            if input.len() == 4 && written < 3 {
                self.terminal_seen = true;
            }
            Ok(())
        }
    }

    /// A streaming Base64 encoder for `std::io::Read`.
    pub struct EncoderReader<R, A, const PAD: bool>
    where
        A: Alphabet,
    {
        inner: Option<R>,
        engine: Engine<A, PAD>,
        pending: [u8; 2],
        pending_len: usize,
        output: OutputQueue<1024>,
        finished: bool,
    }

    impl<R, A, const PAD: bool> EncoderReader<R, A, PAD>
    where
        A: Alphabet,
    {
        /// Creates a new streaming encoder reader.
        #[must_use]
        pub fn new(inner: R, engine: Engine<A, PAD>) -> Self {
            Self {
                inner: Some(inner),
                engine,
                pending: [0; 2],
                pending_len: 0,
                output: OutputQueue::new(),
                finished: false,
            }
        }

        /// Returns a shared reference to the wrapped reader.
        #[must_use]
        pub fn get_ref(&self) -> &R {
            self.inner_ref()
        }

        /// Returns a mutable reference to the wrapped reader.
        pub fn get_mut(&mut self) -> &mut R {
            self.inner_mut()
        }

        /// Returns the Base64 engine used by this adapter.
        #[must_use]
        pub const fn engine(&self) -> Engine<A, PAD> {
            self.engine
        }

        /// Returns whether this adapter uses padded Base64.
        #[must_use]
        pub const fn is_padded(&self) -> bool {
            PAD
        }

        /// Returns the number of raw input bytes currently buffered until a
        /// complete 3-byte Base64 encode quantum is available.
        #[must_use]
        pub const fn pending_len(&self) -> usize {
            self.pending_len
        }

        /// Returns whether this encoder reader currently holds a partial input
        /// quantum.
        #[must_use]
        pub const fn has_pending_input(&self) -> bool {
            self.pending_len != 0
        }

        /// Returns how many additional raw input bytes are needed to complete
        /// the currently buffered encode quantum.
        ///
        /// Returns `0` when no partial input quantum is buffered.
        #[must_use]
        pub const fn pending_input_needed_len(&self) -> usize {
            if self.has_pending_input() {
                3 - self.pending_len
            } else {
                0
            }
        }

        /// Returns the number of encoded bytes currently buffered and ready to
        /// be read before this adapter polls the wrapped reader again.
        #[must_use]
        pub const fn buffered_output_len(&self) -> usize {
            self.output.len()
        }

        /// Returns the maximum number of encoded bytes this adapter can buffer
        /// before returning bytes to the caller.
        #[must_use]
        pub const fn buffered_output_capacity(&self) -> usize {
            self.output.capacity()
        }

        /// Returns how many more encoded bytes can be buffered before this
        /// adapter must return bytes to the caller.
        #[must_use]
        pub const fn buffered_output_remaining_capacity(&self) -> usize {
            self.output.available_capacity()
        }

        /// Returns whether this encoder reader currently has encoded output
        /// waiting in its internal queue.
        #[must_use]
        pub const fn has_buffered_output(&self) -> bool {
            !self.output.is_empty()
        }

        /// Returns whether this encoder reader has reached EOF in the wrapped
        /// reader.
        ///
        /// This may become `true` before [`Self::is_finished`] when encoded
        /// output is still buffered for the caller.
        #[must_use]
        pub const fn has_finished_input(&self) -> bool {
            self.finished
        }

        /// Returns whether this reader has reached EOF and has no encoded
        /// output buffered for the caller.
        #[must_use]
        pub const fn is_finished(&self) -> bool {
            self.finished && self.output.is_empty()
        }

        /// Returns whether [`Self::try_into_inner`] can recover the wrapped
        /// reader without discarding pending input or buffered encoded output.
        #[must_use]
        pub const fn can_into_inner(&self) -> bool {
            self.is_finished()
        }

        /// Consumes the encoder reader and returns the wrapped reader.
        #[must_use]
        pub fn into_inner(mut self) -> R {
            self.take_inner()
        }

        /// Consumes the encoder reader only after the encoded stream is fully
        /// drained.
        ///
        /// This is a checked alternative to [`Self::into_inner`] for callers
        /// that want to avoid accidentally discarding pending input or encoded
        /// output buffered inside the adapter.
        #[allow(clippy::result_large_err)]
        pub fn try_into_inner(mut self) -> Result<R, Self> {
            if !self.can_into_inner() {
                return Err(self);
            }
            Ok(self.take_inner())
        }

        fn inner_ref(&self) -> &R {
            match &self.inner {
                Some(inner) => inner,
                None => unreachable!("stream encoder reader inner reader was already taken"),
            }
        }

        fn inner_mut(&mut self) -> &mut R {
            match &mut self.inner {
                Some(inner) => inner,
                None => unreachable!("stream encoder reader inner reader was already taken"),
            }
        }

        fn take_inner(&mut self) -> R {
            match self.inner.take() {
                Some(inner) => inner,
                None => unreachable!("stream encoder reader inner reader was already taken"),
            }
        }

        fn clear_pending(&mut self) {
            crate::wipe_bytes(&mut self.pending);
            self.pending_len = 0;
        }
    }

    impl<R, A, const PAD: bool> Drop for EncoderReader<R, A, PAD>
    where
        A: Alphabet,
    {
        fn drop(&mut self) {
            self.clear_pending();
            self.output.clear_all();
        }
    }

    impl<R, A, const PAD: bool> core::fmt::Debug for EncoderReader<R, A, PAD>
    where
        A: Alphabet,
    {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            formatter
                .debug_struct("EncoderReader")
                .field("inner", &redacted_inner_state(self.inner.is_some()))
                .field("engine", &self.engine)
                .field("pending", &"<redacted>")
                .field("pending_len", &self.pending_len)
                .field("pending_input_needed_len", &self.pending_input_needed_len())
                .field("buffered_output_len", &self.output.len())
                .field("buffered_output_capacity", &self.output.capacity())
                .field(
                    "buffered_output_remaining_capacity",
                    &self.output.available_capacity(),
                )
                .field("can_into_inner", &self.can_into_inner())
                .field("finished", &self.finished)
                .finish()
        }
    }

    impl<R, A, const PAD: bool> Read for EncoderReader<R, A, PAD>
    where
        R: Read,
        A: Alphabet,
    {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            if output.is_empty() {
                return Ok(0);
            }

            while self.output.is_empty() && !self.finished {
                self.fill_output()?;
            }

            Ok(self.output.pop_slice(output))
        }
    }

    impl<R, A, const PAD: bool> EncoderReader<R, A, PAD>
    where
        R: Read,
        A: Alphabet,
    {
        fn fill_output(&mut self) -> io::Result<()> {
            let mut input = [0u8; 768];
            let read = self.inner_mut().read(&mut input)?;
            if read == 0 {
                crate::wipe_bytes(&mut input);
                self.finished = true;
                self.push_final_pending()?;
                return Ok(());
            }

            let mut consumed = 0;
            if self.pending_len > 0 {
                let needed = 3 - self.pending_len;
                if read < needed {
                    self.pending[self.pending_len..self.pending_len + read]
                        .copy_from_slice(&input[..read]);
                    self.pending_len += read;
                    crate::wipe_bytes(&mut input);
                    return Ok(());
                }

                let mut chunk = [0u8; 3];
                chunk[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
                chunk[self.pending_len..].copy_from_slice(&input[..needed]);
                let result = self.push_encoded(&chunk);
                crate::wipe_bytes(&mut chunk);
                if let Err(err) = result {
                    crate::wipe_bytes(&mut input);
                    return Err(err);
                }
                self.clear_pending();
                consumed += needed;
            }

            let remaining = &input[consumed..read];
            let full_len = remaining.len() / 3 * 3;
            let tail_len = remaining.len() - full_len;
            let mut tail = [0u8; 2];
            tail[..tail_len].copy_from_slice(&remaining[full_len..]);
            let result = if full_len > 0 {
                self.push_encoded(&remaining[..full_len])
            } else {
                Ok(())
            };
            crate::wipe_bytes(&mut input);
            if let Err(err) = result {
                crate::wipe_bytes(&mut tail);
                return Err(err);
            }
            self.pending[..tail_len].copy_from_slice(&tail[..tail_len]);
            crate::wipe_bytes(&mut tail);
            self.pending_len = tail_len;
            Ok(())
        }

        fn push_final_pending(&mut self) -> io::Result<()> {
            if self.pending_len == 0 {
                return Ok(());
            }

            let mut pending = [0u8; 2];
            pending[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
            let pending_len = self.pending_len;
            self.clear_pending();
            let result = self.push_encoded(&pending[..pending_len]);
            crate::wipe_bytes(&mut pending);
            result
        }

        fn push_encoded(&mut self, input: &[u8]) -> io::Result<()> {
            let mut encoded = [0u8; 1024];
            let written = match self.engine.encode_slice(input, &mut encoded) {
                Ok(written) => written,
                Err(err) => {
                    crate::wipe_bytes(&mut encoded);
                    return Err(encode_error_to_io(err));
                }
            };
            let result = self.output.push_slice(&encoded[..written]);
            crate::wipe_bytes(&mut encoded);
            result
        }
    }

    const fn redacted_inner_state(present: bool) -> &'static str {
        if present { "<present>" } else { "<taken>" }
    }
}

/// Constant-time-oriented scalar decoding APIs.
///
/// This module is separate from the default decoder so callers can opt into a
/// slower path with a narrower timing target. It avoids lookup tables indexed
/// by secret input bytes while mapping Base64 symbols and reports malformed
/// content through one opaque error. It is not documented as a formally
/// verified cryptographic constant-time API.
pub mod ct {
    use super::{
        Alphabet, DecodeError, DecodedBuffer, Standard, UrlSafe, ct_decode_in_place,
        ct_decode_slice, ct_decoded_len, ct_validate_decode,
    };
    use core::marker::PhantomData;

    /// Standard Base64 constant-time-oriented decoder with padding.
    pub const STANDARD: CtEngine<Standard, true> = CtEngine::new();

    /// Standard Base64 constant-time-oriented decoder without padding.
    pub const STANDARD_NO_PAD: CtEngine<Standard, false> = CtEngine::new();

    /// URL-safe Base64 constant-time-oriented decoder with padding.
    pub const URL_SAFE: CtEngine<UrlSafe, true> = CtEngine::new();

    /// URL-safe Base64 constant-time-oriented decoder without padding.
    pub const URL_SAFE_NO_PAD: CtEngine<UrlSafe, false> = CtEngine::new();

    /// A zero-sized constant-time-oriented Base64 decoder.
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    pub struct CtEngine<A, const PAD: bool> {
        alphabet: PhantomData<A>,
    }

    impl<A, const PAD: bool> CtEngine<A, PAD>
    where
        A: Alphabet,
    {
        /// Creates a new constant-time-oriented decoder engine.
        #[must_use]
        pub const fn new() -> Self {
            Self {
                alphabet: PhantomData,
            }
        }

        /// Returns whether this constant-time-oriented decoder expects padded
        /// input.
        #[must_use]
        pub const fn is_padded(&self) -> bool {
            PAD
        }

        /// Validates `input` without writing decoded bytes.
        ///
        /// This uses the same constant-time-oriented symbol mapping and opaque
        /// malformed-input error behavior as [`Self::decode_slice`]. Input
        /// length, padding length, and final success or failure remain public.
        ///
        /// # Examples
        ///
        /// ```
        /// use base64_ng::ct;
        ///
        /// ct::STANDARD.validate_result(b"aGVsbG8=").unwrap();
        /// assert!(ct::STANDARD.validate_result(b"aGVsbG8").is_err());
        /// ```
        pub fn validate_result(&self, input: &[u8]) -> Result<(), DecodeError> {
            ct_validate_decode::<A, PAD>(input)
        }

        /// Returns whether `input` is valid for this constant-time-oriented
        /// decoder.
        ///
        /// This is a convenience wrapper around [`Self::validate_result`].
        ///
        /// # Examples
        ///
        /// ```
        /// use base64_ng::ct;
        ///
        /// assert!(ct::URL_SAFE_NO_PAD.validate(b"-_8"));
        /// assert!(!ct::URL_SAFE_NO_PAD.validate(b"+/8"));
        /// ```
        #[must_use]
        pub fn validate(&self, input: &[u8]) -> bool {
            self.validate_result(input).is_ok()
        }

        /// Returns the exact decoded length for valid input.
        ///
        /// This uses the same constant-time-oriented validation policy as
        /// [`Self::decode_slice`] before returning a length. Input length,
        /// padding length, and final success or failure remain public.
        pub fn decoded_len(&self, input: &[u8]) -> Result<usize, DecodeError> {
            ct_decoded_len::<A, PAD>(input)
        }

        /// Decodes `input` into `output`, returning the number of bytes
        /// written.
        ///
        /// This path uses a fixed alphabet scan for Base64 symbol mapping and
        /// avoids secret-indexed lookup tables. Input length, padding length,
        /// output length, and final success or failure remain public.
        /// Malformed content errors are intentionally opaque and non-localized;
        /// use the normal strict decoder when exact diagnostics are required.
        ///
        /// # Examples
        ///
        /// ```
        /// use base64_ng::ct;
        ///
        /// let mut output = [0u8; 5];
        /// let written = ct::STANDARD
        ///     .decode_slice(b"aGVsbG8=", &mut output)
        ///     .unwrap();
        ///
        /// assert_eq!(&output[..written], b"hello");
        /// ```
        pub fn decode_slice(&self, input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
            ct_decode_slice::<A, PAD>(input, output)
        }

        /// Decodes `input` into `output` and clears all bytes after the
        /// decoded prefix.
        ///
        /// If decoding fails, the entire output buffer is cleared before the
        /// error is returned. Use this variant for sensitive payloads where
        /// partially decoded bytes from rejected input should not remain in the
        /// caller-owned output buffer.
        ///
        /// # Examples
        ///
        /// ```
        /// use base64_ng::ct;
        ///
        /// let mut output = [0xff; 8];
        /// let written = ct::STANDARD
        ///     .decode_slice_clear_tail(b"aGk=", &mut output)
        ///     .unwrap();
        ///
        /// assert_eq!(&output[..written], b"hi");
        /// assert!(output[written..].iter().all(|byte| *byte == 0));
        /// ```
        pub fn decode_slice_clear_tail(
            &self,
            input: &[u8],
            output: &mut [u8],
        ) -> Result<usize, DecodeError> {
            let written = match self.decode_slice(input, output) {
                Ok(written) => written,
                Err(err) => {
                    crate::wipe_bytes(output);
                    return Err(err);
                }
            };
            crate::wipe_tail(output, written);
            Ok(written)
        }

        /// Decodes `input` into a stack-backed buffer.
        ///
        /// This uses the same constant-time-oriented scalar decoder as
        /// [`Self::decode_slice_clear_tail`] and clears the internal backing
        /// array before returning an error.
        ///
        /// # Examples
        ///
        /// ```
        /// use base64_ng::ct;
        ///
        /// let decoded = ct::STANDARD.decode_buffer::<5>(b"aGVsbG8=").unwrap();
        ///
        /// assert_eq!(decoded.as_bytes(), b"hello");
        /// ```
        pub fn decode_buffer<const CAP: usize>(
            &self,
            input: &[u8],
        ) -> Result<DecodedBuffer<CAP>, DecodeError> {
            let mut output = DecodedBuffer::new();
            let written = match self.decode_slice_clear_tail(input, &mut output.bytes) {
                Ok(written) => written,
                Err(err) => {
                    output.clear();
                    return Err(err);
                }
            };
            output.len = written;
            Ok(output)
        }

        /// Decodes `buffer` in place and returns the decoded prefix.
        ///
        /// This uses the constant-time-oriented scalar decoder while reading
        /// each Base64 quantum into local values before writing decoded bytes
        /// back to the front of the same buffer.
        ///
        /// # Examples
        ///
        /// ```
        /// use base64_ng::ct;
        ///
        /// let mut buffer = *b"aGk=";
        /// let decoded = ct::STANDARD.decode_in_place(&mut buffer).unwrap();
        ///
        /// assert_eq!(decoded, b"hi");
        /// ```
        pub fn decode_in_place<'a>(
            &self,
            buffer: &'a mut [u8],
        ) -> Result<&'a mut [u8], DecodeError> {
            let len = ct_decode_in_place::<A, PAD>(buffer)?;
            Ok(&mut buffer[..len])
        }

        /// Decodes `buffer` in place and clears all bytes after the decoded
        /// prefix.
        ///
        /// If decoding fails, the entire buffer is cleared before the error is
        /// returned.
        ///
        /// # Examples
        ///
        /// ```
        /// use base64_ng::ct;
        ///
        /// let mut buffer = *b"aGk=";
        /// let decoded = ct::STANDARD.decode_in_place_clear_tail(&mut buffer).unwrap();
        ///
        /// assert_eq!(decoded, b"hi");
        /// ```
        pub fn decode_in_place_clear_tail<'a>(
            &self,
            buffer: &'a mut [u8],
        ) -> Result<&'a mut [u8], DecodeError> {
            let len = match ct_decode_in_place::<A, PAD>(buffer) {
                Ok(len) => len,
                Err(err) => {
                    crate::wipe_bytes(buffer);
                    return Err(err);
                }
            };
            crate::wipe_tail(buffer, len);
            Ok(&mut buffer[..len])
        }
    }

    impl<A, const PAD: bool> core::fmt::Display for CtEngine<A, PAD> {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(formatter, "ct padded={PAD}")
        }
    }
}

/// Standard Base64 engine with padding.
pub const STANDARD: Engine<Standard, true> = Engine::new();

/// Standard Base64 engine without padding.
pub const STANDARD_NO_PAD: Engine<Standard, false> = Engine::new();

/// URL-safe Base64 engine with padding.
pub const URL_SAFE: Engine<UrlSafe, true> = Engine::new();

/// URL-safe Base64 engine without padding.
pub const URL_SAFE_NO_PAD: Engine<UrlSafe, false> = Engine::new();

/// bcrypt-style Base64 engine without padding.
///
/// This uses the bcrypt alphabet with the crate's normal Base64 bit packing.
/// It does not parse complete bcrypt password-hash strings.
pub const BCRYPT_NO_PAD: Engine<Bcrypt, false> = Engine::new();

/// Unix `crypt(3)`-style Base64 engine without padding.
///
/// This uses the `crypt(3)` alphabet with the crate's normal Base64 bit
/// packing. It does not parse complete password-hash strings.
pub const CRYPT_NO_PAD: Engine<Crypt, false> = Engine::new();

/// Line ending used by wrapped Base64 output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineEnding {
    /// Line feed (`\n`).
    Lf,
    /// Carriage return followed by line feed (`\r\n`).
    CrLf,
}

impl LineEnding {
    /// Returns a stable printable identifier for this line ending.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Lf => "LF",
            Self::CrLf => "CRLF",
        }
    }

    /// Returns the text representation of this line ending.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
        }
    }

    /// Returns the byte representation of this line ending.
    #[must_use]
    pub const fn as_bytes(self) -> &'static [u8] {
        self.as_str().as_bytes()
    }

    /// Returns the byte length of this line ending.
    #[must_use]
    pub const fn byte_len(self) -> usize {
        match self {
            Self::Lf => 1,
            Self::CrLf => 2,
        }
    }
}

impl core::fmt::Display for LineEnding {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.name())
    }
}

/// Base64 line wrapping policy.
///
/// `line_len` is measured in encoded Base64 bytes, not source input bytes.
/// Encoders insert line endings between lines and do not append a trailing line
/// ending after the final line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineWrap {
    /// Maximum encoded bytes per line.
    pub line_len: usize,
    /// Line ending inserted between wrapped lines.
    pub line_ending: LineEnding,
}

impl LineWrap {
    /// MIME-style wrapping: 76 columns with CRLF endings.
    pub const MIME: Self = Self::new(76, LineEnding::CrLf);
    /// PEM-style wrapping: 64 columns with LF endings.
    pub const PEM: Self = Self::new(64, LineEnding::Lf);
    /// PEM-style wrapping: 64 columns with CRLF endings.
    pub const PEM_CRLF: Self = Self::new(64, LineEnding::CrLf);

    /// Creates a wrapping policy.
    #[must_use]
    pub const fn new(line_len: usize, line_ending: LineEnding) -> Self {
        Self {
            line_len,
            line_ending,
        }
    }

    /// Creates a wrapping policy, returning `None` when the line length is
    /// invalid.
    ///
    /// Base64 line-wrapping requires a non-zero encoded line length. This
    /// helper is useful when accepting a wrapping policy from configuration or
    /// another untrusted source.
    #[must_use]
    pub const fn checked_new(line_len: usize, line_ending: LineEnding) -> Option<Self> {
        if line_len == 0 {
            None
        } else {
            Some(Self::new(line_len, line_ending))
        }
    }

    /// Returns the maximum encoded bytes per line.
    #[must_use]
    pub const fn line_len(self) -> usize {
        self.line_len
    }

    /// Returns the line ending inserted between wrapped lines.
    #[must_use]
    pub const fn line_ending(self) -> LineEnding {
        self.line_ending
    }

    /// Returns whether this wrapping policy can be used by the encoder.
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.line_len != 0
    }
}

impl core::fmt::Display for LineWrap {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{}:{}", self.line_len, self.line_ending.name())
    }
}

#[allow(unsafe_code)]
fn wipe_bytes(bytes: &mut [u8]) {
    for byte in bytes {
        // SAFETY: `byte` comes from a unique mutable slice iterator, so the
        // pointer is non-null, aligned, valid for one `u8` write, and does not
        // alias another live mutable reference during this iteration.
        unsafe {
            core::ptr::write_volatile(byte, 0);
        }
    }
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
}

fn wipe_tail(bytes: &mut [u8], start: usize) {
    wipe_bytes(&mut bytes[start..]);
}

#[cfg(feature = "alloc")]
#[allow(unsafe_code)]
fn wipe_vec_spare_capacity(bytes: &mut alloc::vec::Vec<u8>) {
    let ptr = bytes.as_mut_ptr();
    let mut offset = bytes.len();
    while offset < bytes.capacity() {
        // SAFETY: `offset` is within the vector allocation's spare capacity, so
        // the pointer is valid, aligned, and writable for one `u8`. This writes
        // a zero byte without reading the prior uninitialized value.
        unsafe {
            core::ptr::write_volatile(ptr.add(offset), 0);
        }
        offset += 1;
    }
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
}

#[cfg(feature = "alloc")]
fn wipe_vec_all(bytes: &mut alloc::vec::Vec<u8>) {
    wipe_bytes(bytes);
    wipe_vec_spare_capacity(bytes);
}

/// Stack-backed encoded Base64 output.
///
/// This type is intended for short values where heap allocation would be
/// unnecessary but manually sizing and passing a separate output slice is
/// noisy. Its visible bytes are produced by crate encoders, so [`Self::as_str`]
/// can return `&str` without exposing a fallible UTF-8 conversion to callers.
///
/// The backing array is cleared when the value is dropped. This is best-effort
/// data-retention reduction and is not a formal zeroization guarantee.
pub struct EncodedBuffer<const CAP: usize> {
    bytes: [u8; CAP],
    len: usize,
}

impl<const CAP: usize> EncodedBuffer<CAP> {
    /// Creates an empty encoded buffer.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bytes: [0u8; CAP],
            len: 0,
        }
    }

    /// Returns the number of visible encoded bytes.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns whether the buffer has no visible encoded bytes.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns whether the visible encoded bytes fill the stack backing array.
    #[must_use]
    pub const fn is_full(&self) -> bool {
        self.len == CAP
    }

    /// Returns the stack capacity in bytes.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        CAP
    }

    /// Returns the number of unused bytes in the stack backing array.
    #[must_use]
    pub const fn remaining_capacity(&self) -> usize {
        CAP - self.len
    }

    /// Returns the visible encoded bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    /// Returns the visible encoded bytes as UTF-8 text.
    ///
    /// Encoded Base64 output is produced as ASCII by this crate, so this
    /// method should not fail unless an internal invariant has been broken.
    /// It is provided for callers that prefer a fallible accessor over
    /// [`Self::as_str`].
    pub fn as_utf8(&self) -> Result<&str, core::str::Utf8Error> {
        core::str::from_utf8(self.as_bytes())
    }

    /// Returns the visible encoded bytes as UTF-8.
    ///
    /// # Panics
    ///
    /// Panics only if the crate's internal invariant is broken and the buffer
    /// contains non-UTF-8 bytes.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self.as_utf8() {
            Ok(output) => output,
            Err(_) => unreachable!("base64 encoder produced non-UTF-8 output"),
        }
    }

    /// Compares this encoded output to `other` without short-circuiting on the
    /// first differing byte.
    ///
    /// Length and the final equality result remain public. For equal-length
    /// inputs, this helper scans every byte before returning. It is
    /// constant-time-oriented best effort, not a formal cryptographic
    /// constant-time guarantee.
    #[must_use]
    pub fn constant_time_eq(&self, other: &[u8]) -> bool {
        constant_time_eq_public_len(self.as_bytes(), other)
    }

    /// Consumes the wrapper and returns the backing array plus visible length.
    ///
    /// This is an explicit escape hatch for no-alloc interop with APIs that
    /// require ownership of a fixed array. The returned array is no longer
    /// redacted by formatting and will not be cleared by `EncodedBuffer` on
    /// drop; callers that keep handling sensitive data should arrange their
    /// own cleanup.
    #[must_use]
    pub fn into_exposed_array(mut self) -> ([u8; CAP], usize) {
        let len = self.len;
        self.len = 0;
        (core::mem::replace(&mut self.bytes, [0u8; CAP]), len)
    }

    /// Clears the visible bytes and the full backing array.
    pub fn clear(&mut self) {
        wipe_bytes(&mut self.bytes);
        self.len = 0;
    }

    /// Clears bytes after the visible prefix.
    pub fn clear_tail(&mut self) {
        wipe_tail(&mut self.bytes, self.len);
    }
}

impl<const CAP: usize> AsRef<[u8]> for EncodedBuffer<CAP> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<const CAP: usize> Clone for EncodedBuffer<CAP> {
    fn clone(&self) -> Self {
        let mut output = Self::new();
        output.bytes[..self.len].copy_from_slice(self.as_bytes());
        output.len = self.len;
        output
    }
}

impl<const CAP: usize> core::fmt::Debug for EncodedBuffer<CAP> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("EncodedBuffer")
            .field("bytes", &"<redacted>")
            .field("len", &self.len)
            .field("capacity", &CAP)
            .finish()
    }
}

impl<const CAP: usize> core::fmt::Display for EncodedBuffer<CAP> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl<const CAP: usize> Default for EncodedBuffer<CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAP: usize> Drop for EncodedBuffer<CAP> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<const CAP: usize> Eq for EncodedBuffer<CAP> {}

impl<const CAP: usize> PartialEq for EncodedBuffer<CAP> {
    fn eq(&self, other: &Self) -> bool {
        self.constant_time_eq(other.as_bytes())
    }
}

impl<const CAP: usize> PartialEq<&[u8]> for EncodedBuffer<CAP> {
    fn eq(&self, other: &&[u8]) -> bool {
        self.constant_time_eq(other)
    }
}

impl<const CAP: usize, const N: usize> PartialEq<&[u8; N]> for EncodedBuffer<CAP> {
    fn eq(&self, other: &&[u8; N]) -> bool {
        self.constant_time_eq(&other[..])
    }
}

impl<const CAP: usize> PartialEq<&str> for EncodedBuffer<CAP> {
    fn eq(&self, other: &&str) -> bool {
        self.constant_time_eq(other.as_bytes())
    }
}

#[cfg(feature = "alloc")]
impl<const CAP: usize> PartialEq<alloc::string::String> for EncodedBuffer<CAP> {
    fn eq(&self, other: &alloc::string::String) -> bool {
        self.constant_time_eq(other.as_bytes())
    }
}

impl<const CAP: usize> PartialEq<EncodedBuffer<CAP>> for &[u8] {
    fn eq(&self, other: &EncodedBuffer<CAP>) -> bool {
        other.constant_time_eq(self)
    }
}

impl<const CAP: usize, const N: usize> PartialEq<EncodedBuffer<CAP>> for &[u8; N] {
    fn eq(&self, other: &EncodedBuffer<CAP>) -> bool {
        other.constant_time_eq(&self[..])
    }
}

impl<const CAP: usize> PartialEq<EncodedBuffer<CAP>> for &str {
    fn eq(&self, other: &EncodedBuffer<CAP>) -> bool {
        other.constant_time_eq(self.as_bytes())
    }
}

#[cfg(feature = "alloc")]
impl<const CAP: usize> PartialEq<EncodedBuffer<CAP>> for alloc::string::String {
    fn eq(&self, other: &EncodedBuffer<CAP>) -> bool {
        other.constant_time_eq(self.as_bytes())
    }
}

impl<const CAP: usize> TryFrom<&[u8]> for EncodedBuffer<CAP> {
    type Error = EncodeError;

    /// Encodes bytes into strict standard padded Base64 in a stack-backed
    /// buffer.
    ///
    /// Use [`Engine::encode_buffer`] or [`Profile::encode_buffer`] when a
    /// different alphabet, padding mode, or line-wrapping profile is required.
    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        STANDARD.encode_buffer(input)
    }
}

impl<const CAP: usize> TryFrom<&str> for EncodedBuffer<CAP> {
    type Error = EncodeError;

    /// Encodes UTF-8 text bytes into strict standard padded Base64 in a
    /// stack-backed buffer.
    ///
    /// This treats the string as raw input bytes. Use
    /// [`Engine::encode_buffer`] or [`Profile::encode_buffer`] when a
    /// different alphabet, padding mode, or line-wrapping profile is required.
    fn try_from(input: &str) -> Result<Self, Self::Error> {
        Self::try_from(input.as_bytes())
    }
}

/// Stack-backed decoded Base64 output.
///
/// This type is intended for short decoded values where heap allocation would
/// be unnecessary but manually sizing and passing a separate output slice is
/// noisy. Decoded data may be binary or secret-bearing, so formatting is
/// redacted and contents are exposed only through explicit byte accessors.
///
/// The backing array is cleared when the value is dropped. This is best-effort
/// data-retention reduction and is not a formal zeroization guarantee.
pub struct DecodedBuffer<const CAP: usize> {
    bytes: [u8; CAP],
    len: usize,
}

impl<const CAP: usize> DecodedBuffer<CAP> {
    /// Creates an empty decoded buffer.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bytes: [0u8; CAP],
            len: 0,
        }
    }

    /// Returns the number of visible decoded bytes.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns whether the buffer has no visible decoded bytes.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns whether the visible decoded bytes fill the stack backing array.
    #[must_use]
    pub const fn is_full(&self) -> bool {
        self.len == CAP
    }

    /// Returns the stack capacity in bytes.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        CAP
    }

    /// Returns the number of unused bytes in the stack backing array.
    #[must_use]
    pub const fn remaining_capacity(&self) -> usize {
        CAP - self.len
    }

    /// Returns the visible decoded bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    /// Returns the visible decoded bytes as UTF-8 text.
    ///
    /// Decoded Base64 output is arbitrary bytes, so this method is fallible.
    /// Use [`Self::as_bytes`] when the decoded payload is binary or when text
    /// validation belongs to a higher protocol layer.
    pub fn as_utf8(&self) -> Result<&str, core::str::Utf8Error> {
        core::str::from_utf8(self.as_bytes())
    }

    /// Compares this decoded output to `other` without short-circuiting on the
    /// first differing byte.
    ///
    /// Length and the final equality result remain public. For equal-length
    /// inputs, this helper scans every byte before returning. It is
    /// constant-time-oriented best effort, not a formal cryptographic
    /// constant-time guarantee.
    #[must_use]
    pub fn constant_time_eq(&self, other: &[u8]) -> bool {
        constant_time_eq_public_len(self.as_bytes(), other)
    }

    /// Consumes the wrapper and returns the backing array plus visible length.
    ///
    /// This is an explicit escape hatch for no-alloc interop with APIs that
    /// require ownership of a fixed array. The returned array is no longer
    /// redacted by formatting and will not be cleared by `DecodedBuffer` on
    /// drop; callers that keep handling sensitive data should arrange their
    /// own cleanup.
    #[must_use]
    pub fn into_exposed_array(mut self) -> ([u8; CAP], usize) {
        let len = self.len;
        self.len = 0;
        (core::mem::replace(&mut self.bytes, [0u8; CAP]), len)
    }

    /// Clears the visible bytes and the full backing array.
    pub fn clear(&mut self) {
        wipe_bytes(&mut self.bytes);
        self.len = 0;
    }

    /// Clears bytes after the visible prefix.
    pub fn clear_tail(&mut self) {
        wipe_tail(&mut self.bytes, self.len);
    }
}

impl<const CAP: usize> AsRef<[u8]> for DecodedBuffer<CAP> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<const CAP: usize> Clone for DecodedBuffer<CAP> {
    fn clone(&self) -> Self {
        let mut output = Self::new();
        output.bytes[..self.len].copy_from_slice(self.as_bytes());
        output.len = self.len;
        output
    }
}

impl<const CAP: usize> core::fmt::Debug for DecodedBuffer<CAP> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("DecodedBuffer")
            .field("bytes", &"<redacted>")
            .field("len", &self.len)
            .field("capacity", &CAP)
            .finish()
    }
}

impl<const CAP: usize> Default for DecodedBuffer<CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAP: usize> Drop for DecodedBuffer<CAP> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<const CAP: usize> Eq for DecodedBuffer<CAP> {}

impl<const CAP: usize> PartialEq for DecodedBuffer<CAP> {
    fn eq(&self, other: &Self) -> bool {
        self.constant_time_eq(other.as_bytes())
    }
}

impl<const CAP: usize> PartialEq<&[u8]> for DecodedBuffer<CAP> {
    fn eq(&self, other: &&[u8]) -> bool {
        self.constant_time_eq(other)
    }
}

impl<const CAP: usize, const N: usize> PartialEq<&[u8; N]> for DecodedBuffer<CAP> {
    fn eq(&self, other: &&[u8; N]) -> bool {
        self.constant_time_eq(&other[..])
    }
}

impl<const CAP: usize> PartialEq<&str> for DecodedBuffer<CAP> {
    fn eq(&self, other: &&str) -> bool {
        self.constant_time_eq(other.as_bytes())
    }
}

#[cfg(feature = "alloc")]
impl<const CAP: usize> PartialEq<alloc::string::String> for DecodedBuffer<CAP> {
    fn eq(&self, other: &alloc::string::String) -> bool {
        self.constant_time_eq(other.as_bytes())
    }
}

impl<const CAP: usize> PartialEq<DecodedBuffer<CAP>> for &[u8] {
    fn eq(&self, other: &DecodedBuffer<CAP>) -> bool {
        other.constant_time_eq(self)
    }
}

impl<const CAP: usize, const N: usize> PartialEq<DecodedBuffer<CAP>> for &[u8; N] {
    fn eq(&self, other: &DecodedBuffer<CAP>) -> bool {
        other.constant_time_eq(&self[..])
    }
}

impl<const CAP: usize> PartialEq<DecodedBuffer<CAP>> for &str {
    fn eq(&self, other: &DecodedBuffer<CAP>) -> bool {
        other.constant_time_eq(self.as_bytes())
    }
}

#[cfg(feature = "alloc")]
impl<const CAP: usize> PartialEq<DecodedBuffer<CAP>> for alloc::string::String {
    fn eq(&self, other: &DecodedBuffer<CAP>) -> bool {
        other.constant_time_eq(self.as_bytes())
    }
}

impl<const CAP: usize> TryFrom<&[u8]> for DecodedBuffer<CAP> {
    type Error = DecodeError;

    /// Decodes strict standard padded Base64 into a stack-backed buffer.
    ///
    /// Use [`Engine::decode_buffer`] or [`Profile::decode_buffer`] when a
    /// different alphabet, padding mode, or line-wrapping profile is required.
    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        STANDARD.decode_buffer(input)
    }
}

impl<const CAP: usize> TryFrom<&str> for DecodedBuffer<CAP> {
    type Error = DecodeError;

    /// Decodes strict standard padded Base64 text into a stack-backed buffer.
    ///
    /// Use [`Engine::decode_buffer`] or [`Profile::decode_buffer`] when a
    /// different alphabet, padding mode, or line-wrapping profile is required.
    fn try_from(input: &str) -> Result<Self, Self::Error> {
        Self::try_from(input.as_bytes())
    }
}

/// Owned sensitive bytes with redacted formatting and drop-time cleanup.
///
/// `SecretBuffer` is available with the `alloc` feature. It is intended for
/// decoded keys, tokens, and other values that should not be accidentally
/// logged. The buffer exposes contents only through explicit reveal methods.
///
/// Spare vector capacity is cleared when wrapping owned bytes. On drop,
/// initialized bytes and vector spare capacity are cleared with the crate's
/// internal best-effort wipe helpers. This is data-retention reduction, not a
/// formal zeroization guarantee, and it cannot make claims about allocator
/// behavior or historical copies outside the wrapper.
#[cfg(feature = "alloc")]
pub struct SecretBuffer {
    bytes: alloc::vec::Vec<u8>,
}

#[cfg(feature = "alloc")]
impl SecretBuffer {
    /// Wraps an existing vector as sensitive material.
    #[must_use]
    pub fn from_vec(mut bytes: alloc::vec::Vec<u8>) -> Self {
        wipe_vec_spare_capacity(&mut bytes);
        Self { bytes }
    }

    /// Copies a slice into an owned sensitive buffer.
    #[must_use]
    pub fn from_slice(bytes: &[u8]) -> Self {
        Self::from_vec(bytes.to_vec())
    }

    /// Returns the number of initialized secret bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Returns whether the buffer contains no initialized secret bytes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Reveals the secret bytes.
    ///
    /// This method is intentionally named to make secret access explicit at the
    /// call site.
    #[must_use]
    pub fn expose_secret(&self) -> &[u8] {
        &self.bytes
    }

    /// Reveals the secret bytes as UTF-8 text.
    ///
    /// This method is intentionally named to make secret access explicit at the
    /// call site. Secret material may be arbitrary binary data, so this method
    /// is fallible.
    pub fn expose_secret_utf8(&self) -> Result<&str, core::str::Utf8Error> {
        core::str::from_utf8(self.expose_secret())
    }

    /// Reveals the secret bytes mutably.
    ///
    /// This method is intentionally named to make secret access explicit at the
    /// call site.
    #[must_use]
    pub fn expose_secret_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
    }

    /// Consumes the wrapper and returns the owned secret bytes.
    ///
    /// This is an explicit escape hatch for interop with APIs that require an
    /// owned vector. The returned `Vec<u8>` is no longer redacted by
    /// formatting and will not be cleared by `SecretBuffer` on drop; callers
    /// that keep handling sensitive data should arrange their own cleanup.
    #[must_use]
    pub fn into_exposed_vec(mut self) -> alloc::vec::Vec<u8> {
        core::mem::take(&mut self.bytes)
    }

    /// Consumes the wrapper and returns the owned secret bytes as UTF-8 text.
    ///
    /// This is an explicit escape hatch for interop with APIs that require an
    /// owned string. The returned `String` is no longer redacted by formatting
    /// and will not be cleared by `SecretBuffer` on drop; callers that keep
    /// handling sensitive data should arrange their own cleanup.
    ///
    /// If the secret bytes are not valid UTF-8, the original redacted wrapper
    /// is returned unchanged.
    pub fn try_into_exposed_string(self) -> Result<alloc::string::String, Self> {
        if core::str::from_utf8(self.expose_secret()).is_err() {
            return Err(self);
        }

        match alloc::string::String::from_utf8(self.into_exposed_vec()) {
            Ok(text) => Ok(text),
            Err(error) => Err(Self::from_vec(error.into_bytes())),
        }
    }

    /// Compares this secret to `other` without short-circuiting on the first
    /// differing byte.
    ///
    /// Length and the final equality result remain public. For equal-length
    /// inputs, this helper scans every byte before returning. It is
    /// constant-time-oriented best effort, not a formal cryptographic
    /// constant-time guarantee.
    #[must_use]
    pub fn constant_time_eq(&self, other: &[u8]) -> bool {
        constant_time_eq_public_len(self.expose_secret(), other)
    }

    /// Clears the initialized bytes and makes the buffer empty.
    pub fn clear(&mut self) {
        wipe_vec_all(&mut self.bytes);
        self.bytes.clear();
    }
}

#[cfg(feature = "alloc")]
impl Clone for SecretBuffer {
    fn clone(&self) -> Self {
        Self::from_slice(self.expose_secret())
    }
}

#[cfg(feature = "alloc")]
impl core::fmt::Debug for SecretBuffer {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("SecretBuffer")
            .field("bytes", &"<redacted>")
            .field("len", &self.len())
            .finish()
    }
}

#[cfg(feature = "alloc")]
impl core::fmt::Display for SecretBuffer {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("<redacted>")
    }
}

#[cfg(feature = "alloc")]
impl Drop for SecretBuffer {
    fn drop(&mut self) {
        wipe_vec_all(&mut self.bytes);
    }
}

#[cfg(feature = "alloc")]
impl Eq for SecretBuffer {}

#[cfg(feature = "alloc")]
impl PartialEq for SecretBuffer {
    fn eq(&self, other: &Self) -> bool {
        self.constant_time_eq(other.expose_secret())
    }
}

#[cfg(feature = "alloc")]
impl PartialEq<&[u8]> for SecretBuffer {
    fn eq(&self, other: &&[u8]) -> bool {
        self.constant_time_eq(other)
    }
}

#[cfg(feature = "alloc")]
impl<const N: usize> PartialEq<&[u8; N]> for SecretBuffer {
    fn eq(&self, other: &&[u8; N]) -> bool {
        self.constant_time_eq(&other[..])
    }
}

#[cfg(feature = "alloc")]
impl PartialEq<&str> for SecretBuffer {
    fn eq(&self, other: &&str) -> bool {
        self.constant_time_eq(other.as_bytes())
    }
}

#[cfg(feature = "alloc")]
impl PartialEq<alloc::string::String> for SecretBuffer {
    fn eq(&self, other: &alloc::string::String) -> bool {
        self.constant_time_eq(other.as_bytes())
    }
}

#[cfg(feature = "alloc")]
impl PartialEq<SecretBuffer> for &[u8] {
    fn eq(&self, other: &SecretBuffer) -> bool {
        other.constant_time_eq(self)
    }
}

#[cfg(feature = "alloc")]
impl<const N: usize> PartialEq<SecretBuffer> for &[u8; N] {
    fn eq(&self, other: &SecretBuffer) -> bool {
        other.constant_time_eq(&self[..])
    }
}

#[cfg(feature = "alloc")]
impl PartialEq<SecretBuffer> for &str {
    fn eq(&self, other: &SecretBuffer) -> bool {
        other.constant_time_eq(self.as_bytes())
    }
}

#[cfg(feature = "alloc")]
impl PartialEq<SecretBuffer> for alloc::string::String {
    fn eq(&self, other: &SecretBuffer) -> bool {
        other.constant_time_eq(self.as_bytes())
    }
}

#[cfg(feature = "alloc")]
impl From<alloc::vec::Vec<u8>> for SecretBuffer {
    /// Wraps an owned vector as sensitive material.
    ///
    /// Spare capacity is cleared immediately before the vector is stored.
    /// Use [`SecretBuffer::from_slice`] when the source data is borrowed.
    fn from(bytes: alloc::vec::Vec<u8>) -> Self {
        Self::from_vec(bytes)
    }
}

#[cfg(feature = "alloc")]
impl From<alloc::string::String> for SecretBuffer {
    /// Wraps an owned UTF-8 string as sensitive material.
    ///
    /// The string is consumed without copying its initialized bytes. Spare
    /// vector capacity is cleared immediately before the bytes are stored.
    fn from(text: alloc::string::String) -> Self {
        Self::from_vec(text.into_bytes())
    }
}

#[cfg(feature = "alloc")]
impl<const CAP: usize> From<EncodedBuffer<CAP>> for SecretBuffer {
    /// Copies visible encoded bytes from a stack-backed buffer into an owned
    /// redacted buffer.
    ///
    /// The consumed stack-backed buffer clears its backing array when it is
    /// dropped at the end of the conversion.
    fn from(buffer: EncodedBuffer<CAP>) -> Self {
        Self::from_slice(buffer.as_bytes())
    }
}

#[cfg(feature = "alloc")]
impl<const CAP: usize> From<DecodedBuffer<CAP>> for SecretBuffer {
    /// Copies visible decoded bytes from a stack-backed buffer into an owned
    /// redacted buffer.
    ///
    /// The consumed stack-backed buffer clears its backing array when it is
    /// dropped at the end of the conversion.
    fn from(buffer: DecodedBuffer<CAP>) -> Self {
        Self::from_slice(buffer.as_bytes())
    }
}

#[cfg(feature = "alloc")]
impl TryFrom<&[u8]> for SecretBuffer {
    type Error = DecodeError;

    /// Decodes strict standard padded Base64 into a redacted owned buffer.
    ///
    /// Use [`Engine::decode_secret`] or [`Profile::decode_secret`] when a
    /// different alphabet, padding mode, or line-wrapping profile is required.
    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        STANDARD.decode_secret(input)
    }
}

#[cfg(feature = "alloc")]
impl TryFrom<&str> for SecretBuffer {
    type Error = DecodeError;

    /// Decodes strict standard padded Base64 text into a redacted owned buffer.
    ///
    /// Use [`Engine::decode_secret`] or [`Profile::decode_secret`] when a
    /// different alphabet, padding mode, or line-wrapping profile is required.
    fn try_from(input: &str) -> Result<Self, Self::Error> {
        Self::try_from(input.as_bytes())
    }
}

/// A named Base64 profile with an engine and optional strict line wrapping.
///
/// Profiles are convenience values for protocol-shaped Base64. They keep the
/// same strict alphabet, padding, canonical-bit, and output-buffer rules as
/// [`Engine`], while carrying the wrapping policy for MIME/PEM-like formats.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Profile<A, const PAD: bool> {
    engine: Engine<A, PAD>,
    wrap: Option<LineWrap>,
}

impl<A, const PAD: bool> Profile<A, PAD>
where
    A: Alphabet,
{
    /// Creates a profile from an engine and optional strict line wrapping.
    #[must_use]
    pub const fn new(engine: Engine<A, PAD>, wrap: Option<LineWrap>) -> Self {
        Self { engine, wrap }
    }

    /// Creates a profile, returning `None` when the wrapping policy is invalid.
    ///
    /// This is useful when a profile is assembled from configuration or other
    /// untrusted metadata. Use [`Self::new`] for compile-time constants where
    /// the wrapping policy is known to be valid.
    #[must_use]
    pub const fn checked_new(engine: Engine<A, PAD>, wrap: Option<LineWrap>) -> Option<Self> {
        match wrap {
            Some(wrap) if !wrap.is_valid() => None,
            _ => Some(Self::new(engine, wrap)),
        }
    }

    /// Returns whether this profile can be used by encoders and decoders.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        match self.wrap {
            Some(wrap) => wrap.is_valid(),
            None => true,
        }
    }

    /// Returns the underlying engine.
    #[must_use]
    pub const fn engine(&self) -> Engine<A, PAD> {
        self.engine
    }

    /// Returns whether this profile uses padded Base64.
    #[must_use]
    pub const fn is_padded(&self) -> bool {
        PAD
    }

    /// Returns whether this profile carries a strict line-wrapping policy.
    #[must_use]
    pub const fn is_wrapped(&self) -> bool {
        self.wrap.is_some()
    }

    /// Returns the strict wrapping policy carried by this profile, if any.
    #[must_use]
    pub const fn line_wrap(&self) -> Option<LineWrap> {
        self.wrap
    }

    /// Returns the encoded line length for wrapped profiles.
    #[must_use]
    pub const fn line_len(&self) -> Option<usize> {
        match self.wrap {
            Some(wrap) => Some(wrap.line_len()),
            None => None,
        }
    }

    /// Returns the line ending for wrapped profiles.
    #[must_use]
    pub const fn line_ending(&self) -> Option<LineEnding> {
        match self.wrap {
            Some(wrap) => Some(wrap.line_ending()),
            None => None,
        }
    }

    /// Returns the encoded length for this profile.
    pub const fn encoded_len(&self, input_len: usize) -> Result<usize, EncodeError> {
        match self.wrap {
            Some(wrap) => wrapped_encoded_len(input_len, PAD, wrap),
            None => encoded_len(input_len, PAD),
        }
    }

    /// Returns the encoded length for this profile, or `None` on overflow or
    /// invalid line wrapping.
    #[must_use]
    pub const fn checked_encoded_len(&self, input_len: usize) -> Option<usize> {
        match self.wrap {
            Some(wrap) => checked_wrapped_encoded_len(input_len, PAD, wrap),
            None => checked_encoded_len(input_len, PAD),
        }
    }

    /// Returns the exact decoded length for this profile.
    pub fn decoded_len(&self, input: &[u8]) -> Result<usize, DecodeError> {
        match self.wrap {
            Some(wrap) => self.engine.decoded_len_wrapped(input, wrap),
            None => self.engine.decoded_len(input),
        }
    }

    /// Validates input according to this profile without writing decoded bytes.
    pub fn validate_result(&self, input: &[u8]) -> Result<(), DecodeError> {
        match self.wrap {
            Some(wrap) => self.engine.validate_wrapped_result(input, wrap),
            None => self.engine.validate_result(input),
        }
    }

    /// Returns whether `input` is valid for this profile.
    #[must_use]
    pub fn validate(&self, input: &[u8]) -> bool {
        self.validate_result(input).is_ok()
    }

    /// Encodes `input` into `output` according to this profile.
    pub fn encode_slice(&self, input: &[u8], output: &mut [u8]) -> Result<usize, EncodeError> {
        match self.wrap {
            Some(wrap) => self.engine.encode_slice_wrapped(input, output, wrap),
            None => self.engine.encode_slice(input, output),
        }
    }

    /// Encodes `input` into `output` and clears all bytes after the encoded
    /// prefix.
    pub fn encode_slice_clear_tail(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, EncodeError> {
        match self.wrap {
            Some(wrap) => self
                .engine
                .encode_slice_wrapped_clear_tail(input, output, wrap),
            None => self.engine.encode_slice_clear_tail(input, output),
        }
    }

    /// Encodes `input` into a stack-backed buffer.
    ///
    /// This is useful for short values where heap allocation is unnecessary.
    /// If encoding fails, the internal backing array is cleared before the
    /// error is returned.
    pub fn encode_buffer<const CAP: usize>(
        &self,
        input: &[u8],
    ) -> Result<EncodedBuffer<CAP>, EncodeError> {
        let mut output = EncodedBuffer::new();
        let written = match self.encode_slice_clear_tail(input, &mut output.bytes) {
            Ok(written) => written,
            Err(err) => {
                output.clear();
                return Err(err);
            }
        };
        output.len = written;
        Ok(output)
    }

    /// Decodes `input` into `output` according to this profile.
    pub fn decode_slice(&self, input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
        match self.wrap {
            Some(wrap) => self.engine.decode_slice_wrapped(input, output, wrap),
            None => self.engine.decode_slice(input, output),
        }
    }

    /// Decodes `input` into `output` and clears all bytes after the decoded
    /// prefix.
    pub fn decode_slice_clear_tail(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError> {
        match self.wrap {
            Some(wrap) => self
                .engine
                .decode_slice_wrapped_clear_tail(input, output, wrap),
            None => self.engine.decode_slice_clear_tail(input, output),
        }
    }

    /// Decodes `input` into a stack-backed buffer according to this profile.
    ///
    /// This is useful for short decoded values where heap allocation is
    /// unnecessary. If decoding fails, the internal backing array is cleared
    /// before the error is returned.
    pub fn decode_buffer<const CAP: usize>(
        &self,
        input: &[u8],
    ) -> Result<DecodedBuffer<CAP>, DecodeError> {
        let mut output = DecodedBuffer::new();
        let written = match self.decode_slice_clear_tail(input, &mut output.bytes) {
            Ok(written) => written,
            Err(err) => {
                output.clear();
                return Err(err);
            }
        };
        output.len = written;
        Ok(output)
    }

    /// Decodes `buffer` in place according to this profile.
    ///
    /// For wrapped profiles, configured line endings are compacted out before
    /// decoding. If validation fails, the buffer contents are unspecified.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{LineEnding, LineWrap, Profile, STANDARD};
    ///
    /// let profile = Profile::new(STANDARD, Some(LineWrap::new(4, LineEnding::Lf)));
    /// let mut buffer = *b"aGVs\nbG8=";
    /// let decoded = profile.decode_in_place(&mut buffer).unwrap();
    ///
    /// assert_eq!(decoded, b"hello");
    /// ```
    pub fn decode_in_place<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], DecodeError> {
        match self.wrap {
            Some(wrap) => self.engine.decode_in_place_wrapped(buffer, wrap),
            None => self.engine.decode_in_place(buffer),
        }
    }

    /// Decodes `buffer` in place according to this profile and clears all
    /// bytes after the decoded prefix.
    ///
    /// If validation or decoding fails, the entire buffer is cleared before the
    /// error is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{LineEnding, LineWrap, Profile, STANDARD};
    ///
    /// let profile = Profile::new(STANDARD, Some(LineWrap::new(4, LineEnding::Lf)));
    /// let mut buffer = *b"aGVs\nbG8=";
    /// let len = profile.decode_in_place_clear_tail(&mut buffer).unwrap().len();
    ///
    /// assert_eq!(&buffer[..len], b"hello");
    /// assert!(buffer[len..].iter().all(|byte| *byte == 0));
    /// ```
    pub fn decode_in_place_clear_tail<'a>(
        &self,
        buffer: &'a mut [u8],
    ) -> Result<&'a mut [u8], DecodeError> {
        match self.wrap {
            Some(wrap) => self.engine.decode_in_place_wrapped_clear_tail(buffer, wrap),
            None => self.engine.decode_in_place_clear_tail(buffer),
        }
    }

    /// Encodes `input` into a newly allocated byte vector.
    #[cfg(feature = "alloc")]
    pub fn encode_vec(&self, input: &[u8]) -> Result<alloc::vec::Vec<u8>, EncodeError> {
        match self.wrap {
            Some(wrap) => self.engine.encode_wrapped_vec(input, wrap),
            None => self.engine.encode_vec(input),
        }
    }

    /// Encodes `input` into a redacted owned secret buffer.
    #[cfg(feature = "alloc")]
    pub fn encode_secret(&self, input: &[u8]) -> Result<SecretBuffer, EncodeError> {
        self.encode_vec(input).map(SecretBuffer::from_vec)
    }

    /// Encodes `input` into a newly allocated UTF-8 string.
    #[cfg(feature = "alloc")]
    pub fn encode_string(&self, input: &[u8]) -> Result<alloc::string::String, EncodeError> {
        match self.wrap {
            Some(wrap) => self.engine.encode_wrapped_string(input, wrap),
            None => self.engine.encode_string(input),
        }
    }

    /// Decodes `input` into a newly allocated byte vector.
    #[cfg(feature = "alloc")]
    pub fn decode_vec(&self, input: &[u8]) -> Result<alloc::vec::Vec<u8>, DecodeError> {
        match self.wrap {
            Some(wrap) => self.engine.decode_wrapped_vec(input, wrap),
            None => self.engine.decode_vec(input),
        }
    }

    /// Decodes `input` into a redacted owned secret buffer.
    #[cfg(feature = "alloc")]
    pub fn decode_secret(&self, input: &[u8]) -> Result<SecretBuffer, DecodeError> {
        self.decode_vec(input).map(SecretBuffer::from_vec)
    }
}

impl<A, const PAD: bool> Default for Profile<A, PAD>
where
    A: Alphabet,
{
    fn default() -> Self {
        Self::new(Engine::new(), None)
    }
}

impl<A, const PAD: bool> core::fmt::Display for Profile<A, PAD>
where
    A: Alphabet,
{
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.wrap {
            Some(wrap) => write!(formatter, "padded={PAD} wrap={wrap}"),
            None => write!(formatter, "padded={PAD} wrap=none"),
        }
    }
}

impl<A, const PAD: bool> From<Engine<A, PAD>> for Profile<A, PAD>
where
    A: Alphabet,
{
    fn from(engine: Engine<A, PAD>) -> Self {
        Self::new(engine, None)
    }
}

/// MIME Base64 profile: standard alphabet, padding, 76-column CRLF wrapping.
pub const MIME: Profile<Standard, true> = Profile::new(STANDARD, Some(LineWrap::MIME));

/// PEM Base64 profile: standard alphabet, padding, 64-column LF wrapping.
pub const PEM: Profile<Standard, true> = Profile::new(STANDARD, Some(LineWrap::PEM));

/// PEM Base64 profile with CRLF line endings.
pub const PEM_CRLF: Profile<Standard, true> = Profile::new(STANDARD, Some(LineWrap::PEM_CRLF));

/// bcrypt-style no-padding Base64 profile.
///
/// This profile carries the bcrypt alphabet and no padding. It does not parse
/// complete bcrypt password-hash strings.
pub const BCRYPT: Profile<Bcrypt, false> = Profile::new(BCRYPT_NO_PAD, None);

/// Unix `crypt(3)`-style no-padding Base64 profile.
///
/// This profile carries the `crypt(3)` alphabet and no padding. It does not
/// parse complete password-hash strings.
pub const CRYPT: Profile<Crypt, false> = Profile::new(CRYPT_NO_PAD, None);

/// Returns the encoded length for an input length and padding policy.
///
/// This function returns [`EncodeError::LengthOverflow`] instead of panicking.
/// Use [`checked_encoded_len`] when an `Option<usize>` is more convenient.
///
/// # Examples
///
/// ```
/// use base64_ng::encoded_len;
///
/// assert_eq!(encoded_len(5, true).unwrap(), 8);
/// assert_eq!(encoded_len(5, false).unwrap(), 7);
/// assert!(encoded_len(usize::MAX, true).is_err());
/// ```
pub const fn encoded_len(input_len: usize, padded: bool) -> Result<usize, EncodeError> {
    match checked_encoded_len(input_len, padded) {
        Some(len) => Ok(len),
        None => Err(EncodeError::LengthOverflow),
    }
}

/// Returns the encoded length after applying a line wrapping policy.
///
/// The returned length includes inserted line endings but does not include a
/// trailing line ending after the final encoded line.
///
/// # Examples
///
/// ```
/// use base64_ng::{LineEnding, LineWrap, wrapped_encoded_len};
///
/// let wrap = LineWrap::new(4, LineEnding::Lf);
/// assert_eq!(wrapped_encoded_len(5, true, wrap).unwrap(), 9);
/// ```
pub const fn wrapped_encoded_len(
    input_len: usize,
    padded: bool,
    wrap: LineWrap,
) -> Result<usize, EncodeError> {
    if wrap.line_len == 0 {
        return Err(EncodeError::InvalidLineWrap { line_len: 0 });
    }

    let Some(encoded) = checked_encoded_len(input_len, padded) else {
        return Err(EncodeError::LengthOverflow);
    };
    if encoded == 0 {
        return Ok(0);
    }

    let breaks = (encoded - 1) / wrap.line_len;
    let Some(line_ending_bytes) = breaks.checked_mul(wrap.line_ending.byte_len()) else {
        return Err(EncodeError::LengthOverflow);
    };
    match encoded.checked_add(line_ending_bytes) {
        Some(len) => Ok(len),
        None => Err(EncodeError::LengthOverflow),
    }
}

/// Returns the encoded length after line wrapping, or `None` on overflow or
/// invalid line wrapping.
///
/// The returned length includes inserted line endings but does not include a
/// trailing line ending after the final encoded line.
///
/// # Examples
///
/// ```
/// use base64_ng::{LineEnding, LineWrap, checked_wrapped_encoded_len};
///
/// let wrap = LineWrap::new(4, LineEnding::Lf);
/// assert_eq!(checked_wrapped_encoded_len(5, true, wrap), Some(9));
/// assert_eq!(checked_wrapped_encoded_len(5, true, LineWrap::new(0, LineEnding::Lf)), None);
/// ```
#[must_use]
pub const fn checked_wrapped_encoded_len(
    input_len: usize,
    padded: bool,
    wrap: LineWrap,
) -> Option<usize> {
    if wrap.line_len == 0 {
        return None;
    }

    let Some(encoded) = checked_encoded_len(input_len, padded) else {
        return None;
    };
    if encoded == 0 {
        return Some(0);
    }

    let breaks = (encoded - 1) / wrap.line_len;
    let Some(line_ending_bytes) = breaks.checked_mul(wrap.line_ending.byte_len()) else {
        return None;
    };
    encoded.checked_add(line_ending_bytes)
}

/// Returns the encoded length, or `None` if it would overflow `usize`.
///
/// # Examples
///
/// ```
/// use base64_ng::checked_encoded_len;
///
/// assert_eq!(checked_encoded_len(5, true), Some(8));
/// assert_eq!(checked_encoded_len(usize::MAX, true), None);
/// ```
#[must_use]
pub const fn checked_encoded_len(input_len: usize, padded: bool) -> Option<usize> {
    let groups = input_len / 3;
    if groups > usize::MAX / 4 {
        return None;
    }
    let full = groups * 4;
    let rem = input_len % 3;
    if rem == 0 {
        Some(full)
    } else if padded {
        full.checked_add(4)
    } else {
        full.checked_add(rem + 1)
    }
}

/// Returns the maximum decoded length for an encoded input length.
///
/// # Examples
///
/// ```
/// use base64_ng::decoded_capacity;
///
/// assert_eq!(decoded_capacity(8), 6);
/// assert_eq!(decoded_capacity(7), 5);
/// ```
#[must_use]
pub const fn decoded_capacity(encoded_len: usize) -> usize {
    let rem = encoded_len % 4;
    encoded_len / 4 * 3
        + if rem == 2 {
            1
        } else if rem == 3 {
            2
        } else {
            0
        }
}

/// Returns the exact decoded length implied by input length and padding.
///
/// This validates padding placement and impossible lengths, but it does not
/// validate alphabet membership or non-canonical trailing bits.
///
/// # Examples
///
/// ```
/// use base64_ng::decoded_len;
///
/// assert_eq!(decoded_len(b"aGVsbG8=", true).unwrap(), 5);
/// assert_eq!(decoded_len(b"aGVsbG8", false).unwrap(), 5);
/// ```
pub fn decoded_len(input: &[u8], padded: bool) -> Result<usize, DecodeError> {
    if padded {
        decoded_len_padded(input)
    } else {
        decoded_len_unpadded(input)
    }
}

/// Defines a custom [`Alphabet`] from a 64-byte string literal.
///
/// The generated alphabet is validated at compile time with
/// [`validate_alphabet`]. Invalid, duplicate, or padding bytes fail the build
/// instead of creating a malformed runtime profile.
///
/// The generated implementation uses the conservative default
/// [`Alphabet::encode`] behavior: every emitted Base64 byte performs a fixed
/// 64-entry scan to avoid secret-indexed table lookups. Built-in alphabets use
/// optimized arithmetic mappers.
///
/// # Examples
///
/// ```
/// base64_ng::define_alphabet! {
///     struct DotSlash = b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
/// }
///
/// let engine = base64_ng::Engine::<DotSlash, false>::new();
/// let mut encoded = [0u8; 4];
/// let written = engine.encode_slice(&[0xff, 0xff, 0xff], &mut encoded).unwrap();
/// assert_eq!(&encoded[..written], b"9999");
/// ```
///
/// Invalid alphabets fail during compilation:
///
/// ```compile_fail
/// base64_ng::define_alphabet! {
///     struct Bad = b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
/// }
/// ```
#[macro_export]
macro_rules! define_alphabet {
    ($(#[$meta:meta])* $vis:vis struct $name:ident = $encode:expr;) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
        $vis struct $name;

        impl $crate::Alphabet for $name {
            const ENCODE: [u8; 64] = *$encode;

            #[inline]
            fn decode(byte: u8) -> Option<u8> {
                $crate::decode_alphabet_byte(byte, &Self::ENCODE)
            }
        }

        const _: [(); 1] = [(); match $crate::validate_alphabet(
            &<$name as $crate::Alphabet>::ENCODE,
        ) {
            Ok(()) => 1,
            Err(_) => 0,
        }];
    };
}

/// Validates a 64-byte Base64 alphabet table.
///
/// A valid alphabet must contain exactly 64 unique visible ASCII bytes and must
/// not contain the padding byte `=`.
///
/// # Examples
///
/// ```
/// use base64_ng::{Alphabet, Standard, validate_alphabet};
///
/// validate_alphabet(&Standard::ENCODE).unwrap();
/// ```
pub const fn validate_alphabet(encode: &[u8; 64]) -> Result<(), AlphabetError> {
    let mut index = 0;
    while index < encode.len() {
        let byte = encode[index];
        if !is_visible_ascii(byte) {
            return Err(AlphabetError::InvalidByte { index, byte });
        }
        if byte == b'=' {
            return Err(AlphabetError::PaddingByte { index });
        }

        let mut duplicate = index + 1;
        while duplicate < encode.len() {
            if encode[duplicate] == byte {
                return Err(AlphabetError::DuplicateByte {
                    first: index,
                    second: duplicate,
                    byte,
                });
            }
            duplicate += 1;
        }

        index += 1;
    }

    Ok(())
}

/// Decodes one byte by scanning a caller-provided alphabet table.
///
/// This helper is intended for custom [`Alphabet`] implementations. Validate
/// the table with [`validate_alphabet`] before trusting the alphabet in a
/// protocol or public API.
///
/// # Examples
///
/// ```
/// use base64_ng::{Alphabet, decode_alphabet_byte};
///
/// struct DotSlash;
///
/// impl Alphabet for DotSlash {
///     const ENCODE: [u8; 64] =
///         *b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
///
///     fn decode(byte: u8) -> Option<u8> {
///         decode_alphabet_byte(byte, &Self::ENCODE)
///     }
/// }
///
/// assert_eq!(DotSlash::decode(b'.'), Some(0));
/// assert_eq!(DotSlash::decode(b'9'), Some(63));
/// ```
#[must_use]
pub const fn decode_alphabet_byte(byte: u8, encode: &[u8; 64]) -> Option<u8> {
    let mut index = 0;
    let mut value = 0;
    while index < encode.len() {
        if encode[index] == byte {
            return Some(value);
        }
        index += 1;
        value += 1;
    }
    None
}

/// A Base64 alphabet.
pub trait Alphabet {
    /// Encoding table indexed by 6-bit values.
    const ENCODE: [u8; 64];

    /// Encode one 6-bit value into an alphabet byte.
    ///
    /// The default implementation scans the alphabet table instead of using a
    /// secret-indexed table lookup. Built-in alphabets override this with the
    /// branch-minimized ASCII arithmetic mapper. Custom alphabets that keep the
    /// default method prioritize timing posture over throughput: every emitted
    /// Base64 byte performs a fixed 64-entry scan. For massive payloads with
    /// user-defined alphabets, profile this cost and consider an audited custom
    /// override only if the alphabet has a structure that can be mapped without
    /// secret-indexed table access.
    #[must_use]
    fn encode(value: u8) -> u8 {
        encode_alphabet_value(value, &Self::ENCODE)
    }

    /// Decode one byte into a 6-bit value.
    fn decode(byte: u8) -> Option<u8>;
}

const fn is_visible_ascii(byte: u8) -> bool {
    byte >= 0x21 && byte <= 0x7e
}

/// The RFC 4648 standard Base64 alphabet.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Standard;

impl Alphabet for Standard {
    const ENCODE: [u8; 64] = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    #[inline]
    fn encode(value: u8) -> u8 {
        encode_ascii_base64(value, Self::ENCODE[62], Self::ENCODE[63])
    }

    #[inline]
    fn decode(byte: u8) -> Option<u8> {
        decode_ascii_base64(byte, Self::ENCODE[62], Self::ENCODE[63])
    }
}

/// The RFC 4648 URL-safe Base64 alphabet.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UrlSafe;

impl Alphabet for UrlSafe {
    const ENCODE: [u8; 64] = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

    #[inline]
    fn encode(value: u8) -> u8 {
        encode_ascii_base64(value, Self::ENCODE[62], Self::ENCODE[63])
    }

    #[inline]
    fn decode(byte: u8) -> Option<u8> {
        decode_ascii_base64(byte, Self::ENCODE[62], Self::ENCODE[63])
    }
}

/// The bcrypt Base64 alphabet.
///
/// This alphabet is commonly used by bcrypt hash strings. It is provided as an
/// alphabet/profile building block; `base64-ng` does not parse or verify full
/// bcrypt password-hash records.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Bcrypt;

impl Alphabet for Bcrypt {
    const ENCODE: [u8; 64] = *b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

    #[inline]
    fn decode(byte: u8) -> Option<u8> {
        decode_alphabet_byte(byte, &Self::ENCODE)
    }
}

/// The Unix `crypt(3)` Base64 alphabet.
///
/// This alphabet is provided as an explicit legacy interoperability profile.
/// `base64-ng` does not parse or verify complete password-hash records.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Crypt;

impl Alphabet for Crypt {
    const ENCODE: [u8; 64] = *b"./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

    #[inline]
    fn decode(byte: u8) -> Option<u8> {
        decode_alphabet_byte(byte, &Self::ENCODE)
    }
}

#[inline]
const fn encode_base64_value<A: Alphabet>(value: u8) -> u8 {
    encode_alphabet_value(value, &A::ENCODE)
}

#[inline]
fn encode_base64_value_runtime<A: Alphabet>(value: u8) -> u8 {
    A::encode(value)
}

#[inline]
const fn encode_alphabet_value(value: u8, encode: &[u8; 64]) -> u8 {
    let mut output = 0;
    let mut index = 0;
    let mut candidate = 0;
    while index < encode.len() {
        output |= encode[index] & ct_mask_eq_u8(value, candidate);
        index += 1;
        candidate += 1;
    }
    output
}

#[inline]
const fn encode_ascii_base64(value: u8, value_62_byte: u8, value_63_byte: u8) -> u8 {
    let upper = ct_mask_lt_u8(value, 26);
    let lower = ct_mask_lt_u8(value.wrapping_sub(26), 26);
    let digit = ct_mask_lt_u8(value.wrapping_sub(52), 10);
    let value_62 = ct_mask_eq_u8(value, 0x3e);
    let value_63 = ct_mask_eq_u8(value, 0x3f);

    (value.wrapping_add(b'A') & upper)
        | (value.wrapping_sub(26).wrapping_add(b'a') & lower)
        | (value.wrapping_sub(52).wrapping_add(b'0') & digit)
        | (value_62_byte & value_62)
        | (value_63_byte & value_63)
}

#[inline]
fn decode_ascii_base64(byte: u8, value_62_byte: u8, value_63_byte: u8) -> Option<u8> {
    let upper = ct_mask_lt_u8(byte.wrapping_sub(b'A'), 26);
    let lower = ct_mask_lt_u8(byte.wrapping_sub(b'a'), 26);
    let digit = ct_mask_lt_u8(byte.wrapping_sub(b'0'), 10);
    let value_62 = ct_mask_eq_u8(byte, value_62_byte);
    let value_63 = ct_mask_eq_u8(byte, value_63_byte);
    let valid = upper | lower | digit | value_62 | value_63;

    let decoded = (byte.wrapping_sub(b'A') & upper)
        | (byte.wrapping_sub(b'a').wrapping_add(26) & lower)
        | (byte.wrapping_sub(b'0').wrapping_add(52) & digit)
        | (0x3e & value_62)
        | (0x3f & value_63);

    if valid == 0 { None } else { Some(decoded) }
}

#[inline]
const fn ct_mask_bit(bit: u8) -> u8 {
    0u8.wrapping_sub(bit & 1)
}

#[inline]
const fn ct_mask_nonzero_u8(value: u8) -> u8 {
    let wide = value as u16;
    let negative = 0u16.wrapping_sub(wide);
    let nonzero = ((wide | negative) >> 8) as u8;
    ct_mask_bit(nonzero)
}

#[inline]
const fn ct_mask_eq_u8(left: u8, right: u8) -> u8 {
    !ct_mask_nonzero_u8(left ^ right)
}

#[inline]
const fn ct_mask_lt_u8(left: u8, right: u8) -> u8 {
    let diff = (left as u16).wrapping_sub(right as u16);
    ct_mask_bit((diff >> 8) as u8)
}

fn constant_time_eq_public_len(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let diff = left
        .iter()
        .zip(right)
        .fold(0u8, |diff, (left, right)| diff | (*left ^ *right));
    diff == 0
}

mod backend {
    use super::{
        Alphabet, DecodeError, EncodeError, checked_encoded_len, decode_padded, decode_unpadded,
        encode_base64_value_runtime,
    };

    pub(super) fn encode_slice<A, const PAD: bool>(
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, EncodeError>
    where
        A: Alphabet,
    {
        #[cfg(feature = "simd")]
        match super::simd::active_backend() {
            super::simd::ActiveBackend::Scalar => {}
        }

        scalar_encode_slice::<A, PAD>(input, output)
    }

    pub(super) fn decode_slice<A, const PAD: bool>(
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError>
    where
        A: Alphabet,
    {
        #[cfg(feature = "simd")]
        match super::simd::active_backend() {
            super::simd::ActiveBackend::Scalar => {}
        }

        scalar_decode_slice::<A, PAD>(input, output)
    }

    #[cfg(test)]
    pub(super) fn scalar_reference_encode_slice<A, const PAD: bool>(
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, EncodeError>
    where
        A: Alphabet,
    {
        scalar_encode_slice::<A, PAD>(input, output)
    }

    #[cfg(test)]
    pub(super) fn scalar_reference_decode_slice<A, const PAD: bool>(
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError>
    where
        A: Alphabet,
    {
        scalar_decode_slice::<A, PAD>(input, output)
    }

    fn scalar_encode_slice<A, const PAD: bool>(
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, EncodeError>
    where
        A: Alphabet,
    {
        let required = checked_encoded_len(input.len(), PAD).ok_or(EncodeError::LengthOverflow)?;
        if output.len() < required {
            return Err(EncodeError::OutputTooSmall {
                required,
                available: output.len(),
            });
        }

        let mut read = 0;
        let mut write = 0;
        while read + 3 <= input.len() {
            let b0 = input[read];
            let b1 = input[read + 1];
            let b2 = input[read + 2];

            output[write] = encode_base64_value_runtime::<A>(b0 >> 2);
            output[write + 1] =
                encode_base64_value_runtime::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
            output[write + 2] =
                encode_base64_value_runtime::<A>(((b1 & 0b0000_1111) << 2) | (b2 >> 6));
            output[write + 3] = encode_base64_value_runtime::<A>(b2 & 0b0011_1111);

            read += 3;
            write += 4;
        }

        match input.len() - read {
            0 => {}
            1 => {
                let b0 = input[read];
                output[write] = encode_base64_value_runtime::<A>(b0 >> 2);
                output[write + 1] = encode_base64_value_runtime::<A>((b0 & 0b0000_0011) << 4);
                write += 2;
                if PAD {
                    output[write] = b'=';
                    output[write + 1] = b'=';
                    write += 2;
                }
            }
            2 => {
                let b0 = input[read];
                let b1 = input[read + 1];
                output[write] = encode_base64_value_runtime::<A>(b0 >> 2);
                output[write + 1] =
                    encode_base64_value_runtime::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
                output[write + 2] = encode_base64_value_runtime::<A>((b1 & 0b0000_1111) << 2);
                write += 3;
                if PAD {
                    output[write] = b'=';
                    write += 1;
                }
            }
            _ => unreachable!(),
        }

        Ok(write)
    }

    fn scalar_decode_slice<A, const PAD: bool>(
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError>
    where
        A: Alphabet,
    {
        if input.is_empty() {
            return Ok(0);
        }

        if PAD {
            decode_padded::<A>(input, output)
        } else {
            decode_unpadded::<A>(input, output)
        }
    }
}

/// A zero-sized Base64 engine parameterized by alphabet and padding policy.
pub struct Engine<A, const PAD: bool> {
    alphabet: core::marker::PhantomData<A>,
}

impl<A, const PAD: bool> Clone for Engine<A, PAD> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, const PAD: bool> Copy for Engine<A, PAD> {}

impl<A, const PAD: bool> core::fmt::Debug for Engine<A, PAD> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("Engine")
            .field("padded", &PAD)
            .finish()
    }
}

impl<A, const PAD: bool> core::fmt::Display for Engine<A, PAD> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "padded={PAD}")
    }
}

impl<A, const PAD: bool> Default for Engine<A, PAD> {
    fn default() -> Self {
        Self {
            alphabet: core::marker::PhantomData,
        }
    }
}

impl<A, const PAD: bool> Eq for Engine<A, PAD> {}

impl<A, const PAD: bool> PartialEq for Engine<A, PAD> {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl<A, const PAD: bool> Engine<A, PAD>
where
    A: Alphabet,
{
    /// Creates a new engine value.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            alphabet: core::marker::PhantomData,
        }
    }

    /// Returns whether this engine uses padded Base64.
    #[must_use]
    pub const fn is_padded(&self) -> bool {
        PAD
    }

    /// Returns this engine as an unwrapped profile.
    ///
    /// Use [`Profile::new`] or [`Profile::checked_new`] when a strict
    /// line-wrapping policy should travel with the profile.
    #[must_use]
    pub const fn profile(&self) -> Profile<A, PAD> {
        Profile::new(*self, None)
    }

    /// Returns the matching constant-time-oriented decoder for this engine's
    /// alphabet and padding policy.
    ///
    /// The returned decoder is still an explicit opt-in to the [`ct`] module's
    /// slower, opaque-error, constant-time-oriented scalar path.
    #[must_use]
    pub const fn ct_decoder(&self) -> ct::CtEngine<A, PAD> {
        ct::CtEngine::new()
    }

    /// Returns the encoded length for this engine's padding policy.
    pub const fn encoded_len(&self, input_len: usize) -> Result<usize, EncodeError> {
        encoded_len(input_len, PAD)
    }

    /// Returns the encoded length for this engine, or `None` on overflow.
    #[must_use]
    pub const fn checked_encoded_len(&self, input_len: usize) -> Option<usize> {
        checked_encoded_len(input_len, PAD)
    }

    /// Returns the encoded length after applying a line wrapping policy.
    ///
    /// The returned length includes inserted line endings but does not include
    /// a trailing line ending after the final encoded line.
    pub const fn wrapped_encoded_len(
        &self,
        input_len: usize,
        wrap: LineWrap,
    ) -> Result<usize, EncodeError> {
        wrapped_encoded_len(input_len, PAD, wrap)
    }

    /// Returns the encoded length after line wrapping, or `None` on overflow or
    /// invalid line wrapping.
    #[must_use]
    pub const fn checked_wrapped_encoded_len(
        &self,
        input_len: usize,
        wrap: LineWrap,
    ) -> Option<usize> {
        checked_wrapped_encoded_len(input_len, PAD, wrap)
    }

    /// Returns the exact decoded length implied by input length and padding.
    ///
    /// This validates padding placement and impossible lengths, but it does not
    /// validate alphabet membership or non-canonical trailing bits.
    pub fn decoded_len(&self, input: &[u8]) -> Result<usize, DecodeError> {
        decoded_len(input, PAD)
    }

    /// Returns the exact decoded length for the explicit legacy profile.
    ///
    /// The legacy profile ignores ASCII space, tab, carriage return, and line
    /// feed bytes before applying the same alphabet, padding, and canonical-bit
    /// checks as strict decoding.
    pub fn decoded_len_legacy(&self, input: &[u8]) -> Result<usize, DecodeError> {
        validate_legacy_decode::<A, PAD>(input)
    }

    /// Returns the exact decoded length for a line-wrapped profile.
    ///
    /// The wrapped profile accepts only the configured line ending. Non-final
    /// lines must contain exactly `wrap.line_len` encoded bytes; the final line
    /// may be shorter. A single trailing line ending after the final line is
    /// accepted.
    pub fn decoded_len_wrapped(&self, input: &[u8], wrap: LineWrap) -> Result<usize, DecodeError> {
        validate_wrapped_decode::<A, PAD>(input, wrap)
    }

    /// Validates strict Base64 input without writing decoded bytes.
    ///
    /// This applies the same alphabet, padding, and canonical-bit checks as
    /// [`Self::decode_slice`]. Use this method when malformed-input
    /// diagnostics matter; use [`Self::validate`] when a boolean is enough.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// STANDARD.validate_result(b"aGVsbG8=").unwrap();
    /// assert!(STANDARD.validate_result(b"aGVsbG8").is_err());
    /// ```
    pub fn validate_result(&self, input: &[u8]) -> Result<(), DecodeError> {
        validate_decode::<A, PAD>(input).map(|_| ())
    }

    /// Returns whether `input` is valid strict Base64 for this engine.
    ///
    /// This is a convenience wrapper around [`Self::validate_result`].
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::URL_SAFE_NO_PAD;
    ///
    /// assert!(URL_SAFE_NO_PAD.validate(b"-_8"));
    /// assert!(!URL_SAFE_NO_PAD.validate(b"+/8"));
    /// ```
    #[must_use]
    pub fn validate(&self, input: &[u8]) -> bool {
        self.validate_result(input).is_ok()
    }

    /// Validates input using the explicit legacy whitespace profile.
    ///
    /// ASCII space, tab, carriage return, and line feed bytes are ignored
    /// before applying the same alphabet, padding, and canonical-bit checks as
    /// strict decoding.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// STANDARD.validate_legacy_result(b" aG\r\nVsbG8= ").unwrap();
    /// assert!(STANDARD.validate_legacy_result(b" aG-=").is_err());
    /// ```
    pub fn validate_legacy_result(&self, input: &[u8]) -> Result<(), DecodeError> {
        validate_legacy_decode::<A, PAD>(input).map(|_| ())
    }

    /// Returns whether `input` is valid for the explicit legacy whitespace
    /// profile.
    ///
    /// This is a convenience wrapper around [`Self::validate_legacy_result`].
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// assert!(STANDARD.validate_legacy(b" aG\r\nVsbG8= "));
    /// assert!(!STANDARD.validate_legacy(b"aG-V"));
    /// ```
    #[must_use]
    pub fn validate_legacy(&self, input: &[u8]) -> bool {
        self.validate_legacy_result(input).is_ok()
    }

    /// Validates input using a strict line-wrapped profile.
    ///
    /// This is stricter than [`Self::validate_legacy_result`]: it accepts only
    /// the configured line ending and enforces the configured line length for
    /// every non-final line.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{LineEnding, LineWrap, STANDARD};
    ///
    /// let wrap = LineWrap::new(4, LineEnding::Lf);
    /// STANDARD.validate_wrapped_result(b"aGVs\nbG8=", wrap).unwrap();
    /// assert!(STANDARD.validate_wrapped_result(b"aG\nVsbG8=", wrap).is_err());
    /// ```
    pub fn validate_wrapped_result(&self, input: &[u8], wrap: LineWrap) -> Result<(), DecodeError> {
        validate_wrapped_decode::<A, PAD>(input, wrap).map(|_| ())
    }

    /// Returns whether `input` is valid for a strict line-wrapped profile.
    ///
    /// This is a convenience wrapper around [`Self::validate_wrapped_result`].
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{LineEnding, LineWrap, STANDARD};
    ///
    /// let wrap = LineWrap::new(4, LineEnding::Lf);
    /// assert!(STANDARD.validate_wrapped(b"aGVs\nbG8=", wrap));
    /// assert!(!STANDARD.validate_wrapped(b"aG\nVsbG8=", wrap));
    /// ```
    #[must_use]
    pub fn validate_wrapped(&self, input: &[u8], wrap: LineWrap) -> bool {
        self.validate_wrapped_result(input, wrap).is_ok()
    }

    /// Encodes a fixed-size input into a fixed-size output array in const contexts.
    ///
    /// Stable Rust does not yet allow this API to return an array whose length
    /// is computed from `INPUT_LEN` directly. Instead, the caller supplies the
    /// output length through the destination type and this function panics
    /// during const evaluation if the length is wrong.
    ///
    /// # Panics
    ///
    /// Panics if `OUTPUT_LEN` is not exactly the encoded length for `INPUT_LEN`
    /// and this engine's padding policy, or if that length overflows `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{STANDARD, URL_SAFE_NO_PAD};
    ///
    /// const HELLO: [u8; 8] = STANDARD.encode_array(b"hello");
    /// const URL_SAFE: [u8; 3] = URL_SAFE_NO_PAD.encode_array(b"\xfb\xff");
    ///
    /// assert_eq!(&HELLO, b"aGVsbG8=");
    /// assert_eq!(&URL_SAFE, b"-_8");
    /// ```
    ///
    /// Incorrect output lengths fail during const evaluation:
    ///
    /// ```compile_fail
    /// use base64_ng::STANDARD;
    ///
    /// const TOO_SHORT: [u8; 7] = STANDARD.encode_array(b"hello");
    /// ```
    #[must_use]
    pub const fn encode_array<const INPUT_LEN: usize, const OUTPUT_LEN: usize>(
        &self,
        input: &[u8; INPUT_LEN],
    ) -> [u8; OUTPUT_LEN] {
        let Some(required) = checked_encoded_len(INPUT_LEN, PAD) else {
            panic!("encoded base64 length overflows usize");
        };
        assert!(
            required == OUTPUT_LEN,
            "base64 output array has incorrect length"
        );

        let mut output = [0u8; OUTPUT_LEN];
        let mut read = 0;
        let mut write = 0;
        while INPUT_LEN - read >= 3 {
            let b0 = input[read];
            let b1 = input[read + 1];
            let b2 = input[read + 2];

            output[write] = encode_base64_value::<A>(b0 >> 2);
            output[write + 1] = encode_base64_value::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
            output[write + 2] = encode_base64_value::<A>(((b1 & 0b0000_1111) << 2) | (b2 >> 6));
            output[write + 3] = encode_base64_value::<A>(b2 & 0b0011_1111);

            read += 3;
            write += 4;
        }

        match INPUT_LEN - read {
            0 => {}
            1 => {
                let b0 = input[read];
                output[write] = encode_base64_value::<A>(b0 >> 2);
                output[write + 1] = encode_base64_value::<A>((b0 & 0b0000_0011) << 4);
                write += 2;
                if PAD {
                    output[write] = b'=';
                    output[write + 1] = b'=';
                }
            }
            2 => {
                let b0 = input[read];
                let b1 = input[read + 1];
                output[write] = encode_base64_value::<A>(b0 >> 2);
                output[write + 1] = encode_base64_value::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
                output[write + 2] = encode_base64_value::<A>((b1 & 0b0000_1111) << 2);
                if PAD {
                    output[write + 3] = b'=';
                }
            }
            _ => unreachable!(),
        }

        output
    }

    /// Encodes `input` into `output`, returning the number of bytes written.
    pub fn encode_slice(&self, input: &[u8], output: &mut [u8]) -> Result<usize, EncodeError> {
        backend::encode_slice::<A, PAD>(input, output)
    }

    /// Encodes `input` into `output` with line wrapping.
    ///
    /// The wrapping policy inserts line endings between encoded lines and does
    /// not append a trailing line ending after the final line.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{LineEnding, LineWrap, STANDARD};
    ///
    /// let wrap = LineWrap::new(4, LineEnding::Lf);
    /// let mut output = [0u8; 9];
    /// let written = STANDARD
    ///     .encode_slice_wrapped(b"hello", &mut output, wrap)
    ///     .unwrap();
    ///
    /// assert_eq!(&output[..written], b"aGVs\nbG8=");
    /// ```
    pub fn encode_slice_wrapped(
        &self,
        input: &[u8],
        output: &mut [u8],
        wrap: LineWrap,
    ) -> Result<usize, EncodeError> {
        let required = self.wrapped_encoded_len(input.len(), wrap)?;
        if output.len() < required {
            return Err(EncodeError::OutputTooSmall {
                required,
                available: output.len(),
            });
        }

        let encoded_len =
            checked_encoded_len(input.len(), PAD).ok_or(EncodeError::LengthOverflow)?;
        if encoded_len == 0 {
            return Ok(0);
        }

        if output.len() < required.saturating_add(encoded_len) {
            let mut scratch = [0u8; 1024];
            let mut input_offset = 0;
            let mut output_offset = 0;
            let mut column = 0;

            while input_offset < input.len() {
                let remaining = input.len() - input_offset;
                let mut take = remaining.min(768);
                if remaining > take {
                    take -= take % 3;
                }
                if take == 0 {
                    take = remaining;
                }

                let encoded =
                    self.encode_slice(&input[input_offset..input_offset + take], &mut scratch)?;
                write_wrapped_bytes(
                    &scratch[..encoded],
                    output,
                    &mut output_offset,
                    &mut column,
                    wrap,
                );
                wipe_bytes(&mut scratch[..encoded]);
                input_offset += take;
            }

            Ok(output_offset)
        } else {
            let encoded =
                self.encode_slice(input, &mut output[required..required + encoded_len])?;
            let mut output_offset = 0;
            let mut column = 0;
            let mut read = required;
            while read < required + encoded {
                let byte = output[read];
                write_wrapped_byte(byte, output, &mut output_offset, &mut column, wrap);
                read += 1;
            }
            wipe_bytes(&mut output[required..required + encoded]);
            Ok(output_offset)
        }
    }

    /// Encodes `input` with line wrapping and clears all bytes after the
    /// encoded prefix.
    ///
    /// If encoding fails, the entire output buffer is cleared before the error
    /// is returned.
    pub fn encode_slice_wrapped_clear_tail(
        &self,
        input: &[u8],
        output: &mut [u8],
        wrap: LineWrap,
    ) -> Result<usize, EncodeError> {
        let written = match self.encode_slice_wrapped(input, output, wrap) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(output);
                return Err(err);
            }
        };
        wipe_tail(output, written);
        Ok(written)
    }

    /// Encodes `input` with line wrapping into a stack-backed buffer.
    ///
    /// This is useful for MIME/PEM-style protocols where heap allocation is
    /// unnecessary. If encoding fails, the internal backing array is cleared
    /// before the error is returned.
    pub fn encode_wrapped_buffer<const CAP: usize>(
        &self,
        input: &[u8],
        wrap: LineWrap,
    ) -> Result<EncodedBuffer<CAP>, EncodeError> {
        let mut output = EncodedBuffer::new();
        let written = match self.encode_slice_wrapped_clear_tail(input, &mut output.bytes, wrap) {
            Ok(written) => written,
            Err(err) => {
                output.clear();
                return Err(err);
            }
        };
        output.len = written;
        Ok(output)
    }

    /// Encodes `input` with line wrapping into a newly allocated byte vector.
    #[cfg(feature = "alloc")]
    pub fn encode_wrapped_vec(
        &self,
        input: &[u8],
        wrap: LineWrap,
    ) -> Result<alloc::vec::Vec<u8>, EncodeError> {
        let required = self.wrapped_encoded_len(input.len(), wrap)?;
        let mut output = alloc::vec![0; required];
        let written = self.encode_slice_wrapped(input, &mut output, wrap)?;
        output.truncate(written);
        Ok(output)
    }

    /// Encodes `input` with line wrapping into a newly allocated UTF-8 string.
    #[cfg(feature = "alloc")]
    pub fn encode_wrapped_string(
        &self,
        input: &[u8],
        wrap: LineWrap,
    ) -> Result<alloc::string::String, EncodeError> {
        let output = self.encode_wrapped_vec(input, wrap)?;
        match alloc::string::String::from_utf8(output) {
            Ok(output) => Ok(output),
            Err(_) => unreachable!("base64 encoder produced non-UTF-8 output"),
        }
    }

    /// Encodes `input` with line wrapping into a redacted owned secret buffer.
    ///
    /// This is useful when the wrapped encoded representation itself is
    /// sensitive and should not be accidentally logged through formatting.
    #[cfg(feature = "alloc")]
    pub fn encode_wrapped_secret(
        &self,
        input: &[u8],
        wrap: LineWrap,
    ) -> Result<SecretBuffer, EncodeError> {
        self.encode_wrapped_vec(input, wrap)
            .map(SecretBuffer::from_vec)
    }

    /// Encodes `input` into `output` and clears all bytes after the encoded
    /// prefix.
    ///
    /// If encoding fails, the entire output buffer is cleared before the error
    /// is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut output = [0xff; 12];
    /// let written = STANDARD
    ///     .encode_slice_clear_tail(b"hello", &mut output)
    ///     .unwrap();
    ///
    /// assert_eq!(&output[..written], b"aGVsbG8=");
    /// assert!(output[written..].iter().all(|byte| *byte == 0));
    /// ```
    pub fn encode_slice_clear_tail(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, EncodeError> {
        let written = match self.encode_slice(input, output) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(output);
                return Err(err);
            }
        };
        wipe_tail(output, written);
        Ok(written)
    }

    /// Encodes `input` into a stack-backed buffer.
    ///
    /// This helper is useful for short values where callers want the
    /// convenience of an owned result without enabling `alloc`.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let encoded = STANDARD.encode_buffer::<8>(b"hello").unwrap();
    ///
    /// assert_eq!(encoded.as_str(), "aGVsbG8=");
    /// ```
    pub fn encode_buffer<const CAP: usize>(
        &self,
        input: &[u8],
    ) -> Result<EncodedBuffer<CAP>, EncodeError> {
        let mut output = EncodedBuffer::new();
        let written = match self.encode_slice_clear_tail(input, &mut output.bytes) {
            Ok(written) => written,
            Err(err) => {
                output.clear();
                return Err(err);
            }
        };
        output.len = written;
        Ok(output)
    }

    /// Encodes `input` into a newly allocated byte vector.
    #[cfg(feature = "alloc")]
    pub fn encode_vec(&self, input: &[u8]) -> Result<alloc::vec::Vec<u8>, EncodeError> {
        let required = checked_encoded_len(input.len(), PAD).ok_or(EncodeError::LengthOverflow)?;
        let mut output = alloc::vec![0; required];
        let written = self.encode_slice(input, &mut output)?;
        output.truncate(written);
        Ok(output)
    }

    /// Encodes `input` into a redacted owned secret buffer.
    ///
    /// This is useful when the encoded representation itself is sensitive and
    /// should not be accidentally logged through formatting.
    #[cfg(feature = "alloc")]
    pub fn encode_secret(&self, input: &[u8]) -> Result<SecretBuffer, EncodeError> {
        self.encode_vec(input).map(SecretBuffer::from_vec)
    }

    /// Encodes `input` into a newly allocated UTF-8 string.
    ///
    /// Base64 output is ASCII by construction. This helper is available with
    /// the `alloc` feature and has the same encoding semantics as
    /// [`Self::encode_slice`].
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{STANDARD, URL_SAFE_NO_PAD};
    ///
    /// assert_eq!(STANDARD.encode_string(b"hello").unwrap(), "aGVsbG8=");
    /// assert_eq!(URL_SAFE_NO_PAD.encode_string(b"\xfb\xff").unwrap(), "-_8");
    /// ```
    #[cfg(feature = "alloc")]
    pub fn encode_string(&self, input: &[u8]) -> Result<alloc::string::String, EncodeError> {
        let output = self.encode_vec(input)?;
        match alloc::string::String::from_utf8(output) {
            Ok(output) => Ok(output),
            Err(_) => unreachable!("base64 encoder produced non-UTF-8 output"),
        }
    }

    /// Encodes the first `input_len` bytes of `buffer` in place.
    ///
    /// The buffer must have enough spare capacity for the encoded output. The
    /// implementation writes from right to left, so unread input bytes are not
    /// overwritten before they are encoded.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut buffer = [0u8; 8];
    /// buffer[..5].copy_from_slice(b"hello");
    /// let encoded = STANDARD.encode_in_place(&mut buffer, 5).unwrap();
    /// assert_eq!(encoded, b"aGVsbG8=");
    /// ```
    pub fn encode_in_place<'a>(
        &self,
        buffer: &'a mut [u8],
        input_len: usize,
    ) -> Result<&'a mut [u8], EncodeError> {
        if input_len > buffer.len() {
            return Err(EncodeError::InputTooLarge {
                input_len,
                buffer_len: buffer.len(),
            });
        }

        let required = checked_encoded_len(input_len, PAD).ok_or(EncodeError::LengthOverflow)?;
        if buffer.len() < required {
            return Err(EncodeError::OutputTooSmall {
                required,
                available: buffer.len(),
            });
        }

        let mut read = input_len;
        let mut write = required;

        match input_len % 3 {
            0 => {}
            1 => {
                read -= 1;
                let b0 = buffer[read];
                if PAD {
                    write -= 4;
                    buffer[write] = encode_base64_value_runtime::<A>(b0 >> 2);
                    buffer[write + 1] = encode_base64_value_runtime::<A>((b0 & 0b0000_0011) << 4);
                    buffer[write + 2] = b'=';
                    buffer[write + 3] = b'=';
                } else {
                    write -= 2;
                    buffer[write] = encode_base64_value_runtime::<A>(b0 >> 2);
                    buffer[write + 1] = encode_base64_value_runtime::<A>((b0 & 0b0000_0011) << 4);
                }
            }
            2 => {
                read -= 2;
                let b0 = buffer[read];
                let b1 = buffer[read + 1];
                if PAD {
                    write -= 4;
                    buffer[write] = encode_base64_value_runtime::<A>(b0 >> 2);
                    buffer[write + 1] =
                        encode_base64_value_runtime::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
                    buffer[write + 2] = encode_base64_value_runtime::<A>((b1 & 0b0000_1111) << 2);
                    buffer[write + 3] = b'=';
                } else {
                    write -= 3;
                    buffer[write] = encode_base64_value_runtime::<A>(b0 >> 2);
                    buffer[write + 1] =
                        encode_base64_value_runtime::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
                    buffer[write + 2] = encode_base64_value_runtime::<A>((b1 & 0b0000_1111) << 2);
                }
            }
            _ => unreachable!(),
        }

        while read > 0 {
            read -= 3;
            write -= 4;
            let b0 = buffer[read];
            let b1 = buffer[read + 1];
            let b2 = buffer[read + 2];

            buffer[write] = encode_base64_value_runtime::<A>(b0 >> 2);
            buffer[write + 1] =
                encode_base64_value_runtime::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
            buffer[write + 2] =
                encode_base64_value_runtime::<A>(((b1 & 0b0000_1111) << 2) | (b2 >> 6));
            buffer[write + 3] = encode_base64_value_runtime::<A>(b2 & 0b0011_1111);
        }

        debug_assert_eq!(write, 0);
        Ok(&mut buffer[..required])
    }

    /// Encodes the first `input_len` bytes of `buffer` in place and clears all
    /// bytes after the encoded prefix.
    ///
    /// If encoding fails because `input_len` is too large, the output buffer is
    /// too small, or the encoded length overflows `usize`, the entire buffer is
    /// cleared before the error is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut buffer = [0xff; 12];
    /// buffer[..5].copy_from_slice(b"hello");
    /// let encoded = STANDARD.encode_in_place_clear_tail(&mut buffer, 5).unwrap();
    /// assert_eq!(encoded, b"aGVsbG8=");
    /// ```
    pub fn encode_in_place_clear_tail<'a>(
        &self,
        buffer: &'a mut [u8],
        input_len: usize,
    ) -> Result<&'a mut [u8], EncodeError> {
        let len = match self.encode_in_place(buffer, input_len) {
            Ok(encoded) => encoded.len(),
            Err(err) => {
                wipe_bytes(buffer);
                return Err(err);
            }
        };
        wipe_tail(buffer, len);
        Ok(&mut buffer[..len])
    }

    /// Decodes `input` into `output`, returning the number of bytes written.
    ///
    /// This is strict decoding. Whitespace, mixed alphabets, malformed padding,
    /// and trailing non-padding data are rejected.
    pub fn decode_slice(&self, input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
        backend::decode_slice::<A, PAD>(input, output)
    }

    /// Decodes `input` into `output` and clears all bytes after the decoded
    /// prefix.
    ///
    /// If decoding fails, the entire output buffer is cleared before the error
    /// is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut output = [0xff; 8];
    /// let written = STANDARD
    ///     .decode_slice_clear_tail(b"aGk=", &mut output)
    ///     .unwrap();
    ///
    /// assert_eq!(&output[..written], b"hi");
    /// assert!(output[written..].iter().all(|byte| *byte == 0));
    /// ```
    pub fn decode_slice_clear_tail(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError> {
        let written = match self.decode_slice(input, output) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(output);
                return Err(err);
            }
        };
        wipe_tail(output, written);
        Ok(written)
    }

    /// Decodes `input` into a stack-backed buffer.
    ///
    /// This helper is useful for short decoded values where callers want the
    /// convenience of an owned result without enabling `alloc`.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let decoded = STANDARD.decode_buffer::<5>(b"aGVsbG8=").unwrap();
    ///
    /// assert_eq!(decoded.as_bytes(), b"hello");
    /// ```
    pub fn decode_buffer<const CAP: usize>(
        &self,
        input: &[u8],
    ) -> Result<DecodedBuffer<CAP>, DecodeError> {
        let mut output = DecodedBuffer::new();
        let written = match self.decode_slice_clear_tail(input, &mut output.bytes) {
            Ok(written) => written,
            Err(err) => {
                output.clear();
                return Err(err);
            }
        };
        output.len = written;
        Ok(output)
    }

    /// Decodes `input` using the explicit legacy whitespace profile.
    ///
    /// ASCII space, tab, carriage return, and line feed bytes are ignored.
    /// Alphabet selection, padding placement, trailing data after padding, and
    /// non-canonical trailing bits remain strict.
    pub fn decode_slice_legacy(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError> {
        let required = validate_legacy_decode::<A, PAD>(input)?;
        if output.len() < required {
            return Err(DecodeError::OutputTooSmall {
                required,
                available: output.len(),
            });
        }
        decode_legacy_to_slice::<A, PAD>(input, output)
    }

    /// Decodes `input` using the explicit legacy whitespace profile and clears
    /// all bytes after the decoded prefix.
    ///
    /// If validation or decoding fails, the entire output buffer is cleared
    /// before the error is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut output = [0xff; 8];
    /// let written = STANDARD
    ///     .decode_slice_legacy_clear_tail(b" aG\r\nk= ", &mut output)
    ///     .unwrap();
    ///
    /// assert_eq!(&output[..written], b"hi");
    /// assert!(output[written..].iter().all(|byte| *byte == 0));
    /// ```
    pub fn decode_slice_legacy_clear_tail(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError> {
        let written = match self.decode_slice_legacy(input, output) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(output);
                return Err(err);
            }
        };
        wipe_tail(output, written);
        Ok(written)
    }

    /// Decodes `input` into a stack-backed buffer using the explicit legacy
    /// whitespace profile.
    ///
    /// ASCII space, tab, carriage return, and line feed bytes are ignored.
    /// Alphabet selection, padding placement, trailing data after padding, and
    /// non-canonical trailing bits remain strict. If decoding fails, the
    /// internal backing array is cleared before the error is returned.
    pub fn decode_buffer_legacy<const CAP: usize>(
        &self,
        input: &[u8],
    ) -> Result<DecodedBuffer<CAP>, DecodeError> {
        let mut output = DecodedBuffer::new();
        let written = match self.decode_slice_legacy_clear_tail(input, &mut output.bytes) {
            Ok(written) => written,
            Err(err) => {
                output.clear();
                return Err(err);
            }
        };
        output.len = written;
        Ok(output)
    }

    /// Decodes `input` using a strict line-wrapped profile.
    ///
    /// The wrapped profile accepts only the configured line ending. Non-final
    /// lines must contain exactly `wrap.line_len` encoded bytes; the final line
    /// may be shorter. A single trailing line ending after the final line is
    /// accepted.
    pub fn decode_slice_wrapped(
        &self,
        input: &[u8],
        output: &mut [u8],
        wrap: LineWrap,
    ) -> Result<usize, DecodeError> {
        let required = validate_wrapped_decode::<A, PAD>(input, wrap)?;
        if output.len() < required {
            return Err(DecodeError::OutputTooSmall {
                required,
                available: output.len(),
            });
        }
        decode_wrapped_to_slice::<A, PAD>(input, output, wrap)
    }

    /// Decodes `input` using a strict line-wrapped profile and clears all bytes
    /// after the decoded prefix.
    ///
    /// If validation or decoding fails, the entire output buffer is cleared
    /// before the error is returned.
    pub fn decode_slice_wrapped_clear_tail(
        &self,
        input: &[u8],
        output: &mut [u8],
        wrap: LineWrap,
    ) -> Result<usize, DecodeError> {
        let written = match self.decode_slice_wrapped(input, output, wrap) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(output);
                return Err(err);
            }
        };
        wipe_tail(output, written);
        Ok(written)
    }

    /// Decodes `input` using a strict line-wrapped profile into a stack-backed
    /// buffer.
    ///
    /// The wrapped profile accepts only the configured line ending. Non-final
    /// lines must contain exactly `wrap.line_len` encoded bytes; the final line
    /// may be shorter. A single trailing line ending after the final line is
    /// accepted. If decoding fails, the internal backing array is cleared
    /// before the error is returned.
    pub fn decode_wrapped_buffer<const CAP: usize>(
        &self,
        input: &[u8],
        wrap: LineWrap,
    ) -> Result<DecodedBuffer<CAP>, DecodeError> {
        let mut output = DecodedBuffer::new();
        let written = match self.decode_slice_wrapped_clear_tail(input, &mut output.bytes, wrap) {
            Ok(written) => written,
            Err(err) => {
                output.clear();
                return Err(err);
            }
        };
        output.len = written;
        Ok(output)
    }

    /// Decodes `input` into a newly allocated byte vector.
    ///
    /// This is strict decoding with the same semantics as [`Self::decode_slice`].
    #[cfg(feature = "alloc")]
    pub fn decode_vec(&self, input: &[u8]) -> Result<alloc::vec::Vec<u8>, DecodeError> {
        let required = validate_decode::<A, PAD>(input)?;
        let mut output = alloc::vec![0; required];
        let written = match self.decode_slice(input, &mut output) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(&mut output);
                return Err(err);
            }
        };
        output.truncate(written);
        Ok(output)
    }

    /// Decodes `input` into a redacted owned secret buffer.
    ///
    /// On malformed input, the intermediate output buffer is cleared before the
    /// error is returned by [`Self::decode_vec`].
    #[cfg(feature = "alloc")]
    pub fn decode_secret(&self, input: &[u8]) -> Result<SecretBuffer, DecodeError> {
        self.decode_vec(input).map(SecretBuffer::from_vec)
    }

    /// Decodes `input` into a newly allocated byte vector using the explicit
    /// legacy whitespace profile.
    #[cfg(feature = "alloc")]
    pub fn decode_vec_legacy(&self, input: &[u8]) -> Result<alloc::vec::Vec<u8>, DecodeError> {
        let required = validate_legacy_decode::<A, PAD>(input)?;
        let mut output = alloc::vec![0; required];
        let written = match self.decode_slice_legacy(input, &mut output) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(&mut output);
                return Err(err);
            }
        };
        output.truncate(written);
        Ok(output)
    }

    /// Decodes `input` into a redacted owned secret buffer using the explicit
    /// legacy whitespace profile.
    ///
    /// ASCII space, tab, carriage return, and line feed bytes are ignored.
    /// Alphabet selection, padding placement, trailing data after padding, and
    /// non-canonical trailing bits remain strict.
    #[cfg(feature = "alloc")]
    pub fn decode_secret_legacy(&self, input: &[u8]) -> Result<SecretBuffer, DecodeError> {
        self.decode_vec_legacy(input).map(SecretBuffer::from_vec)
    }

    /// Decodes line-wrapped input into a newly allocated byte vector.
    #[cfg(feature = "alloc")]
    pub fn decode_wrapped_vec(
        &self,
        input: &[u8],
        wrap: LineWrap,
    ) -> Result<alloc::vec::Vec<u8>, DecodeError> {
        let required = validate_wrapped_decode::<A, PAD>(input, wrap)?;
        let mut output = alloc::vec![0; required];
        let written = match self.decode_slice_wrapped(input, &mut output, wrap) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(&mut output);
                return Err(err);
            }
        };
        output.truncate(written);
        Ok(output)
    }

    /// Decodes line-wrapped input into a redacted owned secret buffer.
    ///
    /// The wrapped profile accepts only the configured line ending. Non-final
    /// lines must contain exactly `wrap.line_len` encoded bytes; the final line
    /// may be shorter. A single trailing line ending after the final line is
    /// accepted.
    #[cfg(feature = "alloc")]
    pub fn decode_wrapped_secret(
        &self,
        input: &[u8],
        wrap: LineWrap,
    ) -> Result<SecretBuffer, DecodeError> {
        self.decode_wrapped_vec(input, wrap)
            .map(SecretBuffer::from_vec)
    }

    /// Decodes `buffer` in place using a strict line-wrapped profile.
    ///
    /// The wrapped profile accepts only the configured line ending. Non-final
    /// lines must contain exactly `wrap.line_len` encoded bytes; the final line
    /// may be shorter. A single trailing line ending after the final line is
    /// accepted. If validation fails, the buffer contents are unspecified.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{LineEnding, LineWrap, STANDARD};
    ///
    /// let mut buffer = *b"aGVs\nbG8=";
    /// let decoded = STANDARD
    ///     .decode_in_place_wrapped(&mut buffer, LineWrap::new(4, LineEnding::Lf))
    ///     .unwrap();
    ///
    /// assert_eq!(decoded, b"hello");
    /// ```
    pub fn decode_in_place_wrapped<'a>(
        &self,
        buffer: &'a mut [u8],
        wrap: LineWrap,
    ) -> Result<&'a mut [u8], DecodeError> {
        let _required = validate_wrapped_decode::<A, PAD>(buffer, wrap)?;
        let compacted = compact_wrapped_input(buffer, wrap)?;
        let len = Self::decode_slice_to_start(&mut buffer[..compacted])?;
        Ok(&mut buffer[..len])
    }

    /// Decodes `buffer` in place using a strict line-wrapped profile and clears
    /// all bytes after the decoded prefix.
    ///
    /// If validation or decoding fails, the entire buffer is cleared before the
    /// error is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{LineEnding, LineWrap, STANDARD};
    ///
    /// let mut buffer = *b"aGVs\nbG8=";
    /// let len = STANDARD
    ///     .decode_in_place_wrapped_clear_tail(&mut buffer, LineWrap::new(4, LineEnding::Lf))
    ///     .unwrap()
    ///     .len();
    ///
    /// assert_eq!(&buffer[..len], b"hello");
    /// assert!(buffer[len..].iter().all(|byte| *byte == 0));
    /// ```
    pub fn decode_in_place_wrapped_clear_tail<'a>(
        &self,
        buffer: &'a mut [u8],
        wrap: LineWrap,
    ) -> Result<&'a mut [u8], DecodeError> {
        if let Err(err) = validate_wrapped_decode::<A, PAD>(buffer, wrap) {
            wipe_bytes(buffer);
            return Err(err);
        }

        let compacted = match compact_wrapped_input(buffer, wrap) {
            Ok(compacted) => compacted,
            Err(err) => {
                wipe_bytes(buffer);
                return Err(err);
            }
        };

        let len = match Self::decode_slice_to_start(&mut buffer[..compacted]) {
            Ok(len) => len,
            Err(err) => {
                wipe_bytes(buffer);
                return Err(err);
            }
        };
        wipe_tail(buffer, len);
        Ok(&mut buffer[..len])
    }

    /// Decodes the buffer in place and returns the decoded prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD_NO_PAD;
    ///
    /// let mut buffer = *b"Zm9vYmFy";
    /// let decoded = STANDARD_NO_PAD.decode_in_place(&mut buffer).unwrap();
    /// assert_eq!(decoded, b"foobar");
    /// ```
    pub fn decode_in_place<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], DecodeError> {
        let len = Self::decode_slice_to_start(buffer)?;
        Ok(&mut buffer[..len])
    }

    /// Decodes the buffer in place and clears all bytes after the decoded prefix.
    ///
    /// If decoding fails, the entire buffer is cleared before the error is
    /// returned. Use this variant when the encoded or partially decoded data is
    /// sensitive and the caller wants best-effort cleanup without adding a
    /// dependency.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut buffer = *b"aGk=";
    /// let decoded = STANDARD.decode_in_place_clear_tail(&mut buffer).unwrap();
    /// assert_eq!(decoded, b"hi");
    /// ```
    pub fn decode_in_place_clear_tail<'a>(
        &self,
        buffer: &'a mut [u8],
    ) -> Result<&'a mut [u8], DecodeError> {
        let len = match Self::decode_slice_to_start(buffer) {
            Ok(len) => len,
            Err(err) => {
                wipe_bytes(buffer);
                return Err(err);
            }
        };
        wipe_tail(buffer, len);
        Ok(&mut buffer[..len])
    }

    /// Decodes `buffer` in place using the explicit legacy whitespace profile.
    ///
    /// Ignored whitespace is compacted out before decoding. If validation
    /// fails, the buffer contents are unspecified.
    pub fn decode_in_place_legacy<'a>(
        &self,
        buffer: &'a mut [u8],
    ) -> Result<&'a mut [u8], DecodeError> {
        let _required = validate_legacy_decode::<A, PAD>(buffer)?;
        let mut write = 0;
        let mut read = 0;
        while read < buffer.len() {
            let byte = buffer[read];
            if !is_legacy_whitespace(byte) {
                buffer[write] = byte;
                write += 1;
            }
            read += 1;
        }
        let len = Self::decode_slice_to_start(&mut buffer[..write])?;
        Ok(&mut buffer[..len])
    }

    /// Decodes `buffer` in place using the explicit legacy whitespace profile
    /// and clears all bytes after the decoded prefix.
    ///
    /// If validation or decoding fails, the entire buffer is cleared before the
    /// error is returned.
    pub fn decode_in_place_legacy_clear_tail<'a>(
        &self,
        buffer: &'a mut [u8],
    ) -> Result<&'a mut [u8], DecodeError> {
        if let Err(err) = validate_legacy_decode::<A, PAD>(buffer) {
            wipe_bytes(buffer);
            return Err(err);
        }

        let mut write = 0;
        let mut read = 0;
        while read < buffer.len() {
            let byte = buffer[read];
            if !is_legacy_whitespace(byte) {
                buffer[write] = byte;
                write += 1;
            }
            read += 1;
        }

        let len = match Self::decode_slice_to_start(&mut buffer[..write]) {
            Ok(len) => len,
            Err(err) => {
                wipe_bytes(buffer);
                return Err(err);
            }
        };
        wipe_tail(buffer, len);
        Ok(&mut buffer[..len])
    }

    fn decode_slice_to_start(buffer: &mut [u8]) -> Result<usize, DecodeError> {
        let input_len = buffer.len();
        let mut read = 0;
        let mut write = 0;
        while read + 4 <= input_len {
            let chunk = read_quad(buffer, read)?;
            let available = buffer.len();
            let output_tail = buffer.get_mut(write..).ok_or(DecodeError::OutputTooSmall {
                required: write,
                available,
            })?;
            let written = decode_chunk::<A, PAD>(chunk, output_tail)
                .map_err(|err| err.with_index_offset(read))?;
            read += 4;
            write += written;
            if written < 3 {
                if read != input_len {
                    return Err(DecodeError::InvalidPadding { index: read - 4 });
                }
                return Ok(write);
            }
        }

        let rem = input_len - read;
        if rem == 0 {
            return Ok(write);
        }
        if PAD {
            return Err(DecodeError::InvalidLength);
        }
        let mut tail = [0u8; 3];
        tail[..rem].copy_from_slice(&buffer[read..input_len]);
        decode_tail_unpadded::<A>(&tail[..rem], &mut buffer[write..])
            .map_err(|err| err.with_index_offset(read))
            .map(|n| write + n)
    }
}

fn write_wrapped_bytes(
    input: &[u8],
    output: &mut [u8],
    output_offset: &mut usize,
    column: &mut usize,
    wrap: LineWrap,
) {
    for byte in input {
        write_wrapped_byte(*byte, output, output_offset, column, wrap);
    }
}

fn write_wrapped_byte(
    byte: u8,
    output: &mut [u8],
    output_offset: &mut usize,
    column: &mut usize,
    wrap: LineWrap,
) {
    if *column == wrap.line_len {
        let line_ending = wrap.line_ending.as_bytes();
        let mut index = 0;
        while index < line_ending.len() {
            output[*output_offset] = line_ending[index];
            *output_offset += 1;
            index += 1;
        }
        *column = 0;
    }

    output[*output_offset] = byte;
    *output_offset += 1;
    *column += 1;
}

/// Encoding error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncodeError {
    /// The encoded output length would overflow `usize`.
    LengthOverflow,
    /// The requested line wrapping policy is invalid.
    InvalidLineWrap {
        /// Requested line length.
        line_len: usize,
    },
    /// The caller-provided input length exceeds the provided buffer.
    InputTooLarge {
        /// Requested input bytes.
        input_len: usize,
        /// Available buffer bytes.
        buffer_len: usize,
    },
    /// The output buffer is too small.
    OutputTooSmall {
        /// Required output bytes.
        required: usize,
        /// Available output bytes.
        available: usize,
    },
}

impl core::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::LengthOverflow => f.write_str("base64 output length overflows usize"),
            Self::InvalidLineWrap { line_len } => {
                write!(f, "base64 line wrap length {line_len} is invalid")
            }
            Self::InputTooLarge {
                input_len,
                buffer_len,
            } => write!(
                f,
                "base64 input length {input_len} exceeds buffer length {buffer_len}"
            ),
            Self::OutputTooSmall {
                required,
                available,
            } => write!(
                f,
                "base64 output buffer too small: required {required}, available {available}"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EncodeError {}

/// Alphabet validation error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlphabetError {
    /// The alphabet contains a non-visible-ASCII byte.
    InvalidByte {
        /// Byte index in the alphabet table.
        index: usize,
        /// Invalid byte value.
        byte: u8,
    },
    /// The alphabet contains the padding byte `=`.
    PaddingByte {
        /// Byte index in the alphabet table.
        index: usize,
    },
    /// The alphabet maps more than one value to the same byte.
    DuplicateByte {
        /// First byte index.
        first: usize,
        /// Second byte index.
        second: usize,
        /// Duplicated byte value.
        byte: u8,
    },
}

impl core::fmt::Display for AlphabetError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidByte { index, byte } => {
                write!(
                    f,
                    "invalid base64 alphabet byte 0x{byte:02x} at index {index}"
                )
            }
            Self::PaddingByte { index } => {
                write!(f, "base64 alphabet contains padding byte at index {index}")
            }
            Self::DuplicateByte {
                first,
                second,
                byte,
            } => write!(
                f,
                "base64 alphabet byte 0x{byte:02x} is duplicated at indexes {first} and {second}"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AlphabetError {}

/// Decoding error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeError {
    /// The encoded input is malformed, but the decoder intentionally does not
    /// disclose a more specific error class.
    InvalidInput,
    /// The encoded input length is impossible for the selected padding policy.
    InvalidLength,
    /// A byte is not valid for the selected alphabet.
    InvalidByte {
        /// Byte index in the input.
        index: usize,
        /// Invalid byte value.
        byte: u8,
    },
    /// Padding is missing, misplaced, or non-canonical.
    InvalidPadding {
        /// Byte index where padding became invalid.
        index: usize,
    },
    /// Line wrapping is missing, misplaced, or uses the wrong line ending.
    InvalidLineWrap {
        /// Byte index where line wrapping became invalid.
        index: usize,
    },
    /// The output buffer is too small.
    OutputTooSmall {
        /// Required output bytes.
        required: usize,
        /// Available output bytes.
        available: usize,
    },
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidInput => f.write_str("malformed base64 input"),
            Self::InvalidLength => f.write_str("invalid base64 input length"),
            Self::InvalidByte { index, byte } => {
                write!(f, "invalid base64 byte 0x{byte:02x} at index {index}")
            }
            Self::InvalidPadding { index } => write!(f, "invalid base64 padding at index {index}"),
            Self::InvalidLineWrap { index } => {
                write!(f, "invalid base64 line wrapping at index {index}")
            }
            Self::OutputTooSmall {
                required,
                available,
            } => write!(
                f,
                "base64 decode output buffer too small: required {required}, available {available}"
            ),
        }
    }
}

impl DecodeError {
    fn with_index_offset(self, offset: usize) -> Self {
        match self {
            Self::InvalidByte { index, byte } => Self::InvalidByte {
                index: index + offset,
                byte,
            },
            Self::InvalidPadding { index } => Self::InvalidPadding {
                index: index + offset,
            },
            Self::InvalidLineWrap { index } => Self::InvalidLineWrap {
                index: index + offset,
            },
            Self::InvalidInput | Self::InvalidLength | Self::OutputTooSmall { .. } => self,
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DecodeError {}

fn validate_legacy_decode<A: Alphabet, const PAD: bool>(
    input: &[u8],
) -> Result<usize, DecodeError> {
    let mut chunk = [0u8; 4];
    let mut indexes = [0usize; 4];
    let mut chunk_len = 0;
    let mut required = 0;
    let mut terminal_seen = false;

    for (index, byte) in input.iter().copied().enumerate() {
        if is_legacy_whitespace(byte) {
            continue;
        }
        if terminal_seen {
            return Err(DecodeError::InvalidPadding { index });
        }

        chunk[chunk_len] = byte;
        indexes[chunk_len] = index;
        chunk_len += 1;

        if chunk_len == 4 {
            let written =
                validate_chunk::<A, PAD>(chunk).map_err(|err| map_chunk_error(err, &indexes))?;
            required += written;
            terminal_seen = written < 3;
            chunk_len = 0;
        }
    }

    if chunk_len == 0 {
        return Ok(required);
    }
    if PAD {
        return Err(DecodeError::InvalidLength);
    }

    validate_tail_unpadded::<A>(&chunk[..chunk_len])
        .map_err(|err| map_partial_chunk_error(err, &indexes, chunk_len))?;
    Ok(required + decoded_capacity(chunk_len))
}

fn decode_legacy_to_slice<A: Alphabet, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    let mut chunk = [0u8; 4];
    let mut indexes = [0usize; 4];
    let mut chunk_len = 0;
    let mut write = 0;
    let mut terminal_seen = false;

    for (index, byte) in input.iter().copied().enumerate() {
        if is_legacy_whitespace(byte) {
            continue;
        }
        if terminal_seen {
            return Err(DecodeError::InvalidPadding { index });
        }

        chunk[chunk_len] = byte;
        indexes[chunk_len] = index;
        chunk_len += 1;

        if chunk_len == 4 {
            let available = output.len();
            let output_tail = output.get_mut(write..).ok_or(DecodeError::OutputTooSmall {
                required: write,
                available,
            })?;
            let written = decode_chunk::<A, PAD>(chunk, output_tail)
                .map_err(|err| map_chunk_error(err, &indexes))?;
            write += written;
            terminal_seen = written < 3;
            chunk_len = 0;
        }
    }

    if chunk_len == 0 {
        return Ok(write);
    }
    if PAD {
        return Err(DecodeError::InvalidLength);
    }

    decode_tail_unpadded::<A>(&chunk[..chunk_len], &mut output[write..])
        .map_err(|err| map_partial_chunk_error(err, &indexes, chunk_len))
        .map(|n| write + n)
}

struct WrappedBytes<'a> {
    input: &'a [u8],
    wrap: LineWrap,
    index: usize,
    line_len: usize,
}

impl<'a> WrappedBytes<'a> {
    const fn new(input: &'a [u8], wrap: LineWrap) -> Result<Self, DecodeError> {
        if wrap.line_len == 0 {
            return Err(DecodeError::InvalidLineWrap { index: 0 });
        }
        Ok(Self {
            input,
            wrap,
            index: 0,
            line_len: 0,
        })
    }

    fn next_byte(&mut self) -> Result<Option<(usize, u8)>, DecodeError> {
        loop {
            if self.index == self.input.len() {
                return Ok(None);
            }

            if self.starts_with_line_ending() {
                let line_end_index = self.index;
                if self.line_len == 0 {
                    return Err(DecodeError::InvalidLineWrap {
                        index: line_end_index,
                    });
                }

                self.index += self.wrap.line_ending.byte_len();
                if self.index == self.input.len() {
                    self.line_len = 0;
                    return Ok(None);
                }

                if self.line_len != self.wrap.line_len {
                    return Err(DecodeError::InvalidLineWrap {
                        index: line_end_index,
                    });
                }
                self.line_len = 0;
                continue;
            }

            let byte = self.input[self.index];
            if matches!(byte, b'\r' | b'\n') {
                return Err(DecodeError::InvalidLineWrap { index: self.index });
            }

            self.line_len += 1;
            if self.line_len > self.wrap.line_len {
                return Err(DecodeError::InvalidLineWrap { index: self.index });
            }

            let index = self.index;
            self.index += 1;
            return Ok(Some((index, byte)));
        }
    }

    fn starts_with_line_ending(&self) -> bool {
        let line_ending = self.wrap.line_ending.as_bytes();
        let end = self.index + line_ending.len();
        end <= self.input.len() && &self.input[self.index..end] == line_ending
    }
}

fn validate_wrapped_decode<A: Alphabet, const PAD: bool>(
    input: &[u8],
    wrap: LineWrap,
) -> Result<usize, DecodeError> {
    let mut bytes = WrappedBytes::new(input, wrap)?;
    let mut chunk = [0u8; 4];
    let mut indexes = [0usize; 4];
    let mut chunk_len = 0;
    let mut required = 0;
    let mut terminal_seen = false;

    while let Some((index, byte)) = bytes.next_byte()? {
        if terminal_seen {
            return Err(DecodeError::InvalidPadding { index });
        }

        chunk[chunk_len] = byte;
        indexes[chunk_len] = index;
        chunk_len += 1;

        if chunk_len == 4 {
            let written =
                validate_chunk::<A, PAD>(chunk).map_err(|err| map_chunk_error(err, &indexes))?;
            required += written;
            terminal_seen = written < 3;
            chunk_len = 0;
        }
    }

    if chunk_len == 0 {
        return Ok(required);
    }
    if PAD {
        return Err(DecodeError::InvalidLength);
    }

    validate_tail_unpadded::<A>(&chunk[..chunk_len])
        .map_err(|err| map_partial_chunk_error(err, &indexes, chunk_len))?;
    Ok(required + decoded_capacity(chunk_len))
}

fn decode_wrapped_to_slice<A: Alphabet, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
    wrap: LineWrap,
) -> Result<usize, DecodeError> {
    let mut bytes = WrappedBytes::new(input, wrap)?;
    let mut chunk = [0u8; 4];
    let mut indexes = [0usize; 4];
    let mut chunk_len = 0;
    let mut write = 0;
    let mut terminal_seen = false;

    while let Some((index, byte)) = bytes.next_byte()? {
        if terminal_seen {
            return Err(DecodeError::InvalidPadding { index });
        }

        chunk[chunk_len] = byte;
        indexes[chunk_len] = index;
        chunk_len += 1;

        if chunk_len == 4 {
            let available = output.len();
            let output_tail = output.get_mut(write..).ok_or(DecodeError::OutputTooSmall {
                required: write,
                available,
            })?;
            let written = decode_chunk::<A, PAD>(chunk, output_tail)
                .map_err(|err| map_chunk_error(err, &indexes))?;
            write += written;
            terminal_seen = written < 3;
            chunk_len = 0;
        }
    }

    if chunk_len == 0 {
        return Ok(write);
    }
    if PAD {
        return Err(DecodeError::InvalidLength);
    }

    decode_tail_unpadded::<A>(&chunk[..chunk_len], &mut output[write..])
        .map_err(|err| map_partial_chunk_error(err, &indexes, chunk_len))
        .map(|n| write + n)
}

fn compact_wrapped_input(buffer: &mut [u8], wrap: LineWrap) -> Result<usize, DecodeError> {
    if !wrap.is_valid() {
        return Err(DecodeError::InvalidLineWrap { index: 0 });
    }

    let line_ending = wrap.line_ending.as_bytes();
    let line_ending_len = line_ending.len();
    let mut read = 0;
    let mut write = 0;

    while read < buffer.len() {
        let line_end = read + line_ending_len;
        if buffer.get(read..line_end) == Some(line_ending) {
            read = line_end;
            continue;
        }

        buffer[write] = buffer[read];
        write += 1;
        read += 1;
    }

    Ok(write)
}

#[inline]
const fn is_legacy_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

fn map_chunk_error(err: DecodeError, indexes: &[usize; 4]) -> DecodeError {
    match err {
        DecodeError::InvalidByte { index, byte } => DecodeError::InvalidByte {
            index: indexes[index],
            byte,
        },
        DecodeError::InvalidPadding { index } => DecodeError::InvalidPadding {
            index: indexes[index],
        },
        DecodeError::InvalidInput
        | DecodeError::InvalidLineWrap { .. }
        | DecodeError::InvalidLength
        | DecodeError::OutputTooSmall { .. } => err,
    }
}

fn map_partial_chunk_error(err: DecodeError, indexes: &[usize; 4], len: usize) -> DecodeError {
    match err {
        DecodeError::InvalidByte { index, byte } if index < len => DecodeError::InvalidByte {
            index: indexes[index],
            byte,
        },
        DecodeError::InvalidPadding { index } if index < len => DecodeError::InvalidPadding {
            index: indexes[index],
        },
        DecodeError::InvalidByte { .. }
        | DecodeError::InvalidPadding { .. }
        | DecodeError::InvalidLineWrap { .. }
        | DecodeError::InvalidInput
        | DecodeError::InvalidLength
        | DecodeError::OutputTooSmall { .. } => err,
    }
}

fn decode_padded<A: Alphabet>(input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
    if !input.len().is_multiple_of(4) {
        return Err(DecodeError::InvalidLength);
    }
    let required = decoded_len_padded(input)?;
    if output.len() < required {
        return Err(DecodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let mut read = 0;
    let mut write = 0;
    while read < input.len() {
        let chunk = read_quad(input, read)?;
        let available = output.len();
        let output_tail = output.get_mut(write..).ok_or(DecodeError::OutputTooSmall {
            required: write,
            available,
        })?;
        let written = decode_chunk::<A, true>(chunk, output_tail)
            .map_err(|err| err.with_index_offset(read))?;
        read += 4;
        write += written;
        if written < 3 && read != input.len() {
            return Err(DecodeError::InvalidPadding { index: read - 4 });
        }
    }
    Ok(write)
}

fn validate_decode<A: Alphabet, const PAD: bool>(input: &[u8]) -> Result<usize, DecodeError> {
    if input.is_empty() {
        return Ok(0);
    }

    if PAD {
        validate_padded::<A>(input)
    } else {
        validate_unpadded::<A>(input)
    }
}

fn validate_padded<A: Alphabet>(input: &[u8]) -> Result<usize, DecodeError> {
    if !input.len().is_multiple_of(4) {
        return Err(DecodeError::InvalidLength);
    }
    let required = decoded_len_padded(input)?;

    let mut read = 0;
    while read < input.len() {
        let chunk = read_quad(input, read)?;
        let written =
            validate_chunk::<A, true>(chunk).map_err(|err| err.with_index_offset(read))?;
        read += 4;
        if written < 3 && read != input.len() {
            return Err(DecodeError::InvalidPadding { index: read - 4 });
        }
    }

    Ok(required)
}

fn validate_unpadded<A: Alphabet>(input: &[u8]) -> Result<usize, DecodeError> {
    let required = decoded_len_unpadded(input)?;

    let mut read = 0;
    while read + 4 <= input.len() {
        let chunk = read_quad(input, read)?;
        validate_chunk::<A, false>(chunk).map_err(|err| err.with_index_offset(read))?;
        read += 4;
    }
    validate_tail_unpadded::<A>(&input[read..]).map_err(|err| err.with_index_offset(read))?;

    Ok(required)
}

fn decode_unpadded<A: Alphabet>(input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
    let required = decoded_len_unpadded(input)?;
    if output.len() < required {
        return Err(DecodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let mut read = 0;
    let mut write = 0;
    while read + 4 <= input.len() {
        let chunk = read_quad(input, read)?;
        let available = output.len();
        let output_tail = output.get_mut(write..).ok_or(DecodeError::OutputTooSmall {
            required: write,
            available,
        })?;
        let written = decode_chunk::<A, false>(chunk, output_tail)
            .map_err(|err| err.with_index_offset(read))?;
        read += 4;
        write += written;
    }
    decode_tail_unpadded::<A>(&input[read..], &mut output[write..])
        .map_err(|err| err.with_index_offset(read))
        .map(|n| write + n)
}

fn decoded_len_padded(input: &[u8]) -> Result<usize, DecodeError> {
    if input.is_empty() {
        return Ok(0);
    }
    if !input.len().is_multiple_of(4) {
        return Err(DecodeError::InvalidLength);
    }
    let mut padding = 0;
    if input[input.len() - 1] == b'=' {
        padding += 1;
    }
    if input[input.len() - 2] == b'=' {
        padding += 1;
    }
    if padding == 0
        && let Some(index) = input.iter().position(|byte| *byte == b'=')
    {
        return Err(DecodeError::InvalidPadding { index });
    }
    if padding > 0 {
        let first_pad = input.len() - padding;
        if let Some(index) = input[..first_pad].iter().position(|byte| *byte == b'=') {
            return Err(DecodeError::InvalidPadding { index });
        }
    }
    Ok(input.len() / 4 * 3 - padding)
}

fn decoded_len_unpadded(input: &[u8]) -> Result<usize, DecodeError> {
    if input.len() % 4 == 1 {
        return Err(DecodeError::InvalidLength);
    }
    if let Some(index) = input.iter().position(|byte| *byte == b'=') {
        return Err(DecodeError::InvalidPadding { index });
    }
    Ok(decoded_capacity(input.len()))
}

fn read_quad(input: &[u8], offset: usize) -> Result<[u8; 4], DecodeError> {
    let end = offset.checked_add(4).ok_or(DecodeError::InvalidLength)?;
    match input.get(offset..end) {
        Some([b0, b1, b2, b3]) => Ok([*b0, *b1, *b2, *b3]),
        _ => Err(DecodeError::InvalidLength),
    }
}

fn first_padding_index(input: [u8; 4]) -> usize {
    let [b0, b1, b2, b3] = input;
    if b0 == b'=' {
        0
    } else if b1 == b'=' {
        1
    } else if b2 == b'=' {
        2
    } else if b3 == b'=' {
        3
    } else {
        0
    }
}

fn validate_chunk<A: Alphabet, const PAD: bool>(input: [u8; 4]) -> Result<usize, DecodeError> {
    let [b0, b1, b2, b3] = input;
    let _v0 = decode_byte::<A>(b0, 0)?;
    let v1 = decode_byte::<A>(b1, 1)?;

    match (b2, b3) {
        (b'=', b'=') if PAD => {
            if v1 & 0b0000_1111 != 0 {
                return Err(DecodeError::InvalidPadding { index: 1 });
            }
            Ok(1)
        }
        (b'=', _) if PAD => Err(DecodeError::InvalidPadding { index: 2 }),
        (_, b'=') if PAD => {
            let v2 = decode_byte::<A>(b2, 2)?;
            if v2 & 0b0000_0011 != 0 {
                return Err(DecodeError::InvalidPadding { index: 2 });
            }
            Ok(2)
        }
        (b'=', _) | (_, b'=') => Err(DecodeError::InvalidPadding {
            index: first_padding_index(input),
        }),
        _ => {
            decode_byte::<A>(b2, 2)?;
            decode_byte::<A>(b3, 3)?;
            Ok(3)
        }
    }
}

fn decode_chunk<A: Alphabet, const PAD: bool>(
    input: [u8; 4],
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    let [b0, b1, b2, b3] = input;
    let v0 = decode_byte::<A>(b0, 0)?;
    let v1 = decode_byte::<A>(b1, 1)?;

    match (b2, b3) {
        (b'=', b'=') if PAD => {
            if output.is_empty() {
                return Err(DecodeError::OutputTooSmall {
                    required: 1,
                    available: output.len(),
                });
            }
            if v1 & 0b0000_1111 != 0 {
                return Err(DecodeError::InvalidPadding { index: 1 });
            }
            output[0] = (v0 << 2) | (v1 >> 4);
            Ok(1)
        }
        (b'=', _) if PAD => Err(DecodeError::InvalidPadding { index: 2 }),
        (_, b'=') if PAD => {
            if output.len() < 2 {
                return Err(DecodeError::OutputTooSmall {
                    required: 2,
                    available: output.len(),
                });
            }
            let v2 = decode_byte::<A>(b2, 2)?;
            if v2 & 0b0000_0011 != 0 {
                return Err(DecodeError::InvalidPadding { index: 2 });
            }
            output[0] = (v0 << 2) | (v1 >> 4);
            output[1] = (v1 << 4) | (v2 >> 2);
            Ok(2)
        }
        (b'=', _) | (_, b'=') => Err(DecodeError::InvalidPadding {
            index: first_padding_index(input),
        }),
        _ => {
            if output.len() < 3 {
                return Err(DecodeError::OutputTooSmall {
                    required: 3,
                    available: output.len(),
                });
            }
            let v2 = decode_byte::<A>(b2, 2)?;
            let v3 = decode_byte::<A>(b3, 3)?;
            output[0] = (v0 << 2) | (v1 >> 4);
            output[1] = (v1 << 4) | (v2 >> 2);
            output[2] = (v2 << 6) | v3;
            Ok(3)
        }
    }
}

fn validate_tail_unpadded<A: Alphabet>(input: &[u8]) -> Result<(), DecodeError> {
    match input.len() {
        0 => Ok(()),
        2 => {
            decode_byte::<A>(input[0], 0)?;
            let v1 = decode_byte::<A>(input[1], 1)?;
            if v1 & 0b0000_1111 != 0 {
                return Err(DecodeError::InvalidPadding { index: 1 });
            }
            Ok(())
        }
        3 => {
            decode_byte::<A>(input[0], 0)?;
            decode_byte::<A>(input[1], 1)?;
            let v2 = decode_byte::<A>(input[2], 2)?;
            if v2 & 0b0000_0011 != 0 {
                return Err(DecodeError::InvalidPadding { index: 2 });
            }
            Ok(())
        }
        _ => Err(DecodeError::InvalidLength),
    }
}

fn decode_tail_unpadded<A: Alphabet>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    match input.len() {
        0 => Ok(0),
        2 => {
            if output.is_empty() {
                return Err(DecodeError::OutputTooSmall {
                    required: 1,
                    available: output.len(),
                });
            }
            let v0 = decode_byte::<A>(input[0], 0)?;
            let v1 = decode_byte::<A>(input[1], 1)?;
            if v1 & 0b0000_1111 != 0 {
                return Err(DecodeError::InvalidPadding { index: 1 });
            }
            output[0] = (v0 << 2) | (v1 >> 4);
            Ok(1)
        }
        3 => {
            if output.len() < 2 {
                return Err(DecodeError::OutputTooSmall {
                    required: 2,
                    available: output.len(),
                });
            }
            let v0 = decode_byte::<A>(input[0], 0)?;
            let v1 = decode_byte::<A>(input[1], 1)?;
            let v2 = decode_byte::<A>(input[2], 2)?;
            if v2 & 0b0000_0011 != 0 {
                return Err(DecodeError::InvalidPadding { index: 2 });
            }
            output[0] = (v0 << 2) | (v1 >> 4);
            output[1] = (v1 << 4) | (v2 >> 2);
            Ok(2)
        }
        _ => Err(DecodeError::InvalidLength),
    }
}

fn decode_byte<A: Alphabet>(byte: u8, index: usize) -> Result<u8, DecodeError> {
    A::decode(byte).ok_or(DecodeError::InvalidByte { index, byte })
}

fn ct_decode_slice<A: Alphabet, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    if input.is_empty() {
        return Ok(0);
    }

    if PAD {
        ct_decode_padded::<A>(input, output)
    } else {
        ct_decode_unpadded::<A>(input, output)
    }
}

fn ct_decode_in_place<A: Alphabet, const PAD: bool>(
    buffer: &mut [u8],
) -> Result<usize, DecodeError> {
    if buffer.is_empty() {
        return Ok(0);
    }

    if PAD {
        ct_decode_padded_in_place::<A>(buffer)
    } else {
        ct_decode_unpadded_in_place::<A>(buffer)
    }
}

fn ct_validate_decode<A: Alphabet, const PAD: bool>(input: &[u8]) -> Result<(), DecodeError> {
    if input.is_empty() {
        return Ok(());
    }

    if PAD {
        ct_validate_padded::<A>(input)
    } else {
        ct_validate_unpadded::<A>(input)
    }
}

fn ct_decoded_len<A: Alphabet, const PAD: bool>(input: &[u8]) -> Result<usize, DecodeError> {
    ct_validate_decode::<A, PAD>(input)?;
    if input.is_empty() {
        return Ok(0);
    }

    if PAD {
        Ok(input.len() / 4 * 3 - ct_padding_len(input))
    } else {
        let full_quads = input.len() / 4 * 3;
        match input.len() % 4 {
            0 => Ok(full_quads),
            2 => Ok(full_quads + 1),
            3 => Ok(full_quads + 2),
            _ => Err(DecodeError::InvalidLength),
        }
    }
}

fn ct_validate_padded<A: Alphabet>(input: &[u8]) -> Result<(), DecodeError> {
    if !input.len().is_multiple_of(4) {
        return Err(DecodeError::InvalidLength);
    }

    let padding = ct_padding_len(input);
    let mut invalid_byte = 0u8;
    let mut invalid_padding = 0u8;
    let mut read = 0;

    while read + 4 < input.len() {
        let [b0, b1, b2, b3] = read_quad(input, read)?;
        let (_, valid0) = ct_decode_alphabet_byte::<A>(b0);
        let (_, valid1) = ct_decode_alphabet_byte::<A>(b1);
        let (_, valid2) = ct_decode_alphabet_byte::<A>(b2);
        let (_, valid3) = ct_decode_alphabet_byte::<A>(b3);

        invalid_byte |= !valid0;
        invalid_byte |= !valid1;
        invalid_byte |= !valid2;
        invalid_byte |= !valid3;
        invalid_padding |= ct_mask_eq_u8(b2, b'=');
        invalid_padding |= ct_mask_eq_u8(b3, b'=');
        read += 4;
    }

    let final_chunk = read_quad(input, read)?;
    let (_, final_invalid_byte, final_invalid_padding, _) =
        ct_padded_final_quantum::<A>(final_chunk, padding);
    invalid_byte |= final_invalid_byte;
    invalid_padding |= final_invalid_padding;

    report_ct_error(invalid_byte, invalid_padding)
}

fn ct_validate_unpadded<A: Alphabet>(input: &[u8]) -> Result<(), DecodeError> {
    if input.len() % 4 == 1 {
        return Err(DecodeError::InvalidLength);
    }

    let mut invalid_byte = 0u8;
    let mut invalid_padding = 0u8;
    let mut read = 0;

    while read + 4 <= input.len() {
        let b0 = input[read];
        let b1 = input[read + 1];
        let b2 = input[read + 2];
        let b3 = input[read + 3];
        let (_, valid0) = ct_decode_alphabet_byte::<A>(b0);
        let (_, valid1) = ct_decode_alphabet_byte::<A>(b1);
        let (_, valid2) = ct_decode_alphabet_byte::<A>(b2);
        let (_, valid3) = ct_decode_alphabet_byte::<A>(b3);

        invalid_byte |= !valid0;
        invalid_byte |= !valid1;
        invalid_byte |= !valid2;
        invalid_byte |= !valid3;
        invalid_padding |= ct_mask_eq_u8(b0, b'=');
        invalid_padding |= ct_mask_eq_u8(b1, b'=');
        invalid_padding |= ct_mask_eq_u8(b2, b'=');
        invalid_padding |= ct_mask_eq_u8(b3, b'=');

        read += 4;
    }

    match input.len() - read {
        0 => {}
        2 => {
            let b0 = input[read];
            let b1 = input[read + 1];
            let (_, valid0) = ct_decode_alphabet_byte::<A>(b0);
            let (v1, valid1) = ct_decode_alphabet_byte::<A>(b1);
            invalid_byte |= !valid0;
            invalid_byte |= !valid1;
            invalid_padding |= ct_mask_eq_u8(b0, b'=');
            invalid_padding |= ct_mask_eq_u8(b1, b'=');
            invalid_padding |= ct_mask_nonzero_u8(v1 & 0b0000_1111);
        }
        3 => {
            let b0 = input[read];
            let b1 = input[read + 1];
            let b2 = input[read + 2];
            let (_, valid0) = ct_decode_alphabet_byte::<A>(b0);
            let (_, valid1) = ct_decode_alphabet_byte::<A>(b1);
            let (v2, valid2) = ct_decode_alphabet_byte::<A>(b2);
            invalid_byte |= !valid0;
            invalid_byte |= !valid1;
            invalid_byte |= !valid2;
            invalid_padding |= ct_mask_eq_u8(b0, b'=');
            invalid_padding |= ct_mask_eq_u8(b1, b'=');
            invalid_padding |= ct_mask_eq_u8(b2, b'=');
            invalid_padding |= ct_mask_nonzero_u8(v2 & 0b0000_0011);
        }
        _ => return Err(DecodeError::InvalidLength),
    }

    report_ct_error(invalid_byte, invalid_padding)
}

fn ct_padded_final_quantum<A: Alphabet>(
    input: [u8; 4],
    padding: usize,
) -> ([u8; 3], u8, u8, usize) {
    let [b0, b1, b2, b3] = input;
    let (v0, valid0) = ct_decode_alphabet_byte::<A>(b0);
    let (v1, valid1) = ct_decode_alphabet_byte::<A>(b1);
    let (v2, valid2) = ct_decode_alphabet_byte::<A>(b2);
    let (v3, valid3) = ct_decode_alphabet_byte::<A>(b3);

    let padding_byte = padding.to_le_bytes()[0];
    let no_padding = ct_mask_eq_u8(padding_byte, 0);
    let one_padding = ct_mask_eq_u8(padding_byte, 1);
    let two_padding = ct_mask_eq_u8(padding_byte, 2);
    let require_v2 = no_padding | one_padding;
    let require_v3 = no_padding;

    let invalid_byte = !valid0 | !valid1 | (!valid2 & require_v2) | (!valid3 & require_v3);
    let invalid_padding = (ct_mask_nonzero_u8(v1 & 0b0000_1111) & two_padding)
        | ((ct_mask_eq_u8(b2, b'=') | ct_mask_nonzero_u8(v2 & 0b0000_0011)) & one_padding)
        | ((ct_mask_eq_u8(b2, b'=') | ct_mask_eq_u8(b3, b'=')) & no_padding);

    (
        [(v0 << 2) | (v1 >> 4), (v1 << 4) | (v2 >> 2), (v2 << 6) | v3],
        invalid_byte,
        invalid_padding,
        3 - padding,
    )
}

fn ct_decode_padded<A: Alphabet>(input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
    if !input.len().is_multiple_of(4) {
        return Err(DecodeError::InvalidLength);
    }

    let padding = ct_padding_len(input);
    let required = input.len() / 4 * 3 - padding;
    if output.len() < required {
        return Err(DecodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let mut invalid_byte = 0u8;
    let mut invalid_padding = 0u8;
    let mut write = 0;
    let mut read = 0;

    while read + 4 < input.len() {
        let [b0, b1, b2, b3] = read_quad(input, read)?;
        let (v0, valid0) = ct_decode_alphabet_byte::<A>(b0);
        let (v1, valid1) = ct_decode_alphabet_byte::<A>(b1);
        let (v2, valid2) = ct_decode_alphabet_byte::<A>(b2);
        let (v3, valid3) = ct_decode_alphabet_byte::<A>(b3);

        invalid_byte |= !valid0;
        invalid_byte |= !valid1;
        invalid_byte |= !valid2;
        invalid_byte |= !valid3;
        invalid_padding |= ct_mask_eq_u8(b2, b'=');
        invalid_padding |= ct_mask_eq_u8(b3, b'=');
        output[write] = (v0 << 2) | (v1 >> 4);
        output[write + 1] = (v1 << 4) | (v2 >> 2);
        output[write + 2] = (v2 << 6) | v3;
        write += 3;
        read += 4;
    }

    let final_chunk = read_quad(input, read)?;
    let (final_bytes, final_invalid_byte, final_invalid_padding, final_written) =
        ct_padded_final_quantum::<A>(final_chunk, padding);
    invalid_byte |= final_invalid_byte;
    invalid_padding |= final_invalid_padding;
    output[write..write + final_written].copy_from_slice(&final_bytes[..final_written]);
    write += final_written;

    report_ct_error(invalid_byte, invalid_padding)?;
    Ok(write)
}

fn ct_decode_padded_in_place<A: Alphabet>(buffer: &mut [u8]) -> Result<usize, DecodeError> {
    if !buffer.len().is_multiple_of(4) {
        return Err(DecodeError::InvalidLength);
    }

    let padding = ct_padding_len(buffer);
    let required = buffer.len() / 4 * 3 - padding;
    debug_assert!(required <= buffer.len());

    let mut invalid_byte = 0u8;
    let mut invalid_padding = 0u8;
    let mut write = 0;
    let mut read = 0;

    while read + 4 < buffer.len() {
        let [b0, b1, b2, b3] = read_quad(buffer, read)?;
        let (v0, valid0) = ct_decode_alphabet_byte::<A>(b0);
        let (v1, valid1) = ct_decode_alphabet_byte::<A>(b1);
        let (v2, valid2) = ct_decode_alphabet_byte::<A>(b2);
        let (v3, valid3) = ct_decode_alphabet_byte::<A>(b3);

        invalid_byte |= !valid0;
        invalid_byte |= !valid1;
        invalid_byte |= !valid2;
        invalid_byte |= !valid3;
        invalid_padding |= ct_mask_eq_u8(b2, b'=');
        invalid_padding |= ct_mask_eq_u8(b3, b'=');
        buffer[write] = (v0 << 2) | (v1 >> 4);
        buffer[write + 1] = (v1 << 4) | (v2 >> 2);
        buffer[write + 2] = (v2 << 6) | v3;
        write += 3;
        read += 4;
    }

    let final_chunk = read_quad(buffer, read)?;
    let (final_bytes, final_invalid_byte, final_invalid_padding, final_written) =
        ct_padded_final_quantum::<A>(final_chunk, padding);
    invalid_byte |= final_invalid_byte;
    invalid_padding |= final_invalid_padding;
    buffer[write..write + final_written].copy_from_slice(&final_bytes[..final_written]);
    write += final_written;

    debug_assert_eq!(write, required);
    report_ct_error(invalid_byte, invalid_padding)?;
    Ok(write)
}

fn ct_decode_unpadded<A: Alphabet>(input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
    if input.len() % 4 == 1 {
        return Err(DecodeError::InvalidLength);
    }

    let required = decoded_capacity(input.len());
    if output.len() < required {
        return Err(DecodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let mut invalid_byte = 0u8;
    let mut invalid_padding = 0u8;
    let mut write = 0;
    let mut read = 0;

    while read + 4 <= input.len() {
        let b0 = input[read];
        let b1 = input[read + 1];
        let b2 = input[read + 2];
        let b3 = input[read + 3];
        let (v0, valid0) = ct_decode_alphabet_byte::<A>(b0);
        let (v1, valid1) = ct_decode_alphabet_byte::<A>(b1);
        let (v2, valid2) = ct_decode_alphabet_byte::<A>(b2);
        let (v3, valid3) = ct_decode_alphabet_byte::<A>(b3);

        invalid_byte |= !valid0;
        invalid_byte |= !valid1;
        invalid_byte |= !valid2;
        invalid_byte |= !valid3;
        invalid_padding |= ct_mask_eq_u8(b0, b'=');
        invalid_padding |= ct_mask_eq_u8(b1, b'=');
        invalid_padding |= ct_mask_eq_u8(b2, b'=');
        invalid_padding |= ct_mask_eq_u8(b3, b'=');

        output[write] = (v0 << 2) | (v1 >> 4);
        output[write + 1] = (v1 << 4) | (v2 >> 2);
        output[write + 2] = (v2 << 6) | v3;
        read += 4;
        write += 3;
    }

    match input.len() - read {
        0 => {}
        2 => {
            let b0 = input[read];
            let b1 = input[read + 1];
            let (v0, valid0) = ct_decode_alphabet_byte::<A>(b0);
            let (v1, valid1) = ct_decode_alphabet_byte::<A>(b1);
            invalid_byte |= !valid0;
            invalid_byte |= !valid1;
            invalid_padding |= ct_mask_eq_u8(b0, b'=');
            invalid_padding |= ct_mask_eq_u8(b1, b'=');
            invalid_padding |= ct_mask_nonzero_u8(v1 & 0b0000_1111);
            output[write] = (v0 << 2) | (v1 >> 4);
            write += 1;
        }
        3 => {
            let b0 = input[read];
            let b1 = input[read + 1];
            let b2 = input[read + 2];
            let (v0, valid0) = ct_decode_alphabet_byte::<A>(b0);
            let (v1, valid1) = ct_decode_alphabet_byte::<A>(b1);
            let (v2, valid2) = ct_decode_alphabet_byte::<A>(b2);
            invalid_byte |= !valid0;
            invalid_byte |= !valid1;
            invalid_byte |= !valid2;
            invalid_padding |= ct_mask_eq_u8(b0, b'=');
            invalid_padding |= ct_mask_eq_u8(b1, b'=');
            invalid_padding |= ct_mask_eq_u8(b2, b'=');
            invalid_padding |= ct_mask_nonzero_u8(v2 & 0b0000_0011);
            output[write] = (v0 << 2) | (v1 >> 4);
            output[write + 1] = (v1 << 4) | (v2 >> 2);
            write += 2;
        }
        _ => return Err(DecodeError::InvalidLength),
    }

    report_ct_error(invalid_byte, invalid_padding)?;
    Ok(write)
}

fn ct_decode_unpadded_in_place<A: Alphabet>(buffer: &mut [u8]) -> Result<usize, DecodeError> {
    if buffer.len() % 4 == 1 {
        return Err(DecodeError::InvalidLength);
    }

    let required = decoded_capacity(buffer.len());
    debug_assert!(required <= buffer.len());

    let mut invalid_byte = 0u8;
    let mut invalid_padding = 0u8;
    let mut write = 0;
    let mut read = 0;

    while read + 4 <= buffer.len() {
        let b0 = buffer[read];
        let b1 = buffer[read + 1];
        let b2 = buffer[read + 2];
        let b3 = buffer[read + 3];
        let (v0, valid0) = ct_decode_alphabet_byte::<A>(b0);
        let (v1, valid1) = ct_decode_alphabet_byte::<A>(b1);
        let (v2, valid2) = ct_decode_alphabet_byte::<A>(b2);
        let (v3, valid3) = ct_decode_alphabet_byte::<A>(b3);

        invalid_byte |= !valid0;
        invalid_byte |= !valid1;
        invalid_byte |= !valid2;
        invalid_byte |= !valid3;
        invalid_padding |= ct_mask_eq_u8(b0, b'=');
        invalid_padding |= ct_mask_eq_u8(b1, b'=');
        invalid_padding |= ct_mask_eq_u8(b2, b'=');
        invalid_padding |= ct_mask_eq_u8(b3, b'=');

        buffer[write] = (v0 << 2) | (v1 >> 4);
        buffer[write + 1] = (v1 << 4) | (v2 >> 2);
        buffer[write + 2] = (v2 << 6) | v3;
        read += 4;
        write += 3;
    }

    match buffer.len() - read {
        0 => {}
        2 => {
            let b0 = buffer[read];
            let b1 = buffer[read + 1];
            let (v0, valid0) = ct_decode_alphabet_byte::<A>(b0);
            let (v1, valid1) = ct_decode_alphabet_byte::<A>(b1);
            invalid_byte |= !valid0;
            invalid_byte |= !valid1;
            invalid_padding |= ct_mask_eq_u8(b0, b'=');
            invalid_padding |= ct_mask_eq_u8(b1, b'=');
            invalid_padding |= ct_mask_nonzero_u8(v1 & 0b0000_1111);
            buffer[write] = (v0 << 2) | (v1 >> 4);
            write += 1;
        }
        3 => {
            let b0 = buffer[read];
            let b1 = buffer[read + 1];
            let b2 = buffer[read + 2];
            let (v0, valid0) = ct_decode_alphabet_byte::<A>(b0);
            let (v1, valid1) = ct_decode_alphabet_byte::<A>(b1);
            let (v2, valid2) = ct_decode_alphabet_byte::<A>(b2);
            invalid_byte |= !valid0;
            invalid_byte |= !valid1;
            invalid_byte |= !valid2;
            invalid_padding |= ct_mask_eq_u8(b0, b'=');
            invalid_padding |= ct_mask_eq_u8(b1, b'=');
            invalid_padding |= ct_mask_eq_u8(b2, b'=');
            invalid_padding |= ct_mask_nonzero_u8(v2 & 0b0000_0011);
            buffer[write] = (v0 << 2) | (v1 >> 4);
            buffer[write + 1] = (v1 << 4) | (v2 >> 2);
            write += 2;
        }
        _ => return Err(DecodeError::InvalidLength),
    }

    debug_assert_eq!(write, required);
    report_ct_error(invalid_byte, invalid_padding)?;
    Ok(write)
}

#[inline]
fn ct_decode_alphabet_byte<A: Alphabet>(byte: u8) -> (u8, u8) {
    let mut decoded = 0u8;
    let mut valid = 0u8;
    let mut candidate = 0u8;

    while candidate < 64 {
        let matches = ct_mask_eq_u8(byte, A::ENCODE[candidate as usize]);
        decoded |= candidate & matches;
        valid |= matches;
        candidate += 1;
    }

    (decoded, valid)
}

fn ct_padding_len(input: &[u8]) -> usize {
    let last = input[input.len() - 1];
    let before_last = input[input.len() - 2];
    usize::from(ct_mask_eq_u8(last, b'=') & 1) + usize::from(ct_mask_eq_u8(before_last, b'=') & 1)
}

fn report_ct_error(invalid_byte: u8, invalid_padding: u8) -> Result<(), DecodeError> {
    if (invalid_byte | invalid_padding) != 0 {
        Err(DecodeError::InvalidInput)
    } else {
        Ok(())
    }
}

#[cfg(kani)]
mod kani_proofs {
    use super::{STANDARD, checked_encoded_len, ct, decoded_capacity};

    #[kani::proof]
    fn checked_encoded_len_is_bounded_for_small_inputs() {
        let len = usize::from(kani::any::<u8>());
        let padded = kani::any::<bool>();
        let encoded = checked_encoded_len(len, padded).expect("u8 input length cannot overflow");

        assert!(encoded >= len);
        assert!(encoded <= len / 3 * 4 + 4);
    }

    #[kani::proof]
    fn decoded_capacity_is_bounded_for_small_inputs() {
        let len = usize::from(kani::any::<u8>());
        let capacity = decoded_capacity(len);

        assert!(capacity <= len / 4 * 3 + 2);
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn standard_in_place_decode_returns_prefix_within_buffer() {
        let mut buffer = kani::any::<[u8; 8]>();
        let result = STANDARD.decode_in_place(&mut buffer);

        if let Ok(decoded) = result {
            assert!(decoded.len() <= 8);
        }
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn standard_decode_slice_returns_written_within_output() {
        let input = kani::any::<[u8; 4]>();
        let mut output = kani::any::<[u8; 3]>();
        let result = STANDARD.decode_slice(&input, &mut output);

        if let Ok(written) = result {
            assert!(written <= output.len());
        }
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn standard_decode_slice_clear_tail_clears_output_on_error() {
        let input = kani::any::<[u8; 4]>();
        let mut output = kani::any::<[u8; 3]>();
        let result = STANDARD.decode_slice_clear_tail(&input, &mut output);

        if result.is_err() {
            assert!(output.iter().all(|byte| *byte == 0));
        }
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn standard_encode_slice_returns_written_within_output() {
        let input = kani::any::<[u8; 3]>();
        let mut output = kani::any::<[u8; 4]>();
        let result = STANDARD.encode_slice(&input, &mut output);

        if let Ok(written) = result {
            assert!(written <= output.len());
        }
    }

    #[kani::proof]
    #[kani::unwind(4)]
    fn standard_encode_in_place_returns_prefix_within_buffer() {
        let mut buffer = kani::any::<[u8; 8]>();
        let input_len = usize::from(kani::any::<u8>() % 9);
        let result = STANDARD.encode_in_place(&mut buffer, input_len);

        if let Ok(encoded) = result {
            assert!(encoded.len() <= 8);
        }
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn standard_clear_tail_decode_clears_buffer_on_error() {
        let mut buffer = kani::any::<[u8; 4]>();
        let result = STANDARD.decode_in_place_clear_tail(&mut buffer);

        if result.is_err() {
            assert!(buffer.iter().all(|byte| *byte == 0));
        }
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn ct_standard_decode_slice_returns_written_within_output() {
        let input = kani::any::<[u8; 4]>();
        let mut output = kani::any::<[u8; 3]>();
        let result = ct::STANDARD.decode_slice(&input, &mut output);

        if let Ok(written) = result {
            assert!(written <= output.len());
        }
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn ct_standard_decode_slice_clear_tail_clears_output_on_error() {
        let input = kani::any::<[u8; 4]>();
        let mut output = kani::any::<[u8; 3]>();
        let result = ct::STANDARD.decode_slice_clear_tail(&input, &mut output);

        if result.is_err() {
            assert!(output.iter().all(|byte| *byte == 0));
        }
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn ct_standard_decode_in_place_clear_tail_clears_buffer_on_error() {
        let mut buffer = kani::any::<[u8; 4]>();
        let result = ct::STANDARD.decode_in_place_clear_tail(&mut buffer);

        if result.is_err() {
            assert!(buffer.iter().all(|byte| *byte == 0));
        }
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn ct_standard_validate_matches_decode_for_one_quantum() {
        let input = kani::any::<[u8; 4]>();
        let mut output = kani::any::<[u8; 3]>();

        let validate_ok = ct::STANDARD.validate_result(&input).is_ok();
        let decode_ok = ct::STANDARD.decode_slice(&input, &mut output).is_ok();

        assert!(validate_ok == decode_ok);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fill_pattern(output: &mut [u8], seed: usize) {
        for (index, byte) in output.iter_mut().enumerate() {
            let value = (index * 73 + seed * 19) % 256;
            *byte = u8::try_from(value).unwrap();
        }
    }

    fn assert_encode_backend_matches_scalar<A, const PAD: bool>(input: &[u8])
    where
        A: Alphabet,
    {
        let engine = Engine::<A, PAD>::new();
        let mut dispatched = [0x55; 256];
        let mut scalar = [0xaa; 256];

        let dispatched_result = engine.encode_slice(input, &mut dispatched);
        let scalar_result = backend::scalar_reference_encode_slice::<A, PAD>(input, &mut scalar);

        assert_eq!(dispatched_result, scalar_result);
        if let Ok(written) = dispatched_result {
            assert_eq!(&dispatched[..written], &scalar[..written]);
        }

        let required = checked_encoded_len(input.len(), PAD).unwrap();
        if required > 0 {
            let mut dispatched_short = [0x55; 256];
            let mut scalar_short = [0xaa; 256];
            let available = required - 1;

            assert_eq!(
                engine.encode_slice(input, &mut dispatched_short[..available]),
                backend::scalar_reference_encode_slice::<A, PAD>(
                    input,
                    &mut scalar_short[..available],
                )
            );
        }
    }

    fn assert_decode_backend_matches_scalar<A, const PAD: bool>(input: &[u8])
    where
        A: Alphabet,
    {
        let engine = Engine::<A, PAD>::new();
        let mut dispatched = [0x55; 128];
        let mut scalar = [0xaa; 128];

        let dispatched_result = engine.decode_slice(input, &mut dispatched);
        let scalar_result = backend::scalar_reference_decode_slice::<A, PAD>(input, &mut scalar);

        assert_eq!(dispatched_result, scalar_result);
        if let Ok(written) = dispatched_result {
            assert_eq!(&dispatched[..written], &scalar[..written]);

            if written > 0 {
                let mut dispatched_short = [0x55; 128];
                let mut scalar_short = [0xaa; 128];
                let available = written - 1;

                assert_eq!(
                    engine.decode_slice(input, &mut dispatched_short[..available]),
                    backend::scalar_reference_decode_slice::<A, PAD>(
                        input,
                        &mut scalar_short[..available],
                    )
                );
            }
        }
    }

    fn assert_backend_round_trip_matches_scalar<A, const PAD: bool>(input: &[u8])
    where
        A: Alphabet,
    {
        assert_encode_backend_matches_scalar::<A, PAD>(input);

        let mut encoded = [0; 256];
        let encoded_len =
            backend::scalar_reference_encode_slice::<A, PAD>(input, &mut encoded).unwrap();
        assert_decode_backend_matches_scalar::<A, PAD>(&encoded[..encoded_len]);
    }

    #[test]
    fn backend_dispatch_matches_scalar_reference_for_canonical_inputs() {
        let mut input = [0; 128];

        for input_len in 0..=input.len() {
            fill_pattern(&mut input[..input_len], input_len);
            let input = &input[..input_len];

            assert_backend_round_trip_matches_scalar::<Standard, true>(input);
            assert_backend_round_trip_matches_scalar::<Standard, false>(input);
            assert_backend_round_trip_matches_scalar::<UrlSafe, true>(input);
            assert_backend_round_trip_matches_scalar::<UrlSafe, false>(input);
        }
    }

    #[test]
    fn backend_dispatch_matches_scalar_reference_for_malformed_inputs() {
        for input in [
            &b"Z"[..],
            b"====",
            b"AA=A",
            b"Zh==",
            b"Zm9=",
            b"Zm9v$g==",
            b"Zm9vZh==",
        ] {
            assert_decode_backend_matches_scalar::<Standard, true>(input);
        }

        for input in [&b"Z"[..], b"AA=A", b"Zh", b"Zm9", b"Zm9vYg$"] {
            assert_decode_backend_matches_scalar::<Standard, false>(input);
        }

        assert_decode_backend_matches_scalar::<UrlSafe, true>(b"AA+A");
        assert_decode_backend_matches_scalar::<UrlSafe, false>(b"AA/A");
        assert_decode_backend_matches_scalar::<Standard, true>(b"AA-A");
        assert_decode_backend_matches_scalar::<Standard, false>(b"AA_A");
    }

    #[cfg(feature = "simd")]
    #[test]
    fn simd_dispatch_scaffold_keeps_scalar_active() {
        assert_eq!(simd::active_backend(), simd::ActiveBackend::Scalar);
        let _candidate = simd::detected_candidate();
    }

    #[test]
    fn encodes_standard_vectors() {
        let vectors = [
            (&b""[..], &b""[..]),
            (&b"f"[..], &b"Zg=="[..]),
            (&b"fo"[..], &b"Zm8="[..]),
            (&b"foo"[..], &b"Zm9v"[..]),
            (&b"foob"[..], &b"Zm9vYg=="[..]),
            (&b"fooba"[..], &b"Zm9vYmE="[..]),
            (&b"foobar"[..], &b"Zm9vYmFy"[..]),
        ];
        for (input, expected) in vectors {
            let mut output = [0u8; 16];
            let written = STANDARD.encode_slice(input, &mut output).unwrap();
            assert_eq!(&output[..written], expected);
        }
    }

    #[test]
    fn decodes_standard_vectors() {
        let vectors = [
            (&b""[..], &b""[..]),
            (&b"Zg=="[..], &b"f"[..]),
            (&b"Zm8="[..], &b"fo"[..]),
            (&b"Zm9v"[..], &b"foo"[..]),
            (&b"Zm9vYg=="[..], &b"foob"[..]),
            (&b"Zm9vYmE="[..], &b"fooba"[..]),
            (&b"Zm9vYmFy"[..], &b"foobar"[..]),
        ];
        for (input, expected) in vectors {
            let mut output = [0u8; 16];
            let written = STANDARD.decode_slice(input, &mut output).unwrap();
            assert_eq!(&output[..written], expected);
        }
    }

    #[test]
    fn supports_unpadded_url_safe() {
        let mut encoded = [0u8; 16];
        let written = URL_SAFE_NO_PAD
            .encode_slice(b"\xfb\xff", &mut encoded)
            .unwrap();
        assert_eq!(&encoded[..written], b"-_8");

        let mut decoded = [0u8; 2];
        let written = URL_SAFE_NO_PAD
            .decode_slice(&encoded[..written], &mut decoded)
            .unwrap();
        assert_eq!(&decoded[..written], b"\xfb\xff");
    }

    #[test]
    fn decodes_in_place() {
        let mut buffer = *b"Zm9vYmFy";
        let decoded = STANDARD_NO_PAD.decode_in_place(&mut buffer).unwrap();
        assert_eq!(decoded, b"foobar");
    }

    #[test]
    fn rejects_non_canonical_padding_bits() {
        let mut output = [0u8; 4];
        assert_eq!(
            STANDARD.decode_slice(b"Zh==", &mut output),
            Err(DecodeError::InvalidPadding { index: 1 })
        );
        assert_eq!(
            STANDARD.decode_slice(b"Zm9=", &mut output),
            Err(DecodeError::InvalidPadding { index: 2 })
        );
    }
}
