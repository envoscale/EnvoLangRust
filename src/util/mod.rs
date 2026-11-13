//! compiler/util: the hash map, text helpers and files, ported line for line.

pub mod file;

/// FNV-1a over the bytes of s (util/text.envo `Hash`).
#[inline]
pub fn hash(s: &[u8]) -> u64 {
    let mut h: u64 = 0xCBF29CE484222325;
    for &b in s {
        h = (h ^ b as u64).wrapping_mul(0x100000001B3);
    }
    h
}

/// Do a and b hold the same bytes? (util/text.envo `Same`)
#[inline]
pub fn same(a: &[u8], b: &[u8]) -> bool {
    a == b
}

pub fn starts_with(s: &[u8], prefix: &[u8]) -> bool {
    s.starts_with(prefix)
}

/// v in decimal (util/text.envo `Decimal`).
pub fn decimal(v: u64) -> Vec<u8> {
    let mut digits = [0u8; 20];
    let mut n = 0;
    let mut v = v;
    loop {
        digits[n] = (48 + v % 10) as u8;
        n += 1;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    let mut s = Vec::with_capacity(n);
    while n > 0 {
        n -= 1;
        s.push(digits[n]);
    }
    s
}

/// Appends v in decimal to s.
pub fn append_decimal(s: &mut Vec<u8>, v: u64) {
    let d = decimal(v);
    s.extend_from_slice(&d);
}

/// A hash map from strings to values (util/map.envo `StrMap`): open addressing with linear
/// probing; the map owns copies of its keys; the table's size is a power of two and stays
/// at most half full.
pub struct StrMap<V> {
    keys: Vec<Vec<u8>>,
    values: Vec<Option<V>>,
    count: usize,
}

impl<V> StrMap<V> {
    pub fn new() -> Self {
        let mut m = StrMap {
            keys: Vec::new(),
            values: Vec::new(),
            count: 0,
        };
        m.fill(16);
        m
    }

    pub fn len(&self) -> usize {
        self.count
    }

    /// Sets key's value (replacing an earlier one).
    pub fn put(&mut self, key: &[u8], value: V) {
        if (self.count + 1) * 2 > self.keys.len() {
            self.grow();
        }
        let i = self.slot(key);
        if self.values[i].is_none() {
            self.keys[i] = key.to_vec();
            self.count += 1;
        }
        self.values[i] = Some(value);
    }

    pub fn has(&self, key: &[u8]) -> bool {
        self.values[self.slot(key)].is_some()
    }

    /// key's value, if it has one.
    pub fn get(&self, key: &[u8]) -> Option<V>
    where
        V: Copy,
    {
        self.values[self.slot(key)]
    }

    pub fn get_ref(&self, key: &[u8]) -> Option<&V> {
        self.values[self.slot(key)].as_ref()
    }

    pub fn get_mut(&mut self, key: &[u8]) -> Option<&mut V> {
        let i = self.slot(key);
        self.values[i].as_mut()
    }

    /// The slot holding key, or the empty slot where it would go.
    fn slot(&self, key: &[u8]) -> usize {
        let mask = self.keys.len() - 1;
        let mut i = (hash(key) as usize) & mask;
        loop {
            if self.values[i].is_none() {
                return i;
            }
            if same(&self.keys[i], key) {
                return i;
            }
            i = (i + 1) & mask;
        }
    }

    fn fill(&mut self, n: usize) {
        for _ in 0..n {
            self.keys.push(Vec::new());
            self.values.push(None);
        }
    }

    /// Twice the slots; every entry is placed again.
    fn grow(&mut self) {
        let size = self.keys.len() * 2;
        let mut keys: Vec<Vec<u8>> = Vec::with_capacity(size);
        let mut values: Vec<Option<V>> = Vec::with_capacity(size);
        for _ in 0..size {
            keys.push(Vec::new());
            values.push(None);
        }
        while let Some(k) = self.keys.pop() {
            let v = self.values.pop().unwrap();
            if let Some(value) = v {
                Self::place(&mut keys, &mut values, k, value);
            }
        }
        self.keys = keys;
        self.values = values;
    }

    fn place(keys: &mut [Vec<u8>], values: &mut [Option<V>], key: Vec<u8>, value: V) {
        let mask = keys.len() - 1;
        let mut i = (hash(&key) as usize) & mask;
        while values[i].is_some() {
            i = (i + 1) & mask;
        }
        keys[i] = key;
        values[i] = Some(value);
    }
}

impl<V> Default for StrMap<V> {
    fn default() -> Self {
        Self::new()
    }
}
