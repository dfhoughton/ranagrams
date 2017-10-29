# ranagrams
yet another anagram algorithm, this time in Rust

## Usage

The text provided by `--help`.
```
USAGE:
    ranagrams [FLAGS] [OPTIONS] <word>

FLAGS:
    -h, --help        Prints help information
    -C, --no-cache    do not cache partial results (this saves memory and costs speed)
    -r, --random      (partially) shuffle order of discovery
    -w, --words-in    Returns the set of words composable from the letters in the input phrase
    -V, --version     Prints version information

OPTIONS:
    -d, --dictionary <file>    a line-delimited list of words usable in anagrams [default: /Users/houghton/.anagram-dictionary.txt]
    -x, --exclude <word>...    exclude this word from anagrams
    -i, --include <word>...    include this word in the anagrams
    -l, --limit <n>            only find this many anagrams
    -t, --threads <n>          the number of threads to use during anagram collection [default: 8]

ARGS:
    <word>...    the words for which you want an anagram

Ranagrams generates all the possible anagrams from a given phrase, dictionary,
and text normalization (elimination of non-word characters and conversion of
case). Note "given some dictionary." Ranagrams does not have a word list built
in. You must tell it what words it may use in an anagram. I have made myself
such a list out of a list of English words I found on the Internet from which I
delted all the words likely to offend people. By default ranagrams will look in
your home directory for a file called .anagrams-dictionary.txt.

In many cases this a simple phrase will have hundreds of thousands or millions
of phrases, setting aside permutations. The phrase "rotten apple", for example,
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
search. Ranagrams provides several options to facilitate this.

--words-in

This will list all the words in your dictionary composable from some subset of
your phrase.

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

Ranagrams by default uses as many processing threads as there are cores on your
machine. Generally this is what you want, but if you've got a lot of other
things going on, you can limit the number of available threads to reduce the
load your kernel has to deal with.

Ranagrams also uses a dynamic programming algorithm to reduce the complexity of
finding algorithms for large phrases. This is probably unnecessary for short
phrases, though ranagrams provides no lower limit. For larger phrases, like the
complete alphabet, the cache used by the dynamic programming algorithm may grow
so large that the process crashes. If you turn off the cache ranagrams will use
a constant amount of memory, though it may take considerably longer to find all
anagrams.

Text Normalization

Ranagrams attempst to strip away certain characters from your word list and all
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
normalize the text yourself before you give it to ranagrams.

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
alphabets up to 38 characters in size. You won't hit this limit, of course,
unless you modify the ranagrams
```

I have made many variants of this anagram algorithm. This is the first in Rust.
It is more efficient and faster than any of the previous versions. I cannot say
that this is particularly good or idiomatic Rust. It is the fist significant
bit of Rust I've ever written. To the extent that this is good Rust, the credit
is entirely due to @TurkeyMcMac, who knows Rust much better than I do and could
generally tell me when I was doing something particularly stupid.
