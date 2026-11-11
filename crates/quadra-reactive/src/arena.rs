//! A generational arena. A handle is an index and a generation, so one kept after its slot was
//! freed is detected instead of reaching whatever took the slot.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Id {
    index: u32,
    generation: u32,
}

impl Id {
    pub fn index(self) -> usize {
        self.index as usize
    }
}

struct Slot<T> {
    generation: u32,
    value: Option<T>,
}

pub(crate) struct Arena<T> {
    slots: Vec<Slot<T>>,
    free: Vec<u32>,
    len: usize,
}

impl<T> Arena<T> {
    pub fn new() -> Arena<T> {
        Arena {
            slots: Vec::new(),
            free: Vec::new(),
            len: 0,
        }
    }

    pub fn insert(&mut self, value: T) -> Id {
        self.len += 1;
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.value = Some(value);
            return Id {
                index,
                generation: slot.generation,
            };
        }
        self.slots.push(Slot {
            generation: 0,
            value: Some(value),
        });
        Id {
            index: (self.slots.len() - 1) as u32,
            generation: 0,
        }
    }

    pub fn remove(&mut self, id: Id) -> Option<T> {
        let slot = self.slots.get_mut(id.index())?;
        if slot.generation != id.generation {
            return None;
        }
        let value = slot.value.take()?;
        slot.generation = slot.generation.wrapping_add(1);
        self.free.push(id.index);
        self.len -= 1;
        Some(value)
    }

    pub fn get(&self, id: Id) -> Option<&T> {
        let slot = self.slots.get(id.index())?;
        if slot.generation == id.generation {
            slot.value.as_ref()
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, id: Id) -> Option<&mut T> {
        let slot = self.slots.get_mut(id.index())?;
        if slot.generation == id.generation {
            slot.value.as_mut()
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether `id` was the last occupant of its slot and nothing has taken the slot since.
    pub fn freed(&self, id: Id) -> bool {
        self.slots
            .get(id.index())
            .is_some_and(|s| s.value.is_none() && s.generation == id.generation.wrapping_add(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stale_handle_reaches_nothing() {
        let mut arena = Arena::new();
        let a = arena.insert("a");
        assert_eq!(arena.get(a), Some(&"a"));
        assert_eq!(arena.remove(a), Some("a"));
        assert!(arena.freed(a) && arena.get(a).is_none() && arena.remove(a).is_none());
        let b = arena.insert("b"); // takes the same slot
        assert_eq!(b.index(), a.index());
        assert!(arena.get(a).is_none() && !arena.freed(a));
        assert_eq!(arena.get_mut(b).map(|v| *v), Some("b"));
        assert_eq!(arena.len(), 1);
    }
}
