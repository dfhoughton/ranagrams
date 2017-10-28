use std::mem::size_of;
use util::{CharCount, CharSet, ToDo, Translator};
use std::sync::{RwLock, Arc};
use std::collections::HashMap;
use std::cmp::Ordering;

pub struct Trie {
    pub root: TrieNode,
    pub translator: Translator,
    pub cache: RwLock<HashMap<Arc<CharCount>,Arc<Vec<(Arc<Vec<usize>>,Arc<CharCount>)>>>>,
    pub use_cache: bool,
    empty_list: Arc<Vec<(Arc<Vec<usize>>,Arc<CharCount>)>>,
}

impl Trie {
    pub fn new(root: TrieNode, translator: Translator, use_cache: bool) -> Trie {
        Trie { root, translator, use_cache, cache: RwLock::new(HashMap::new()), empty_list: Arc::new(Vec::with_capacity(0)) }
    }
    // for comparing two sort keys
    fn ge_key(a: &[usize], b: &[usize]) -> Ordering {
        for (i, count) in a.iter().enumerate() {
            if i == b.len() {
                return Ordering::Greater
            }
            let count2 = &b[i];
            if count > count2 {
                return Ordering::Greater
            } else if count < count2 {
                return Ordering::Less
            }
        }
        if a.len() < b.len() {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }
    fn cached_index(key: &[usize], sorted_list: &Arc<Vec<(Arc<Vec<usize>>,Arc<CharCount>)>>) -> usize {
        if sorted_list.len() == 0 {
            0
        // } else if sorted_list.len() < 5 {
        //     // linear search
        //     for (i, &(ref k,_)) in sorted_list.iter().enumerate() {
        //         match Trie::ge_key(key, &k) {
        //             Ordering::Less => return i,
        //             _ => (),
        //         }
        //     }
        //     sorted_list.len() // the key is after all the items in the sorted list
        } else {
            let mut start = 0;
            let mut end = sorted_list.len();
            loop {
                let delta = end - start;
                if delta == 1 {
                    return match Trie::ge_key(key, &sorted_list[start].0) {
                        Ordering::Less => start,
                        _ => end,
                    }
                }
                let middle = start + delta / 2;
                let middle_key = &sorted_list[middle].0;
                match Trie::ge_key(middle_key, key) {
                    Ordering::Less => start = middle,
                    Ordering::Greater => end = middle,
                    Ordering::Equal => return middle,
                }
            }
        }
    }
    pub fn words_for(&self, cc: Arc<CharCount>, sort_key: &[usize]) -> Vec<(Arc<Vec<usize>>,Arc<CharCount>)> {
        let list = if self.use_cache {
            let ref mut mutable = cc.clone();
            let hashed = Arc::make_mut(mutable);
            hashed.calculate_hash();
            let cached = {
                let map = self.cache.read().unwrap();
                map.get(hashed).map(Arc::clone)
            };
            if let Some(list) = cached {
                list.clone()
            } else {
                let list = self.non_caching_words_for(&cc, sort_key);
                {
                    let mut map = self.cache.write().unwrap();
                    map.insert(Arc::new(hashed.clone()), list.clone());
                }
                list
            }
        } else {
            self.non_caching_words_for(&cc, sort_key)
        };
        let mut filtered = Vec::with_capacity(list.len());
        for i in Trie::cached_index(sort_key, &list)..list.len() {
            let (ref word, ref counts) = list[i];
            filtered.push((word.clone(), counts.clone()));
        }
        filtered
    }
    fn non_caching_words_for(&self, cc: &CharCount, sort_key: &[usize]) -> Arc<Vec<(Arc<Vec<usize>>,Arc<CharCount>)>> {
        let mut paired = vec![];
        let seed = Vec::with_capacity(0);
        let mut set = cc.to_set();
        Trie::walk(
            &self.root,
            seed,
            cc,
            &mut set,
            1,
            sort_key,
            !self.use_cache,
            &mut paired,
        );
        if set.is_empty() {
            Arc::new(paired.into_iter().map(|(k,v)| (Arc::new(k), Arc::new(v))).collect())
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
        seed: Vec<usize>,
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
        for c in start..(cc.last + 1) {
            if cc.has(c) {
                if let Some(t) = node.get(c) {
                    let mut characters_remaining = cc.clone();
                    unsafe {
                        characters_remaining.decrement(c);
                    }
                    let mut longer = seed.clone(); // TODO remove cloning here
                    longer.push(c);
                    Trie::walk(
                        t,
                        longer,
                        &characters_remaining,
                        set,
                        level + 1,
                        &sort_key,
                        sorting && (c == sort_char),
                        words,
                    );
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
