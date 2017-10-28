extern crate clap;
use self::clap::{App, Arg, ArgMatches};

pub fn parse<'a>(cpus: &'a str, dictionary: Option<&'a str>) -> ArgMatches<'a> {
    let mut dictionary_argument = Arg::with_name("dictionary")
        .short("d")
        .long("dictionary")
        .value_name("file")
        .help("a line-delimited list of words usable in anagrams")
        .takes_value(true);
    if let Some(file) = dictionary {
        dictionary_argument = dictionary_argument.default_value(file);
    }
    App::new("ranagrams")
        .version("0.1")
        .author("David F. Houghton <dfhoughton@gmail.com>")
        .about("Finds anagrams of a phrase")
        .after_help(
            "This is the first line.
This is the second line.
        ",
        )
        .arg(dictionary_argument)
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
                .default_value(cpus)
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
        .arg(
            Arg::with_name("random")
                .short("r")
                .long("random")
                .help("(partially) shuffle order of discovery"),
        )
        .get_matches()
}
