use crate::{Alphabet, LineWrap, Profile, ct};

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

    /// Returns this engine as a profile with strict line wrapping.
    ///
    /// This is a convenience wrapper around [`Profile::new`] for protocol
    /// presets that need a wrapping policy to travel with the engine.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{LineEnding, LineWrap, STANDARD};
    ///
    /// let profile = STANDARD.profile_with_wrap(LineWrap::new(4, LineEnding::Lf));
    /// let mut output = [0u8; 9];
    /// let written = profile.encode_slice(b"hello", &mut output).unwrap();
    ///
    /// assert_eq!(&output[..written], b"aGVs\nbG8=");
    /// ```
    #[must_use]
    pub const fn profile_with_wrap(&self, wrap: LineWrap) -> Profile<A, PAD> {
        Profile::new(*self, Some(wrap))
    }

    /// Returns this engine as a wrapped profile, or `None` when the wrapping
    /// policy is invalid.
    ///
    /// Use this helper when a wrapping policy comes from runtime
    /// configuration or another untrusted source.
    #[must_use]
    pub const fn checked_profile_with_wrap(&self, wrap: LineWrap) -> Option<Profile<A, PAD>> {
        Profile::checked_new(*self, Some(wrap))
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
}

mod decode;
mod decode_const;
mod decode_in_place;
mod encode;
mod encode_in_place;
mod stream;
mod validate;
