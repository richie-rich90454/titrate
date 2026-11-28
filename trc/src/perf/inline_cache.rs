//! Inline primitive caches for the VM run loop.
//!
//! A site is a bytecode instruction offset. The cache remembers the primitive
//! shape last seen at that site and the unboxed value, so a monomorphic hot
//! instruction can avoid rebuilding a boxed `Value`. Slots are inline and
//! replaced round-robin; there is no map, hash, or allocation on either path.

use crate::bytecode::value::Value;

pub const INLINE_CACHE_SLOTS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimShape {
    Int,
    Float,
    Bool,
    Other,
}

impl PrimShape {
    pub fn of(value: &Value) -> Self {
        match value {
            Value::Bool(_) => PrimShape::Bool,
            Value::Byte(_) | Value::Short(_) | Value::Int(_) | Value::Long(_) | Value::Char(_) => {
                PrimShape::Int
            }
            Value::Float(_) | Value::Double(_) | Value::Half(_) | Value::Quad(_) => PrimShape::Float,
            _ => PrimShape::Other,
        }
    }
}

/// Small `Copy` payload that fits in a register pair.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Prim {
    Int(i64),
    Float(f64),
    Bool(bool),
}

impl Prim {
    pub fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Bool(v) => Some(Prim::Bool(*v)),
            Value::Byte(v) => Some(Prim::Int(*v as i64)),
            Value::Short(v) => Some(Prim::Int(*v as i64)),
            Value::Int(v) => Some(Prim::Int(*v as i64)),
            Value::Long(v) => Some(Prim::Int(*v)),
            Value::Char(v) => Some(Prim::Int(*v as i64)),
            Value::Float(v) => Some(Prim::Float(*v as f64)),
            Value::Double(v) => Some(Prim::Float(*v)),
            Value::Half(v) => Some(Prim::Float(*v as f64)),
            Value::Quad(v) => Some(Prim::Float(*v)),
            _ => None,
        }
    }

    pub fn into_value(self) -> Value {
        match self {
            Prim::Int(v) => Value::Long(v),
            Prim::Float(v) => Value::Double(v),
            Prim::Bool(v) => Value::Bool(v),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct CacheSlot {
    site: u32,
    shape: PrimShape,
    prim: Prim,
}

pub struct InlineCache {
    slots: [Option<CacheSlot>; INLINE_CACHE_SLOTS],
    next: usize,
    hits: u64,
    misses: u64,
}

impl Default for InlineCache {
    fn default() -> Self {
        Self::new()
    }
}

impl InlineCache {
    pub const fn new() -> Self {
        Self {
            slots: [None; INLINE_CACHE_SLOTS],
            next: 0,
            hits: 0,
            misses: 0,
        }
    }

    /// Non-mutating peek, used by a run loop that has already decoded a site.
    pub fn cached(&self, site: u32, shape: PrimShape) -> Option<Prim> {
        self.slots
            .iter()
            .flatten()
            .find(|slot| slot.site == site && slot.shape == shape)
            .map(|slot| slot.prim)
    }

    /// Record a value at a site. Returns a cached primitive only on a hit;
    /// the first observation and any shape change are misses and populate
    /// (or refresh) the slot.
    pub fn observe(&mut self, site: u32, value: &Value) -> Option<Prim> {
        let shape = PrimShape::of(value);
        if let Some(prim) = self.cached(site, shape) {
            self.hits += 1;
            return Some(prim);
        }
        self.misses += 1;
        if let Some(prim) = Prim::from_value(value) {
            let idx = self.next;
            self.next = (self.next + 1) % INLINE_CACHE_SLOTS;
            self.slots[idx] = Some(CacheSlot {
                site,
                shape,
                prim,
            });
        }
        None
    }

    pub fn hits(&self) -> u64 {
        self.hits
    }

    pub fn misses(&self) -> u64 {
        self.misses
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn primitives_are_small_and_copy() {
        assert!(size_of::<Prim>() <= 16);
        assert!(size_of::<CacheSlot>() <= 40);
    }

    #[test]
    fn primitives_extract_and_round_trip() {
        assert_eq!(Prim::from_value(&Value::Long(-7)), Some(Prim::Int(-7)));
        assert_eq!(Prim::from_value(&Value::Int(3)), Some(Prim::Int(3)));
        assert_eq!(Prim::from_value(&Value::Bool(true)), Some(Prim::Bool(true)));
        assert_eq!(
            Prim::from_value(&Value::Double(2.5)),
            Some(Prim::Float(2.5))
        );
        assert_eq!(Prim::from_value(&Value::Null), None);
        assert_eq!(Prim::Int(-7).into_value().to_i64(), Some(-7));
        assert!(matches!(
            Prim::Float(2.5).into_value(),
            Value::Double(v) if v == 2.5
        ));
        assert!(matches!(
            Prim::Bool(true).into_value(),
            Value::Bool(true)
        ));
    }

    #[test]
    fn second_observation_is_a_hit() {
        let mut cache = InlineCache::new();
        let value = Value::Long(42);
        assert_eq!(cache.observe(1, &value), None);
        assert_eq!(cache.observe(1, &value), Some(Prim::Int(42)));
        assert_eq!(cache.hits(), 1);
        assert_eq!(cache.misses(), 1);
        assert_eq!(cache.cached(1, PrimShape::Int), Some(Prim::Int(42)));
        assert_eq!(cache.cached(2, PrimShape::Int), None);
    }

    #[test]
    fn shape_change_is_a_miss_and_does_not_overwrite_the_old_shape() {
        let mut cache = InlineCache::new();
        cache.observe(1, &Value::Long(9));
        assert_eq!(cache.observe(1, &Value::Double(9.0)), None);
        assert_eq!(cache.cached(1, PrimShape::Int), Some(Prim::Int(9)));
        assert_eq!(cache.cached(1, PrimShape::Float), Some(Prim::Float(9.0)));
        assert_eq!(cache.misses(), 2);
    }

    #[test]
    fn slots_are_reused_round_robin() {
        let mut cache = InlineCache::new();
        for site in 0..INLINE_CACHE_SLOTS as u32 {
            assert_eq!(cache.observe(site, &Value::Long(site as i64)), None);
        }
        assert_eq!(cache.cached(0, PrimShape::Int), Some(Prim::Int(0)));
        for site in INLINE_CACHE_SLOTS as u32..INLINE_CACHE_SLOTS as u32 * 2 {
            cache.observe(site, &Value::Long(site as i64));
        }
        assert_eq!(cache.cached(0, PrimShape::Int), None);
        assert_eq!(cache.misses(), INLINE_CACHE_SLOTS as u64 * 2);
    }
}
