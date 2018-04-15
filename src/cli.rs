//! command line specification and usage information

extern crate clap;
use self::clap::{App, Arg};

pub fn parse<'a>(cpus: &'a str, dictionary: Option<&'a str>) -> App<'a, 'a> {
    let mut dictionary_argument = Arg::with_name("dictionary")
        .short("d")
        .long("dictionary")
        .value_name("file")
        .help("A line-delimited list of words usable in anagrams")
        .takes_value(true);
    if let Some(file) = dictionary {
        dictionary_argument = dictionary_argument.default_value(file);
    }
    App::new("rana")
        .version("0.1")
        .author("David F. Houghton <dfhoughton@gmail.com>, jwmhjwmh@gmail.com")
        .about("Finds anagrams of a phrase")
        .arg(dictionary_argument)
        .arg(
            Arg::with_name("set")
                .short("w")
                .long("words-in")
                .display_order(1)
                .help("Returns the set of words composable from the letters in the input phrase"),
        )
        .arg(
            Arg::with_name("strict")
                .long("strict")
                .requires("set")
                .display_order(2)
                .help("When finding --words-in, returns only words that occur in some anagram"),
        )
        .arg(
            Arg::with_name("prove")
                .long("prove")
                .requires("set")
                .display_order(3)
                .help("Like --strict, but emits a phrase proving this word occurs in an anagram."),
        )
        .arg(
            Arg::with_name("include")
                .short("i")
                .long("include")
                .value_name("word")
                .help("Include this word in the anagrams")
                .takes_value(true)
                .empty_values(false)
                .multiple(true)
                .number_of_values(1),
        )
        .arg(
            Arg::with_name("exclude")
                .short("x")
                .long("exclude")
                .value_name("word")
                .help("Exclude this word from anagrams")
                .takes_value(true)
                .empty_values(false)
                .multiple(true)
                .number_of_values(1),
        )
        .arg(
            Arg::with_name("threads")
                .short("t")
                .long("threads")
                .takes_value(true)
                .empty_values(false)
                .default_value(cpus)
                .value_name("n")
                .help("The number of threads to use during anagram collection"),
        )
        .arg(
            Arg::with_name("limit")
                .short("l")
                .long("limit")
                .takes_value(true)
                .empty_values(false)
                .value_name("n")
                .help("Only find this many anagrams")
                .conflicts_with("set"),
        )
        .arg(
            Arg::with_name("min")
                .short("m")
                .long("minimum-word-length")
                .takes_value(true)
                .empty_values(false)
                .value_name("n")
                .help("Words in anagrams must be at least this long"),
        )
        .arg(
            Arg::with_name("phrase")
                .value_name("word")
                .multiple(true)
                .required_unless_one(&["long-help", "ribbit"])
                .help("The words for which you want an anagram"),
        )
        .arg(
            Arg::with_name("no_cache")
                .short("C")
                .long("no-cache")
                .help("Do not cache partial results (this saves memory and costs speed)"),
        )
        .arg(
            Arg::with_name("random")
                .short("r")
                .long("random")
                .help("(Partially) shuffle order of discovery"),
        )
        .arg(
            Arg::with_name("long-help")
                .long("help-long")
                .help("Prints *detailed* help information"),
        )
        .arg(Arg::with_name("ribbit").long("ribbit").help("Ego sum"))
}

pub fn long_help() -> String {
    r#"Rana generates all the possible anagrams from a given phrase and
dictionary. Note "given some dictionary." Rana does not have a word list
built in. You must tell it what words it may use in an anagram. I have made
myself such a list out of a list of English words I found on the Internet from
which I delted all the words likely to offend people. By default rana will
look in your home directory for a file called .anagrams-dictionary.txt.

In many cases a simple phrase will have hundreds of thousands or millions of
anagrams, setting aside permutations. The phrase "rotten apple", for example,
with a fairly ordinary dictionary of of 109,217 English words, produces 2695
anagrams. Here are 10:

    pone prattle
    plea portent
    pole pattern
    portent pale
    platter pone
    planter poet
    potent paler
    porn palette
    pron palette
    poler patent

Because so many anagrams are available, you are likely to want to focus your
search. Rana provides several options to facilitate this.

--words-in

This will list all the words in your dictionary composable from some subset of
your phrase. You likely want to add --strict to this, so you only get words that
occur in *some* anagram. The --strict version is slower. If you want to verify
that each word listed has some anagram, you can add --prove, or replace --strict
with --prove. This will cause rana to emit the remaining words in an anagram
using the word in question.

--exclude

Discard from your word list particular words.

--include

Include only those phrases which include particular words.

--limit

Only provide a sample of this many phrases.

--random

Shuffle the search order over partial results while searching for anagrams. This
does not provide a fully random sample of the possible anagrams, since only the
results found at any point are shuffled, not all possible results, but this is
a decent way to look at a sample of anagrams when the phrase you've fed in has
many thousands of results. This is particularly useful when paired with --limit.

Caching and Threads

Rana by default uses as many processing threads as there are cores on your
machine. Generally this is what you want, but if you've got a lot of other
things going on, you can limit the number of available threads to reduce the
load your kernel has to deal with.

Rana also uses a dynamic programming algorithm to reduce the complexity of
finding algorithms for large phrases. This is probably unnecessary for short
phrases, though rana provides no lower limit. For larger phrases, like the
complete alphabet, the cache used by the dynamic programming algorithm may grow
so large that the process crashes. If you turn off the cache rana will use
a constant amount of memory, though it may take considerably longer to find all
anagrams.

Text Normalization

Rana attempts to strip away certain characters from your word list and all
other textual input, so it will treat "c-a-t" and " C A T " the same as "cat".
Here is the actual code that does this:

    pub fn normalize(word: &str) -> String {
        word.trim()
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphabetic())
            .collect::<String>()
    }

I have not tested what this will do for something like ß or Í. You may want to
normalize the text yourself before you give it to rana.

NOTE:

The caching algorithm treats character counts as long base-10 numbers. So, for
example, "cat" might be 111 if "c" is 100s, "a" is 10s, and "t" is 1s. Then
"cad" might be 1110 -- "d" is 1000s, and there is one of them, but there is no
"t", so the 1s place is 0. If you have a phrase, like

    "Dhrtaraashtra uvaaca, dharmakshetre kurukshetre samavetaa yuyutsavah"

which has 15 a's, the "a" count has to be represented as 5 -- 15 mod 10 is 5. In
other words, there isn't space in the a's column of the number for all the a's
in the phrase. It is unlikely that this will ever cause trouble, because for a
particular phrase you are unlikely to be encounter two sets of character counts
during processing that have different counts but the same code. Also, a phrase
this size will consume so much cache space that the process will probably crash
before you encounter this collision.

Another consideration with caching is that this scheme can only accommodate
alphabets up to 38 characters in size.
"#.to_string()
}
