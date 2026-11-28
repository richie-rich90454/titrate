//! Advisory specialization hints for hot bytecode sites.
//!
//! Hints describe what a compiler or JIT tier may assume after observing a
//! site; they do not weaken runtime checks and are not consumed by the VM yet.

use std::ops::BitOr;

use super::inline_cache::PrimShape;

/// Observed calls before a site is considered hot.
pub const HOT_THRESHOLD: u64 = 64;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Hints(u32);

impl Hints {
    pub const NONE: Hints = Hints(0);
    pub const HOT: Hints = Hints(1 << 0);
    pub const MONOMORPHIC: Hints = Hints(1 << 1);
    pub const INTEGER_ONLY: Hints = Hints(1 << 2);
    pub const NO_UPVALUES: Hints = Hints(1 << 3);
    pub const NO_ALLOC: Hints = Hints(1 << 4);

    const ALL: u32 = Self::HOT.0
        | Self::MONOMORPHIC.0
        | Self::INTEGER_ONLY.0
        | Self::NO_UPVALUES.0
        | Self::NO_ALLOC.0;

    pub const fn from_bits(bits: u32) -> Self {
        Hints(bits & Self::ALL)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn insert(self, other: Hints) -> Hints {
        Hints(self.0 | other.0)
    }

    pub const fn remove(self, other: Hints) -> Hints {
        Hints(self.0 & !other.0)
    }

    pub const fn contains(self, other: Hints) -> bool {
        (self.0 & other.0) == other.0
    }

    pub const fn intersects(self, other: Hints) -> bool {
        (self.0 & other.0) != 0
    }

    /// Derive hints from an observed shape and call count. Unknown shapes get
    /// no specialization because their representation is not stable.
    pub fn for_shape(shape: PrimShape, calls: u64, has_upvalues: bool) -> Hints {
        if shape == PrimShape::Other {
            return Hints::NONE;
        }
        let mut hints = Hints::MONOMORPHIC | Hints::NO_ALLOC;
        if calls >= HOT_THRESHOLD {
            hints = hints.insert(Hints::HOT);
        }
        if matches!(shape, PrimShape::Int | PrimShape::Bool) {
            hints = hints.insert(Hints::INTEGER_ONLY);
        }
        if !has_upvalues {
            hints = hints.insert(Hints::NO_UPVALUES);
        }
        hints
    }
}

impl BitOr for Hints {
    type Output = Hints;

    fn bitor(self, rhs: Hints) -> Hints {
        self.insert(rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints_combine_and_remove() {
        let hints = Hints::HOT.insert(Hints::NO_ALLOC);
        assert!(hints.contains(Hints::HOT));
        assert!(hints.contains(Hints::NO_ALLOC));
        assert!(!hints.contains(Hints::MONOMORPHIC));
        assert!(hints.intersects(Hints::MONOMORPHIC | Hints::HOT));
        assert_eq!(hints.remove(Hints::HOT), Hints::NO_ALLOC);
        assert_eq!(Hints::NONE.bits(), 0);
    }

    #[test]
    fn unknown_bits_are_ignored() {
        assert_eq!(Hints::from_bits(u32::MAX).bits(), Hints::ALL);
    }

    #[test]
    fn hot_requires_the_threshold() {
        let cold = Hints::for_shape(PrimShape::Int, HOT_THRESHOLD - 1, false);
        let hot = Hints::for_shape(PrimShape::Int, HOT_THRESHOLD, false);
        assert!(!cold.contains(Hints::HOT));
        assert!(hot.contains(Hints::HOT));
    }

    #[test]
    fn shapes_select_the_expected_hints() {
        let int = Hints::for_shape(PrimShape::Int, HOT_THRESHOLD, true);
        assert!(int.contains(Hints::INTEGER_ONLY));
        assert!(!int.contains(Hints::NO_UPVALUES));
        let float = Hints::for_shape(PrimShape::Float, 0, false);
        assert!(float.contains(Hints::NO_UPVALUES));
        assert!(!float.contains(Hints::INTEGER_ONLY));
        assert_eq!(
            Hints::for_shape(PrimShape::Other, HOT_THRESHOLD * 2, false),
            Hints::NONE
        );
    }
}
