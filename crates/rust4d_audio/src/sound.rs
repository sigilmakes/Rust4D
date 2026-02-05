//! Sound handle types

/// Handle to a loaded sound asset
///
/// This is a lightweight reference that can be cheaply cloned.
/// The actual sound data is stored in the AudioEngine4D.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SoundHandle {
    id: u64,
}

impl SoundHandle {
    /// Create a new sound handle (internal use)
    pub(crate) fn new(id: u64) -> Self {
        Self { id }
    }

    /// Get the internal ID of this sound
    pub fn id(&self) -> u64 {
        self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_handle_equality() {
        let a = SoundHandle::new(1);
        let b = SoundHandle::new(1);
        let c = SoundHandle::new(2);

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_sound_handle_clone() {
        let a = SoundHandle::new(42);
        let b = a;
        assert_eq!(a.id(), b.id());
    }
}
