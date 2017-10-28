use std::collections::HashMap;
use std::sync::Arc;
use std::cmp::{Eq, PartialEq};
use std::hash::{Hash, Hasher};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}

// a set-ish representation of the characters in a CharCount
#[derive(Debug, Clone)]
pub struct CharSet {
    pub chars: Vec<bool>,
    count: usize,
}

impl CharSet {
    pub fn new(chars: &[usize]) -> CharSet {
        let mut contained = vec![false; chars.len()];
        let mut count = 0;
        for i in 0..chars.len() {
            if chars[i] > 0 {
                contained[i] = true;
                count += 1;
            }
        }
        CharSet {
            chars: contained,
            count,
        }
    }
    pub fn remove(&mut self, word: &[usize]) {
        if !(word.is_empty() && self.is_empty()) {
            for c in word {
                unsafe {
                    self.rm(*c);
                }
                if self.is_empty() {
                    break;
                }
            }
        }
    }
    unsafe fn rm(&mut self, i: usize) {
        if *self.chars.get_unchecked(i) {
            self.chars[i] = false;
            self.count -= 1;
        }
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

#[derive(Clone, Debug)]
pub struct CharCount {
    pub counts: Vec<usize>, // TODO pub only for debugging purposes
    sum: usize,
    pub first: usize, // lowest index with any characters
    pub last: usize,  // highest index with any characters
    hash: u128,       // for quick hashing and equality
}

impl PartialEq for CharCount {
    fn eq(&self, other: &CharCount) -> bool {
        if self.hashed() && other.hashed() {
            self.hash == other.hash
        } else {
            if self.sum != other.sum {
                return false;
            }
            for i in 0..self.counts.len() {
                if self.counts[i] != other.counts[i] {
                    return false;
                }
            }
            true
        }
    }
}

impl Eq for CharCount {}

impl Hash for CharCount {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if self.hash == 0 {
            panic!("should never be hashing counts without a calculated hash")
        }
        self.hash.hash(state);
    }
}

impl CharCount {
    fn hashed(&self) -> bool {
        self.hash > 0 || self.sum == 0
    }
    fn confirm_mutable(&self) {
        if self.hashed() {
            panic!("count should be mutable")
        }
    }
    // calculate the hash -- this treats the character counts as a sort of
    // odometer and reads of the values as one big base-10 number
    pub fn calculate_hash(&mut self) {
        if self.hashed() {
            return; // already calculated
        }
        if self.counts.len() > 38 {
            // u128 can only hold 38.5 base-10 digits
            panic!("your alphabet is too large for the character count caching algorithm")
        }
        let mut i = 0;
        let mut accumulator: u128 = 0;
        for c in &self.counts {
            let mut value = (c % 10) as u128;
            if i > 0 {
                // assumption: if a word has more than 9 of a particular character,
                // there won't be another word for which this is also true which
                // is also identical in every other character count mod 10
                i *= 10;
                value = value * i;
            } else {
                i = 1;
            }
            accumulator = accumulator + value;
        }
        self.hash = accumulator;
    }
    pub unsafe fn decrement(&mut self, i: usize) {
        self.confirm_mutable();
        self.counts[i] -= 1;
        self.sum -= 1;
        if self.sum == 0 {
            self.first = 0;
            self.last = 0;
        } else if self.first != self.last {
            if self.sum == 1 {
                for j in self.first..(self.last + 1) {
                    if self.counts[j] > 0 {
                        self.first = j;
                        self.last = j;
                        break;
                    }
                }
            } else if i == self.first {
                for j in self.first..(self.last + 1) {
                    if self.counts[j] > 0 {
                        self.first = j;
                        break;
                    }
                }
            } else if i == self.last {
                for j in (self.first..(self.last + 1)).rev() {
                    if self.counts[j] > 0 {
                        self.last = j;
                        break;
                    }
                }
            }
        }
    }
    pub fn add(&mut self, word: Vec<usize>) -> bool {
        for i in word {
            if i > self.counts.len() {
                return false;
            }
            unsafe {
                *self.counts.get_unchecked_mut(i) += 1;
            }
            self.sum += 1;
        }
        true
    }
    pub fn subtract(&mut self, word: Vec<usize>) -> bool {
        self.confirm_mutable();
        for i in word {
            if i > self.counts.len() && self.counts[i] == 0 {
                return false;
            }
            unsafe {
                *self.counts.get_unchecked_mut(i) -= 1;
            }
            self.sum -= 1;
        }
        true
    }
    pub fn set_limits(&mut self) {
        let mut looking_for_first = true;
        let mut first = 0;
        let mut last = 0;
        for (i, &c) in self.counts.iter().enumerate() {
            if c > 0 {
                last = i;
                if looking_for_first {
                    first = i;
                    looking_for_first = false;
                }
            }
        }
        self.first = first;
        self.last = last;
    }
    unsafe fn increment(&mut self, i: usize) {
        *self.counts.get_unchecked_mut(i) += 1;
        if self.sum == 0 {
            self.first = i;
            self.last = i;
        } else if i < self.first {
            self.first = i;
        } else if i > self.last {
            self.last = i;
        }
        self.sum += 1;
    }
    pub fn has(&self, i: usize) -> bool {
        unsafe {
            let v = *self.counts.get_unchecked(i);
            // println!("{} => {}; {:?}", i, v, self);
            v > 0
        }
    }
    pub fn to_set(&self) -> CharSet {
        CharSet::new(&self.counts)
    }
    pub fn is_empty(&self) -> bool {
        self.sum == 0
    }
}

pub struct Translator {
    map: HashMap<char, usize>,
    map_back: HashMap<usize, char>,
    pub normalizer: fn(&str) -> String,
}

impl Translator {
    pub fn new<'a, I: Iterator<Item = &'a str>>(
        normalizer: fn(&str) -> String,
        i: I,
    ) -> Translator {
        let mut count_map = HashMap::new();
        for word in i {
            for c in normalizer(word).chars() {
                *count_map.entry(c).or_insert(0) += 1;
            }
        }
        let mut counts = count_map.into_iter().collect::<Vec<_>>();
        counts.sort_by(|&(_, ref a), &(_, ref b)| b.cmp(a));
        let map: HashMap<char, usize> = counts
            .into_iter()
            .enumerate()
            .map(|(i, (c, _))| (c, i))
            .collect();
        let map_back = map.iter().map(|(&c, &i)| (i, c)).collect();
        Translator {
            normalizer,
            map,
            map_back,
        }
    }
    pub fn count(&self, word: &str) -> Option<CharCount> {
        let mut cc = CharCount {
            counts: vec![0; self.map.len()],
            sum: 0,
            first: 0,
            last: 0,
            hash: 0,
        };
        for c in word.chars() {
            if let Some(&i) = self.map.get(&c) {
                unsafe {
                    cc.increment(i);
                }
            } else {
                return None;
            }
        }
        Some(cc)
    }
    pub fn snrt(&self, i: &usize) -> Option<&char> {
        self.map_back.get(i)
    }
    pub fn etalsnart(&self, ints: &[usize]) -> Option<String> {
        let mut word = String::new();
        for i in ints {
            if let Some(&c) = self.map_back.get(i) {
                word.push(c);
            } else {
                return None;
            }
        }
        Some(word)
    }
    pub fn translate(&self, word: &str) -> Option<Vec<usize>> {
        let mut translation = Vec::with_capacity(word.len());
        for c in (self.normalizer)(word).chars() {
            match self.map.get(&c) {
                Some(&i) => translation.push(i),
                None => return None,
            }
        }
        return Some(translation);
    }
}

pub fn normalize(word: &str) -> String {
    word.trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphabetic())
        .collect::<String>()
}

#[derive(Debug)]
pub struct ToDo {
    parent: Option<Arc<ToDo>>,
    pub word: Arc<Vec<usize>>,
    pub undone: Arc<CharCount>,
}

impl ToDo {
    pub fn new(parent: Arc<ToDo>, word: Arc<Vec<usize>>, undone: Arc<CharCount>) -> ToDo {
        ToDo {
            parent: Some(parent),
            word,
            undone,
        }
    }
    pub fn seed(undone: CharCount) -> ToDo {
        ToDo {
            parent: None,
            word: Arc::new(Vec::with_capacity(0)),
            undone: Arc::new(undone),
        }
    }
    fn trace(&self, words: &mut Vec<Vec<usize>>) {
        if !self.word.is_empty() {
            words.push((*self.word).clone());
            if let Some(ref t) = self.parent {
                t.trace(words);
            }
        }
    }
    pub fn words(&self) -> Vec<Vec<usize>> {
        let mut words = vec![];
        self.trace(&mut words);
        words
    }
    pub fn done(&self) -> bool {
        self.undone.is_empty()
    }
}
