use log::error;
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use windows::{Win32::Foundation::*, Win32::UI::WindowsAndMessaging::*};

pub trait CursorBackend {
    type Image;
    fn snapshot(&mut self, id: u32) -> io::Result<Self::Image>;
    fn apply(&mut self, image: &Self::Image, target: u32) -> io::Result<()>;
}

pub struct CursorManager<B: CursorBackend> {
    backend: B,
    mapping: BTreeMap<u32, u32>,
    originals: BTreeMap<u32, B::Image>,
    dirty: BTreeSet<u32>,
    applied: Option<bool>,
}

impl<B: CursorBackend> CursorManager<B> {
    pub fn new(mut backend: B, mapping: BTreeMap<u32, u32>) -> io::Result<Self> {
        let ids: BTreeSet<_> = mapping.keys().chain(mapping.values()).copied().collect();
        let mut originals = BTreeMap::new();
        // Capture every source before changing anything, including overlapping mappings.
        for id in ids {
            originals.insert(id, backend.snapshot(id)?);
        }
        Ok(Self {
            backend,
            mapping,
            originals,
            dirty: BTreeSet::new(),
            applied: Some(false),
        })
    }

    pub fn set_custom(&mut self, custom: bool) -> io::Result<()> {
        if self.applied == Some(custom) {
            return Ok(());
        }
        self.applied = None;
        for (&target, &source) in &self.mapping {
            self.dirty.insert(target);
            let image = &self.originals[&if custom { source } else { target }];
            if let Err(error) = self.backend.apply(image, target) {
                // Keep the operation retryable; never commit a partially applied state.
                if let Err(restore_error) = self.restore() {
                    error!("恢复光标失败：{}", restore_error);
                }
                return Err(error);
            }
        }
        if !custom {
            self.dirty.clear();
        }
        self.applied = Some(custom);
        Ok(())
    }

    pub fn restore(&mut self) -> io::Result<()> {
        let mut first_error = None;
        for target in self.dirty.clone() {
            match self.backend.apply(&self.originals[&target], target) {
                Ok(()) => {
                    self.dirty.remove(&target);
                }
                Err(error) => {
                    first_error.get_or_insert(error);
                }
            }
        }
        self.applied = if self.dirty.is_empty() {
            Some(false)
        } else {
            None
        };
        first_error.map_or(Ok(()), Err)
    }
}

impl<B: CursorBackend> Drop for CursorManager<B> {
    fn drop(&mut self) {
        if let Err(error) = self.restore() {
            error!("退出时恢复光标失败：{}", error);
        }
    }
}

pub struct OwnedCursor(HCURSOR);

impl OwnedCursor {
    fn copy(cursor: HCURSOR) -> io::Result<Self> {
        let copy = unsafe { CopyImage(HANDLE(cursor.0), IMAGE_CURSOR, 0, 0, IMAGE_FLAGS(0)) };
        if copy.0 == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(HCURSOR(copy.0)))
        }
    }

    fn into_raw(self) -> HCURSOR {
        let cursor = self.0;
        std::mem::forget(self);
        cursor
    }
}

impl Drop for OwnedCursor {
    fn drop(&mut self) {
        unsafe {
            DestroyCursor(self.0);
        }
    }
}

pub struct WindowsBackend;

impl CursorBackend for WindowsBackend {
    type Image = OwnedCursor;

    fn snapshot(&mut self, id: u32) -> io::Result<OwnedCursor> {
        let shared = unsafe { LoadCursorW(HINSTANCE::default(), PWSTR(id as usize as _)) };
        if shared.0 == 0 {
            return Err(io::Error::last_os_error());
        }
        OwnedCursor::copy(shared)
    }

    fn apply(&mut self, image: &OwnedCursor, target: u32) -> io::Result<()> {
        // SetSystemCursor consumes its input. Pass a fresh owned copy, never a shared
        // LoadCursorW handle or our saved original.
        let copy = OwnedCursor::copy(image.0)?.into_raw();
        if unsafe { SetSystemCursor(copy, SYSTEM_CURSOR_ID(target)) }.as_bool() {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    #[derive(Default)]
    struct State {
        images: BTreeMap<u32, u32>,
        writes: Vec<(u32, u32)>,
        fail_next_target: Option<u32>,
    }

    struct MockBackend(Rc<RefCell<State>>);

    impl CursorBackend for MockBackend {
        type Image = u32;
        fn snapshot(&mut self, id: u32) -> io::Result<u32> {
            Ok(self.0.borrow().images[&id])
        }
        fn apply(&mut self, image: &u32, target: u32) -> io::Result<()> {
            let mut state = self.0.borrow_mut();
            if state.fail_next_target == Some(target) {
                state.fail_next_target = None;
                return Err(io::Error::other("injected failure"));
            }
            state.writes.push((target, *image));
            state.images.insert(target, *image);
            Ok(())
        }
    }

    fn setup() -> (CursorManager<MockBackend>, Rc<RefCell<State>>) {
        let state = Rc::new(RefCell::new(State {
            images: BTreeMap::from([(1, 101), (2, 102), (3, 103)]),
            ..State::default()
        }));
        let manager =
            CursorManager::new(MockBackend(state.clone()), BTreeMap::from([(1, 2), (2, 3)]))
                .unwrap();
        (manager, state)
    }

    #[test]
    fn windows_cursor_snapshots_can_be_copied_and_released_without_applying() {
        let mut backend = WindowsBackend;
        for id in [32512, 32513, 32516, 32648, 32649, 32651] {
            let original = backend.snapshot(id).unwrap();
            let copy = OwnedCursor::copy(original.0).unwrap();
            assert_ne!(original.0, copy.0);
        }
    }

    #[test]
    fn repeated_switches_restore_originals_and_preserve_overlapping_sources() {
        let (mut manager, state) = setup();
        for _ in 0..3 {
            manager.set_custom(true).unwrap();
            assert_eq!(state.borrow().images[&1], 102);
            assert_eq!(state.borrow().images[&2], 103);
            let count = state.borrow().writes.len();
            manager.set_custom(true).unwrap();
            assert_eq!(state.borrow().writes.len(), count);
            manager.set_custom(false).unwrap();
            assert_eq!(state.borrow().images[&1], 101);
            assert_eq!(state.borrow().images[&2], 102);
        }
    }

    #[test]
    fn partial_failure_rolls_back_and_can_be_retried() {
        let (mut manager, state) = setup();
        state.borrow_mut().fail_next_target = Some(2);
        assert!(manager.set_custom(true).is_err());
        assert_eq!(state.borrow().images[&1], 101);
        assert_eq!(state.borrow().images[&2], 102);
        manager.set_custom(true).unwrap();
        assert_eq!(state.borrow().images[&1], 102);
        assert_eq!(state.borrow().images[&2], 103);
    }

    #[test]
    fn failed_restore_is_retried_instead_of_forgetting_the_dirty_cursor() {
        let (mut manager, state) = setup();
        manager.set_custom(true).unwrap();
        state.borrow_mut().fail_next_target = Some(1);
        assert!(manager.restore().is_err());
        assert_eq!(state.borrow().images[&2], 102);
        manager.restore().unwrap();
        assert_eq!(state.borrow().images[&1], 101);
    }

    #[test]
    fn dropping_active_manager_restores_without_touching_other_cursors() {
        let (mut manager, state) = setup();
        manager.set_custom(true).unwrap();
        drop(manager);
        assert_eq!(
            state.borrow().images,
            BTreeMap::from([(1, 101), (2, 102), (3, 103)])
        );
        assert!(state.borrow().writes.iter().all(|(target, _)| *target != 3));
    }
}
