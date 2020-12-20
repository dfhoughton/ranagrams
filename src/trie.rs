//! efficient representation of word lists

// use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::mem::size_of;
use std::sync::{Arc, RwLock};
use util::{CharCount, CharSet, ToDo, Translator};

/// The magical boundary between words and numbers, a `Trie` wraps a `TrieNode`
/// and various things used for stringification, destringification, and various
/// caches and denormalizations.
pub struct Trie {
    pub root: TrieNode,
    pub translator: Translator,
    pub cache: RwLock<HashMap<Arc<CharCount>, Arc<Vec<(Arc<Vec<usize>>, Arc<CharCount>)>>>>,
    pub use_cache: bool,
    pub shuffle: bool,
    empty_list: Arc<Vec<(Arc<Vec<usize>>, Arc<CharCount>)>>,
    powers_of_ten: Vec<u128>,
}

impl Trie {
    pub fn new(root: TrieNode, translator: Translator, use_cache: bool, shuffle: bool) -> Trie {
        let powers_of_ten = if use_cache {
            let n = translator.alphabet_size();
            if n > 38 {
                panic!("the cache only works with alphabets of 38 characters or fewer")
            }
            let mut powers_of_ten = Vec::with_capacity(n);
            let mut p: u128 = 1;
            for _ in 0..n {
                powers_of_ten.push(p);
                p = p * 10;
            }
            powers_of_ten
        } else {
            Vec::with_capacity(0)
        };
        Trie {
            root,
            translator,
            use_cache,
            shuffle,
            cache: RwLock::new(HashMap::new()),
            empty_list: Arc::new(Vec::with_capacity(0)),
            powers_of_ten: powers_of_ten,
        }
    }
    // for comparing two sort keys
    fn compare_words(a: &[usize], b: &[usize]) -> Ordering {
        for (i, count) in a.iter().enumerate() {
            if i >= b.len() {
                return Ordering::Greater;
            }
            let count2 = b.get(i).unwrap();
            if count > count2 {
                return Ordering::Greater;
            } else if count < count2 {
                return Ordering::Less;
            }
        }
        if a.len() < b.len() {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }
    fn index(key: &[usize], sorted_list: &Arc<Vec<(Arc<Vec<usize>>, Arc<CharCount>)>>) -> usize {
        if sorted_list.len() == 0 {
            0
        } else {
            let mut start = 0;
            let mut end = sorted_list.len();
            loop {
                let delta = end - start;
                unsafe {
                    if delta == 1 {
                        return match Trie::compare_words(key, &sorted_list.get_unchecked(start).0) {
                            Ordering::Less | Ordering::Equal => start,
                            _ => end,
                        };
                    }
                    let middle = start + delta / 2;
                    let middle_key = &sorted_list.get_unchecked(middle).0;
                    match Trie::compare_words(middle_key, key) {
                        Ordering::Less => start = middle,
                        Ordering::Greater => end = middle,
                        Ordering::Equal => return middle,
                    }
                }
            }
        }
    }
    /// Removes the given word from the trie
    pub fn remove(&mut self, word: &[usize]) {
        let n = self.root.clone().remove(word);
        self.root = if let Some(n) = n {
            n
        } else {
            TrieNodeBuilder::new().build()
        };
    }
    /// Produces the words, in their numeric representation, extractable from
    /// a `CharCount` along with the residual `CharCount`s remaining after their
    /// extraction. More precisely, it is those words sorting at or above the
    /// order of the given sort key. The sort key ensures that only one
    /// permutation of a given anagram is produced.
    pub fn words_for(
        &self,
        cc: Arc<CharCount>,
        sort_key: &[usize],
        all_words: &bool,
    ) -> Vec<(Arc<Vec<usize>>, Arc<CharCount>)> {
        let list = if self.use_cache {
            let hashed = if !cc.hashed() {
                let ref mut mutable = cc.clone();
                let mut hashed = Arc::make_mut(mutable).clone();
                hashed.calculate_hash(&self.powers_of_ten);
                Arc::new(hashed)
            } else {
                cc.clone()
            };
            let cached = {
                let map = self.cache.read().unwrap();
                map.get(&hashed).map(Arc::clone)
            };
            if let Some(list) = cached {
                list.clone()
            } else {
                let list = self.non_caching_words_for(&cc, sort_key, all_words);
                {
                    let mut map = self.cache.write().unwrap();
                    map.insert(hashed, list.clone());
                }
                list
            }
        } else {
            self.non_caching_words_for(&cc, sort_key, all_words)
        };
        let mut filtered = Vec::with_capacity(list.len());
        for &(ref word, ref counts) in &list[Trie::index(sort_key, &list)..] {
            filtered.push((word.clone(), counts.clone()));
        }
        if self.shuffle {
            let mut rng = thread_rng();
            filtered.shuffle(&mut rng);
        }
        filtered
    }
    // a repeated bit factored out of words_for (necessary after adding caching)
    fn non_caching_words_for(
        &self,
        cc: &CharCount,
        sort_key: &[usize],
        all_words: &bool,
    ) -> Arc<Vec<(Arc<Vec<usize>>, Arc<CharCount>)>> {
        let mut paired = vec![];
        let mut seed = Vec::with_capacity(cc.sum);
        let mut set = cc.to_set();
        Trie::walk(
            &self.root,
            &mut seed,
            cc,
            &mut set,
            0,
            sort_key,
            !self.use_cache,
            &mut paired,
        );
        if *all_words || set.is_empty() {
            Arc::new(
                paired
                    .into_iter()
                    .map(|(k, v)| (Arc::new(k), Arc::new(v)))
                    .collect(),
            )
        } else {
            // there was some character for which we could find no use
            // it therefore won't be possible to find a use for this character with smaller
            // character counts
            self.empty_list.clone()
        }
    }
    // create a new Trie containing only the words present in the given character count
    pub fn optimize(self, cc: CharCount) -> Trie {
        let mut tnb = TrieNodeBuilder::new();
        for (word, _) in self.words_for(Arc::new(cc), &Vec::with_capacity(0), &true) {
            tnb.add(&word);
        }
        Trie::new(tnb.build(), self.translator, self.use_cache, self.shuffle)
    }
    /// Convert a `ToDo` from a linked list of words in numeric representation
    /// to a single `String` representing an anagram.
    pub fn stringify(&self, todo: ToDo) -> String {
        let mut s = String::new();
        let words = todo.words();
        for (i, w) in words.into_iter().enumerate() {
            if let Some(word) = self.translator.etalsnart(&w) {
                if i > 0 {
                    s.push(' ');
                }
                s.push_str(&word);
            }
        }
        s
    }
    // walk the trie, extending an extraction as far as possible from the given
    // `TrieNode`
    fn walk(
        node: &TrieNode,
        seed: &mut Vec<usize>,
        cc: &CharCount,
        set: &mut CharSet,
        level: usize,
        sort_key: &[usize],
        sort: bool,
        words: &mut Vec<(Vec<usize>, CharCount)>,
    ) {
        if node.terminal && !seed.is_empty() {
            words.push((seed.clone(), cc.clone()));
            set.remove(&seed);
        }
        if cc.is_empty() {
            return;
        }
        let mut sorting = sort;
        let mut sort_char = 0;
        let start = if !sort || level >= sort_key.len() {
            sorting = false;
            cc.first
        } else {
            sort_char = sort_key[level];
            if sort_char < cc.first {
                cc.first
            } else {
                sort_char
            }
        };
        for c in start..cc.last {
            if cc.has(c) {
                if let Some(t) = node.get(c) {
                    let mut characters_remaining = cc.clone();
                    unsafe {
                        characters_remaining.decrement(c);
                    }
                    seed.push(c);
                    Trie::walk(
                        t,
                        seed,
                        &characters_remaining,
                        set,
                        level + 1,
                        &sort_key,
                        sorting && (c == sort_char),
                        words,
                    );
                    seed.pop();
                }
            }
        }
    }
}

impl Clone for Trie {
    fn clone(&self) -> Self {
        Trie {
            root: self.root.clone(),
            translator: self.translator.clone(),
            use_cache: self.use_cache.clone(),
            shuffle: self.shuffle.clone(),
            cache: RwLock::new(self.cache.read().unwrap().clone()),
            empty_list: self.empty_list.clone(),
            powers_of_ten: self.powers_of_ten.clone(),
        }
    }
}

/// A node in a trie (re`trie`val tree) representing a word list. A `TrieNode`
/// contains a boolean indicating whether it is the end of a word and a list
/// of child nodes representing possible continuations of the prefix represented
/// by the node itself.
#[derive(PartialEq, Debug, Clone)]
pub struct TrieNode {
    pub terminal: bool,
    pub children: Box<[Option<TrieNode>]>,
}

impl TrieNode {
    pub fn contains(&self, word: &[usize]) -> bool {
        if word.is_empty() {
            self.terminal
        } else {
            if let Some(&Some(ref child)) = self.children.get(word[0]) {
                child.contains(&word[1..])
            } else {
                false
            }
        }
    }
    fn remove(mut self, word: &[usize]) -> Option<TrieNode> {
        if !self.contains(word) {
            Some(self)
        } else {
            if word.is_empty() {
                if self.children.is_empty() {
                    None
                } else {
                    self.terminal = false;
                    Some(self)
                }
            } else {
                let i = word[0];
                let mut v = self.children.to_vec();
                let n = v.remove(i).unwrap().remove(&word[1..]);
                if n.is_none() {
                    if v.iter().all(|ref o| o.is_none()) {
                        //
                        if self.terminal {
                            self.children = vec![].into_boxed_slice();
                            Some(self)
                        } else {
                            None
                        }
                    } else {
                        if i < v.len() {
                            v.insert(i, n);
                        }
                        self.children = v.into_boxed_slice();
                        Some(self)
                    }
                } else {
                    self.children[i] = n;
                    Some(self)
                }
            }
        }
    }
    pub fn size(&self) -> usize {
        let mut s = 0;
        s += size_of::<Box<[Option<TrieNode>]>>();
        for c in self.children.iter() {
            s += match c {
                &Some(ref t) => {
                    s += size_of::<Option<TrieNode>>();
                    t.size()
                }
                &None => size_of::<Option<TrieNode>>(),
            }
        }
        s += size_of::<bool>();
        s
    }
    pub fn get(&self, i: usize) -> Option<&TrieNode> {
        self.children.get(i).and_then(|o| o.as_ref())
    }
}
/// A disposable stage that launches a `TrieNode`. `TrieNodeBuilder`s maintain
/// a mutable state and functionality that are not necessary for a completed
/// `TrieNode`.
#[derive(Clone)]
pub struct TrieNodeBuilder {
    terminal: bool,
    children: Vec<Option<TrieNodeBuilder>>,
}

impl TrieNodeBuilder {
    /// Begins a builder for a `TrieNode`. The initial state of a
    /// `TrieNodeBuilder` represents an empty, non-terminal `TrieNode`.
    pub fn new() -> TrieNodeBuilder {
        TrieNodeBuilder {
            terminal: false,
            children: vec![],
        }
    }
    /// Recursively compiles a `TrieNode` representing the state of this
    /// `TrieNodeBuilder` and its children.
    pub fn build(self) -> TrieNode {
        let children = self
            .children
            .into_iter()
            .map(|opt| opt.and_then(|c| Some(c.build())))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        TrieNode {
            terminal: self.terminal,
            children: children,
        }
    }
    fn get(&mut self, i: usize) -> &mut Option<TrieNodeBuilder> {
        if i >= self.children.len() {
            self.children.resize(i + 1, None);
            unsafe {
                let child = self.children.get_unchecked_mut(i);
                *child = Some(TrieNodeBuilder::new());
                child
            }
        } else if self.children.get(i).unwrap().is_none() {
            let child = &mut self.children[i];
            *child = Some(TrieNodeBuilder::new());
            child
        } else {
            &mut self.children[i]
        }
    }
    /// Adds a word to the trie.
    pub fn add(&mut self, word: &[usize]) {
        if word.is_empty() {
            self.terminal = true;
        } else {
            self.get(word[0]).as_mut().unwrap().add(&word[1..]);
        }
    }
}
