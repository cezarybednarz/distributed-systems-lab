pub struct Fibonacci {
    v_iter_curr: Option<u128>,
    v_iter_prev: u128,
}

impl Fibonacci {
    /// Create new `Fibonacci`.
    pub fn new() -> Fibonacci {
        Fibonacci {
            v_iter_curr: Some(0),
            v_iter_prev: 1,
        }
    }

    /// Calculate the n-th Fibonacci number.
    ///
    /// This shall not change the state of the iterator.
    /// The calculations shall wrap around at the boundary of u8.
    /// The calculations might be slow (recursive calculations are acceptable).
    pub fn fibonacci(n: usize) -> u8 {
        if n < 2 {
            return n as u8;
        }
        return Fibonacci::fibonacci(n - 1).wrapping_add(Fibonacci::fibonacci(n - 2));
    }
}

impl Iterator for Fibonacci {
    type Item = u128;

    /// Calculate the next Fibonacci number.
    ///
    /// The first call to `next()` shall return the 0th Fibonacci number (i.e., `0`).
    /// The calculations shall not overflow and shall not wrap around. If the result
    /// doesn't fit u128, the sequence shall end (the iterator shall return `None`).
    /// The calculations shall be fast (recursive calculations are **un**acceptable).
    fn next(&mut self) -> Option<Self::Item> {
        match self.v_iter_curr {
            Some(v_iter_curr_val) => {
                let v_next = self.v_iter_prev.checked_add(v_iter_curr_val);
                self.v_iter_prev = v_iter_curr_val;
                self.v_iter_curr = v_next;
                Some(self.v_iter_prev)
            }
            None => None,
        }
    }
}
