use std::mem::size_of;
use util::{CharCount, CharSet, ToDo, Translator};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::cmp::Ordering;
use rand::{Rng, StdRng};

pub struct Trie {
    pub root: TrieNode,
    pub translator: Translator,
    pub cache: RwLock<HashMap<Arc<CharCount>, Arc<Vec<(Arc<Vec<usize>>, Arc<CharCount>)>>>>,
    pub use_cache: bool,
    pub shuffle: bool,
    empty_list: Arc<Vec<(Arc<Vec<usize>>, Arc<CharCount>)>>,
    rng: Option<StdRng>,
    powers_of_ten: Vec<u128>,
}

impl Trie {
    pub fn new(
        root: TrieNode,
        translator: Translator,
        use_cache: bool,
        shuffle: bool,
        rng: Option<StdRng>,
    ) -> Trie {
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
            rng,
            cache: RwLock::new(HashMap::new()),
            empty_list: Arc::new(Vec::with_capacity(0)),
            powers_of_ten: powers_of_ten,
        }
    }
    // for comparing two sort keys
    fn compare_words(a: &[usize], b: &[usize]) -> Ordering {
        unsafe {
            for (i, count) in a.iter().enumerate() {
                if i == b.len() {
                    return Ordering::Greater;
                }
                let count2 = &b.get_unchecked(i);
                if count > count2 {
                    return Ordering::Greater;
                } else if count < count2 {
                    return Ordering::Less;
                }
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
                if delta == 1 {
                    return match Trie::compare_words(key, &sorted_list[start].0) {
                        Ordering::Less | Ordering::Equal => start,
                        _ => end,
                    };
                }
                let middle = start + delta / 2;
                let middle_key = &sorted_list[middle].0;
                match Trie::compare_words(middle_key, key) {
                    Ordering::Less => start = middle,
                    Ordering::Greater => end = middle,
                    Ordering::Equal => return middle,
                }
            }
        }
    }
    pub fn words_for(
        &self,
        cc: Arc<CharCount>,
        sort_key: &[usize],
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
                let list = self.non_caching_words_for(&cc, sort_key);
                {
                    let mut map = self.cache.write().unwrap();
                    map.insert(hashed, list.clone());
                }
                list
            }
        } else {
            self.non_caching_words_for(&cc, sort_key)
        };
        let mut filtered = Vec::with_capacity(list.len());
        for &(ref word, ref counts) in &list[Trie::index(sort_key, &list)..] {
            filtered.push((word.clone(), counts.clone()));
        }
        if self.shuffle {
            self.rng.unwrap().shuffle(&mut filtered);
        }
        filtered
    }
    fn non_caching_words_for(
        &self,
        cc: &CharCount,
        sort_key: &[usize],
    ) -> Arc<Vec<(Arc<Vec<usize>>, Arc<CharCount>)>> {
        let mut paired = vec![];
        let mut seed = Vec::with_capacity(cc.sum);
        let mut set = cc.to_set();
        Trie::walk(
            &self.root,
            &mut seed,
            cc,
            &mut set,
            1,
            sort_key,
            !self.use_cache,
            &mut paired,
        );
        if set.is_empty() {
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

    pub fn walk(
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
        let start = if !sort || level > sort_key.len() {
            sorting = false;
            cc.first
        } else {
            sort_char = sort_key[level - 1];
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

#[derive(PartialEq, Debug)]
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

#[derive(Clone)]
pub struct TrieNodeBuilder {
    terminal: bool,
    children: Vec<Option<TrieNodeBuilder>>,
}

impl TrieNodeBuilder {
    pub fn new() -> TrieNodeBuilder {
        TrieNodeBuilder {
            terminal: false,
            children: vec![],
        }
    }
    pub fn build(self) -> TrieNode {
        let children = self.children
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
        } else if unsafe { self.children.get_unchecked(i).is_none() } {
            unsafe {
                let child = self.children.get_unchecked_mut(i);
                *child = Some(TrieNodeBuilder::new());
                child
            }
        } else {
            unsafe { self.children.get_unchecked_mut(i) }
        }
    }
    pub fn add(&mut self, word: &[usize]) {
        if word.is_empty() {
            self.terminal = true;
        } else {
            self.get(word[0]).as_mut().unwrap().add(&word[1..]);
        }
    }
}
