extern crate ranagrams;
use ranagrams::util::{normalize, ToDo, Translator};
use ranagrams::trie::{Trie, TrieNodeBuilder};
use std::io::{Read};
use std::fs::File;
use ranagrams::factory;
use factory::WorkerFun;
use std::sync::Arc;
use std::sync::atomic::Ordering;
extern crate clap;
use clap::{Arg, App, ArgMatches};
extern crate num_cpus;

macro_rules! parse_cli {
    ($num_cpus:ident, $dest:ident) => {
        let $num_cpus = num_cpus::get().to_string();
        let $dest = App::new("ranagrams")
                .version("0.1")
                .author("David F. Houghton <dfhoughton@gmail.com>")
                .about("Finds anagrams of a phrase")
                .after_help(
        "Put stuff to appear after the list of options here."
                )
                .arg(
                    Arg::with_name("dictionary")
                        .short("d")
                        .long("dictionary")
                        .value_name("file")
                        .default_value("/Users/houghton/mostly_inoffensive_words.txt") // FIXME
                        .help("a line-delimited list of words usable in anagrams")
                        .takes_value(true)
                )
                .arg(
                    Arg::with_name("set")
                        .short("w")
                        .long("words-in")
                        .help("Returns the set of words composable from the letters in the input phrase")
                )
                .arg(
                    Arg::with_name("include")
                        .short("i")
                        .long("include")
                        .value_name("word")
                        .help("include this word in the anagrams")
                        .takes_value(true)
                        .multiple(true)
                        .number_of_values(1)
                        .conflicts_with("set")
                )
                .arg(
                    Arg::with_name("threads")
                        .short("t")
                        .long("threads")
                        .takes_value(true)
                        .default_value(&$num_cpus)
                        .value_name("n")
                        .help("the number of threads to use during anagram collection")
                )
                .arg(
                    Arg::with_name("limit")
                        .short("l")
                        .long("limit")
                        .takes_value(true)
                        .value_name("n")
                        .help("only find this many anagrams")
                        .conflicts_with("set")
                )
                .arg(
                    Arg::with_name("phrase")
                        .value_name("word")
                        .multiple(true)
                        .required(true)
                        .help("the words for which you want an anagram")
                )
                .get_matches();
    };
}

fn main() {
    // parse the options
    parse_cli!(n, options);
    let threads = match usize::from_str_radix(options.value_of("threads").unwrap(), 10) {
        Err(why) => {
            panic!("error parsing thread count: {}\n\n{}", why, options.usage());
        },
        Ok(threads) => threads
    };
    let use_limit = options.is_present("limit");
    let limit = if use_limit {
        match usize::from_str_radix(options.value_of("limit").unwrap(), 10) {
            Err(why) => panic!("could not parse anagram limit: {}\n\n{}", why, options.usage()),
            Ok(limit) => limit
        }
    } else {
        0
    };
    let trie = make_trie(&options);
    let af = AnagramFun {
        root: trie,
    };

    // create initial character count
    let mut cc = af.root
        .translator
        .count(&normalize(""))
        .expect("no luck with the char count");
    // add all the words to anagramize
    for word in options.values_of("phrase").unwrap() {
        if let Some(usizes) = af.root.translator.translate(word) {
            if !cc.add(usizes) {
                panic!(
                    "{} contains characters not in any word in the dictionary",
                    word
                );
            }
        } else {
            panic!(
                "{} contains characters not in any word in the dictionary",
                word
            );
        }
    }
    // subtract the words to include
    let prefixed = options.is_present("include");
    let mut prefix = String::new();
    if prefixed {
        for word in options.values_of("include").unwrap() {
            if let Some(usizes) = af.root.translator.translate(word) {
                if cc.subtract(usizes) {
                    prefix.push_str(word);
                    prefix.push(' ');
                } else {
                    panic!(
                        "{} contains characters not present in the input phrase",
                        word
                    );
                }
            } else {
                panic!(
                    "{} contains characters not in any word in the dictionary",
                    word
                );
            }
        }
    }
    cc.set_limits();

    if options.is_present("set") {
        let sort_key = Vec::with_capacity(0);
        let mut found : Vec<String> = af.root.words_for(&cc, &sort_key)
            .into_iter()
            .map(|(chars,_)| af.root.translator.etalsnart(&chars).unwrap() )
            .collect();
        found.sort();
        for word in found {
            println!("{}", word);
        }
    } else {
        let mut count = 0;
        let materials = vec![ToDo::seed(cc)];
        let noah = Arc::new(af);
        let mine = noah.clone();
        let (messages, kill_switch) = factory::manufacture(threads, materials, noah);
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

fn make_trie(opts: &ArgMatches) -> Trie {
    let mut file = match File::open(opts.value_of("dictionary").unwrap()) {
        Err(_) => panic!("could not read dictionary:\n\n{}", opts.usage()),
        Ok(file) => file
    };
    let mut strings = String::new();
    match file.read_to_string(&mut strings) {
        Err(why) => panic!("could not read words from dictionary: {}\n\n{}", why, opts.usage()),
        Ok(_) => (),
    }
    let words : Vec<&str> = strings.lines().collect();
    let translator = Translator::new(normalize, words.iter().map(|s| *s));
    let mut t = TrieNodeBuilder::new();
    for word in words {
        t.add(&translator.translate(word).unwrap());
    }
    Trie::new(t.build(), translator)
}

struct AnagramFun {
    root: Trie,
}

impl WorkerFun<ToDo> for AnagramFun {
    fn improve(&self, needs_work: ToDo) -> Vec<ToDo> {
        let mut done = vec![];
        let arc = Arc::new(needs_work);
        for (word, cc) in self.root.words_for(&arc.undone, &arc.word) {
            done.push(ToDo::new(arc.clone(), word, cc))
        }
        done
    }
    fn inspect(&self, thing: &ToDo) -> bool {
        thing.done()
    }
}
