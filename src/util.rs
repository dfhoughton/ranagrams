//! collection of structs mediating between words and tries and representing
//! intermediate states in the discovery of anagrams

use std::cmp::{Eq, PartialEq};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Actually, there are currently no tests. The proof is in the pudding.
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}

/// A set-ish representation of the characters in a `CharCount`. A `CharSet`
/// is a record of the *types* of characters present without regard to their
/// count.
#[derive(Debug, Clone)]
pub struct CharSet {
    pub chars: Vec<bool>,
    count: usize,
}

impl CharSet {
    pub fn new(chars: &[usize]) -> CharSet {
        let mut contained = vec![false; chars.len()];
        let mut count = 0;
        unsafe {
            for i in 0..chars.len() {
                if *chars.get_unchecked(i) > 0 {
                    *contained.get_unchecked_mut(i) = true;
                    count += 1;
                }
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
        let b = self.chars.get_unchecked_mut(i);
        if *b {
            *b = false;
            self.count -= 1;
        }
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// The fundamental representation of the undigested bit of a phrase in
/// anagram calculation, a `CharCount` keeps track of the characters still
/// looking for a foster word. To accelerate processing, they also cache
/// the first character offset with a non-zero count, the last such offset,
/// the sum of their counts, and a checksum sufficient for hashing and
/// identification.
#[derive(Clone, Debug)]
pub struct CharCount {
    pub counts: Vec<usize>, // TODO pub only for debugging purposes
    pub sum: usize,
    pub first: usize, // lowest index with any characters
    pub last: usize,  // highest index (+1) with any characters
    hash: u128,       // for quick hashing and equality
}

impl PartialEq for CharCount {
    fn eq(&self, other: &CharCount) -> bool {
        if self.hashed() && other.hashed() {
            self.hash == other.hash
        } else {
            if !(self.sum == other.sum && self.first == other.first && self.last == other.last) {
                return false;
            }
            unsafe {
                for i in 0..self.last {
                    if *self.counts.get_unchecked(i) != *other.counts.get_unchecked(i) {
                        return false;
                    }
                }
            }
            true
        }
    }
}

impl Eq for CharCount {}

impl Hash for CharCount {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl CharCount {
    pub fn hashed(&self) -> bool {
        self.hash > 0 || self.sum == 0
    }
    // mostly just for debugging
    // fn confirm_mutable(&self) {
    //     if self.hashed() {
    //         panic!("count should be mutable")
    //     }
    // }
    // calculate the hash -- this treats the character counts as a sort of
    // odometer and reads of the values as one big base-10 number
    pub fn calculate_hash(&mut self, powers_of_ten: &[u128]) {
        if self.hashed() {
            return; // already calculated
        }
        let mut accumulator: u128 = 0;
        unsafe {
            for i in self.first..self.last {
                let c = self.counts.get_unchecked(i);
                if c > &0 {
                    let p = powers_of_ten.get_unchecked(i);
                    let value = (c % 10) as u128;
                    accumulator = accumulator + value * p;
                }
            }
        }
        self.hash = accumulator;
    }
    pub unsafe fn decrement(&mut self, i: usize) {
        *self.counts.get_unchecked_mut(i) -= 1;
        self.sum -= 1;
        if self.sum == 0 {
            self.first = 0;
            self.last = 1;
        } else if self.first + 1 != self.last {
            if self.sum == 1 {
                for j in self.first..self.last {
                    if *self.counts.get_unchecked(j) > 0 {
                        self.first = j;
                        self.last = j + 1;
                        break;
                    }
                }
            } else if i == self.first {
                for j in self.first..self.last {
                    if *self.counts.get_unchecked(j) > 0 {
                        self.first = j;
                        break;
                    }
                }
            } else if i + 1 == self.last {
                for j in (self.first..self.last).rev() {
                    if *self.counts.get_unchecked(j) > 0 {
                        self.last = j + 1;
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
    pub fn subtract(&mut self, word: Vec<usize>) -> Option<(usize, Vec<usize>)> {
        // self.confirm_mutable();
        let copy = word.clone();
        for (idx, &i) in word.iter().enumerate() {
            if i >= self.counts.len() || self.counts[i] == 0 {
                return Some((idx, copy));
            }
            self.counts[i] -= 1;
            self.sum -= 1;
        }
        None
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
        self.last = last + 1;
    }
    unsafe fn increment(&mut self, i: usize) {
        *self.counts.get_unchecked_mut(i) += 1;
        if self.sum == 0 {
            self.first = i;
            self.last = i + 1;
        } else if i < self.first {
            self.first = i;
        } else if i >= self.last {
            self.last = i + 1;
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
/// A `Translator` converts between alphabetic and numeric representations of
/// words. For anagram calculation words are treated as pure numeric sequences.
/// The translator converts back and forth and also keeps track of character
/// frequences in order to produce a dense trie representation of a word list.
#[derive(Clone)]
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
    pub fn alphabet_size(&self) -> usize {
        self.map.len()
    }
    pub fn count(&self, word: &str) -> Option<CharCount> {
        let mut cc = CharCount {
            counts: vec![0; self.map.len()],
            sum: 0,
            first: 0,
            last: 1,
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
    /// for construction of an error message when translate fails
    pub fn unfamiliar_character(&self, word: &str) -> (String, String) {
        let mut s1 = String::new();
        let mut s2 = String::new();
        let mut broken = false;
        for c in (self.normalizer)(word).chars() {
            if broken {
                s2.push(c);
            } else {
                match self.map.get(&c) {
                    Some(_) => {
                        s1.push(c);
                    }
                    None => {
                        broken = true;
                        s2.push(c);
                    }
                }
            }
        }
        (s1, s2)
    }
}
/// A function that strips away characters of no interest -- spaces and
/// punctuation characters, generally -- and removes unimportant distinctions
/// like case. If one wishes to convert this code to a new alphabet this is
/// likely the only things that needs fixing.
pub fn normalize(word: &str) -> String {
    word.trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphabetic())
        .collect::<String>()
}
/// The representation of a partially processed phrase working its way through
/// anagram discovery. `ToDo`s are a linked list keeping track of words already
/// found plus a `CharCount` keeping track of the characters yet to be
/// processed.
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
