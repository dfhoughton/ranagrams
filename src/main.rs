extern crate ranagrams;
use factory::WorkerFun;
use ranagrams::cli;
use ranagrams::factory;
use ranagrams::trie::{Trie, TrieNodeBuilder};
use ranagrams::util::{normalize, ToDo, Translator};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::sync::atomic::Ordering;
use std::sync::Arc;
extern crate clap;
use clap::ArgMatches;
extern crate rand;
use rand::StdRng;
extern crate num_cpus;
use std::ops::Deref;
use std::process;
extern crate dirs;
use dirs::home_dir;

fn main() {
    // parse the options
    let cpus = num_cpus::get().to_string();
    let default_dir = if let Some(mut buf) = home_dir() {
        buf.push(".anagram-dictionary.txt");
        Some(buf.to_str().unwrap().to_string())
    } else {
        None
    };
    let options = cli::parse(&cpus, default_dir.as_ref().map(String::deref)).get_matches();
    if options.is_present("long-help") {
        cli::parse(&cpus, default_dir.as_ref().map(String::deref))
            .print_help()
            .ok();
        println!("\n\n{}", cli::long_help());
        process::exit(0)
    }
    if options.is_present("ribbit") {
        println!("\n{}", include_str!("../rana.txt"));
        process::exit(0)
    }
    let threads = if options.is_present("set") && !options.is_present("strict") {
        // only one thread will ever be used
        1
    } else {
        match usize::from_str_radix(options.value_of("threads").unwrap(), 10) {
            Err(why) => {
                eprintln!("error parsing thread count: {}\n\n{}", why, options.usage());
                process::exit(1)
            }
            Ok(threads) => threads,
        }
    };
    let use_limit = options.is_present("limit");
    let limit = if use_limit {
        match usize::from_str_radix(options.value_of("limit").unwrap(), 10) {
            Err(why) => {
                eprintln!(
                    "could not parse anagram limit: {}\n\n{}",
                    why,
                    options.usage()
                );
                process::exit(1)
            }
            Ok(limit) => limit,
        }
    } else {
        0
    };
    let min_word_length = if options.is_present("min") {
        match usize::from_str_radix(options.value_of("min").unwrap(), 10) {
            Err(why) => {
                eprintln!(
                    "could not parse minimum word length: {}\n\n{}",
                    why,
                    options.usage()
                );
                process::exit(1)
            }
            Ok(min) => {
                if min == 0 {
                    eprintln!(
                        "minimum word length must be positive\n\n{}",
                        options.usage()
                    );
                    process::exit(1)
                } else {
                    min
                }
            }
        }
    } else {
        1
    };
    let trie_word_length = if options.is_present("strict") || options.is_present("prove") {
        1
    } else {
        min_word_length
    };
    let trie = make_trie(&options, trie_word_length);
    let af = AnagramFun { root: trie };

    // create initial character count
    let mut cc = af
        .root
        .translator
        .count(&normalize(""))
        .expect("no luck with the char count");
    // add all the words to anagramize
    for word in options.values_of("phrase").unwrap() {
        if let Some(usizes) = af.root.translator.translate(word) {
            if !cc.add(usizes) {
                dictionary_error(word, &af)
            }
        } else {
            dictionary_error(word, &af)
        }
    }
    // subtract the words to include
    let prefixed = options.is_present("include");
    let mut prefix = String::new();
    if prefixed {
        for word in options.values_of("include").unwrap() {
            if let Some(usizes) = af.root.translator.translate(word) {
                match cc.subtract(usizes) {
                    Some((i, copy)) => {
                        let normalized = af.root.translator.etalsnart(&copy).unwrap();
                        eprintln!(
                            "attempt to use unavailable character in {}:\n\n\t{}-->{}",
                            &normalized,
                            &normalized[0..i],
                            &normalized[i..]
                        );
                        process::exit(1)
                    }
                    None => {
                        prefix.push_str(word);
                        prefix.push(' ');
                    }
                }
            } else {
                dictionary_error(word, &af)
            }
        }
    }
    cc.set_limits();

    if options.is_present("set") {
        if options.is_present("strict") || options.is_present("prove") {
            let prove = options.is_present("prove");
            let sort_key = Vec::with_capacity(0);
            let mut found: Vec<String> = af
                .root
                .words_for(Arc::new(cc.clone()), &sort_key, &true)
                .into_iter()
                .map(|(chars, _)| af.root.translator.etalsnart(&chars).unwrap())
                .collect();
            found.sort();
            let noah = Arc::new(af);
            let mut cache: HashMap<String, String> = HashMap::new();
            for word in found {
                if word.len() >= min_word_length {
                    if cache.contains_key(&word) {
                        println!("{}", word);
                        if prove {
                            println!("\t{}", cache.get(&word).unwrap());
                        }
                    } else {
                        let noah = noah.clone();
                        let mine = noah.clone();
                        if let Some(usizes) = noah.clone().root.translator.translate(word.as_str())
                        {
                            // can we make a least one anagram with the remainder after we subtract this word?
                            let mut cc = cc.clone();
                            cc.subtract(usizes.to_vec());
                            let materials = vec![ToDo::seed(cc)];
                            let (messages, kill_switch) =
                                factory::manufacture(threads, 3, materials, noah.clone());
                            if let Some(Some(done)) = messages.iter().next() {
                                kill_switch.store(true, Ordering::Relaxed);
                                println!("{}", word);
                                let s = if prove {
                                    word.clone() + " " + &mine.root.stringify(done)
                                } else {
                                    mine.root.stringify(done)
                                };
                                if prove {
                                    println!("\t{}", s);
                                } else {
                                    // only do this if we're not proving, because we aren't always saving the proof in the cache
                                    for ow in words_in_word(&word, noah.clone(), threads) {
                                        if ow.len() >= min_word_length
                                            && ow > word
                                            && !cache.contains_key(&ow)
                                        {
                                            cache.insert(ow, s.clone());
                                        }
                                    }
                                }
                                for other_word in s.as_str().split(" ") {
                                    if prove {
                                        if other_word.len() >= min_word_length && other_word > &word
                                        {
                                            cache.insert(String::from(other_word), s.clone());
                                        }
                                    } else {
                                        for ow in words_in_word(other_word, noah.clone(), threads) {
                                            if ow.len() >= min_word_length
                                                && ow > word
                                                && !cache.contains_key(&ow)
                                            {
                                                cache.insert(ow, s.clone());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            let sort_key = Vec::with_capacity(0);
            let mut found: Vec<String> = af
                .root
                .words_for(Arc::new(cc), &sort_key, &true)
                .into_iter()
                .map(|(chars, _)| af.root.translator.etalsnart(&chars).unwrap())
                .collect();
            found.sort();
            for word in found {
                println!("{}", word);
            }
        }
    } else {
        let mut count = 0;
        let materials = vec![ToDo::seed(cc)];
        let noah = Arc::new(af);
        let mine = noah.clone();
        let (messages, kill_switch) = factory::manufacture(threads, 3, materials, noah);
        for m in messages {
            if let Some(todo) = m {
                if prefixed {
                    print!("{}", prefix);
                }
                println!("{}", mine.root.stringify(todo));
                if use_limit {
                    count += 1;
                    if count == limit {
                        kill_switch.store(true, Ordering::Relaxed);
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }
}

// fetch the anagrams of a word -- useful for --strict and --prove
fn words_in_word(word: &str, noah: Arc<AnagramFun>, threads: usize) -> Vec<String> {
    let mut cc = noah
        .root
        .translator
        .count(&normalize(""))
        .expect("no luck with the char count");
    if let Some(usizes) = noah.root.translator.translate(word) {
        if !cc.add(usizes) {
            dictionary_error(word, &noah)
        }
    } else {
        dictionary_error(word, &noah)
    }
    let materials = vec![ToDo::seed(cc)];
    let (messages, _) = factory::manufacture(threads, 3, materials, noah.clone());
    let mut words = vec![];
    for m in messages {
        if let Some(todo) = m {
            for word in todo.words() {
                words.push(noah.root.translator.etalsnart(&word).unwrap());
            }
        } else {
            break;
        }
    }
    words
}

fn dictionary_error(word: &str, af: &AnagramFun) -> ! {
    let (good, bad) = af.root.translator.unfamiliar_character(word);
    eprintln!(
        "character in {} not present in any word in dictionary:\n\n\t{}-->{}",
        word, good, bad
    );
    process::exit(1)
}

fn make_trie(opts: &ArgMatches, minimum_word_length: usize) -> Trie {
    let mut file = match File::open(opts.value_of("dictionary").unwrap()) {
        Err(_) => {
            eprintln!("could not read dictionary:\n\n{}", opts.usage());
            process::exit(1)
        }
        Ok(file) => file,
    };
    let mut strings = String::new();
    match file.read_to_string(&mut strings) {
        Err(why) => {
            eprintln!(
                "could not read words from dictionary: {}\n\n{}",
                why,
                opts.usage()
            );
            process::exit(1)
        }
        Ok(_) => (),
    }
    let words: Vec<&str> = strings
        .lines()
        .filter(|w| w.trim().len() >= minimum_word_length)
        .collect();
    let translator = Translator::new(normalize, words.iter().map(|s| *s));
    let any_excluded = opts.is_present("exclude");
    let mut excluded = HashSet::new();
    if any_excluded {
        for word in opts.values_of("exclude").unwrap() {
            excluded.insert(translator.translate(word).unwrap());
        }
    }
    let mut t = TrieNodeBuilder::new();
    for word in words {
        let translation = translator.translate(word).unwrap();
        if any_excluded && excluded.contains(&translation) {
            continue;
        }
        t.add(&translation);
    }
    let random = opts.is_present("random");
    let rng = if random {
        Some(StdRng::new().unwrap())
    } else {
        None
    };
    Trie::new(
        t.build(),
        translator,
        !(opts.is_present("no_cache") || opts.is_present("set")),
        random,
        rng,
    )
}

struct AnagramFun {
    root: Trie,
}

impl WorkerFun<ToDo> for AnagramFun {
    fn improve(&self, needs_work: ToDo) -> Vec<ToDo> {
        let mut done = vec![];
        let arc = Arc::new(needs_work);
        for (word, cc) in self.root.words_for(arc.undone.clone(), &arc.word, &false) {
            done.push(ToDo::new(arc.clone(), word, cc.clone()))
        }
        done
    }
    fn inspect(&self, thing: &ToDo) -> bool {
        thing.done()
    }
}
