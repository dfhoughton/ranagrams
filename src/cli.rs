extern crate clap;
use self::clap::{App, Arg, ArgMatches};
extern crate num_cpus;
use std::mem;

pub fn parse() -> ArgMatches<'static> {
    App::new("ranagrams")
        .version("0.1")
        .author("David F. Houghton <dfhoughton@gmail.com>")
        .about("Finds anagrams of a phrase")
        .after_help(
            "This is the first line.
This is the second line.
        ",
        )
        .arg(
            Arg::with_name("dictionary")
                    .short("d")
                    .long("dictionary")
                    .value_name("file")
                    .default_value("/Users/houghton/mostly_inoffensive_words.txt") // FIXME
                    .help("a line-delimited list of words usable in anagrams")
                    .takes_value(true),
        )
        .arg(
            Arg::with_name("set")
                .short("w")
                .long("words-in")
                .help("Returns the set of words composable from the letters in the input phrase"),
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
                .conflicts_with("set"),
        )
        .arg(
            Arg::with_name("exclude")
                .short("x")
                .long("exclude")
                .value_name("word")
                .help("exclude this word from anagrams")
                .takes_value(true)
                .multiple(true)
                .number_of_values(1),
        )
        .arg(
            Arg::with_name("threads")
                .short("t")
                .long("threads")
                .takes_value(true)
                .default_value(num_cpus_static_str())
                .value_name("n")
                .help("the number of threads to use during anagram collection"),
        )
        .arg(
            Arg::with_name("limit")
                .short("l")
                .long("limit")
                .takes_value(true)
                .value_name("n")
                .help("only find this many anagrams")
                .conflicts_with("set"),
        )
        .arg(
            Arg::with_name("phrase")
                .value_name("word")
                .multiple(true)
                .required(true)
                .help("the words for which you want an anagram"),
        )
        .arg(
            Arg::with_name("no_cache")
                .short("C")
                .long("no-cache")
                .help("do not cache partial results (this saves memory and costs speed)"),
        )
        .get_matches()
}

fn num_cpus_static_str() -> &'static str {
    let num_cpus_string = num_cpus::get().to_string();
    let num_cpus_static_str = unsafe { mem::transmute::<&str, &'static str>(&num_cpus_string) };
    mem::forget(num_cpus_string);
    num_cpus_static_str
}
