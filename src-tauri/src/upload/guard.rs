use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct InFlightGuard {
    inner: Arc<Mutex<bool>>,
}

pub struct InFlightToken {
    inner: Arc<Mutex<bool>>,
}

impl InFlightGuard {
    pub fn try_acquire(&self) -> Option<InFlightToken> {
        let mut lock = self.inner.lock().unwrap();
        if *lock {
            None
        } else {
            *lock = true;
            Some(InFlightToken { inner: self.inner.clone() })
        }
    }

    pub fn is_busy(&self) -> bool {
        *self.inner.lock().unwrap()
    }
}

impl Drop for InFlightToken {
    fn drop(&mut self) {
        let mut lock = self.inner.lock().unwrap();
        *lock = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_acquire_fails_while_first_held() {
        let g = InFlightGuard::default();
        let _a = g.try_acquire().expect("first should acquire");
        assert!(g.is_busy());
        assert!(g.try_acquire().is_none());
    }

    #[test]
    fn released_on_drop() {
        let g = InFlightGuard::default();
        {
            let _a = g.try_acquire().unwrap();
        }
        assert!(!g.is_busy());
        assert!(g.try_acquire().is_some());
    }
}
