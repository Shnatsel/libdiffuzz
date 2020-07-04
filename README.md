## libdiffuzz: security-oriented alternative to Memory Sanitizer

This is a drop-in replacement for OS memory allocator that can be used to detect uses of uninitialized memory. It is designed to be used in case [Memory Sanitizer](https://clang.llvm.org/docs/MemorySanitizer.html) is not applicable for some reason, such as:

 * Your code contains inline assembly or links to proprietary libraries that cannot be instrumented by MSAN
 * You want to find vulnerabilities in black-box binaries that you do not have the source code for (not always straightforward, see below)
 * You want to check if the bug MSAN found is actually exploitable, i.e. if the uninitialized memory contents actually show up in the output
 * You're debugging code that is specific to an exotic CPU architecture or operating system where MSAN is not available, such as macOS. If you're on a really obscure platform that doesn't have a Rust compiler, a less robust [C99 implementation](https://github.com/Shnatsel/libdiffuzz-c99) is available.

**This is not a drop-in replacement for Memory Sanitizer!** It will likely require changes to your code or your testing setup, see below.

## How it works

When injected into a process, this library initializes every subsequent allocated region of memory to different values. Using this library you can detect uses of uninitialized memory simply by running a certain operation twice *in the same process* and comparing the outputs; if they differ, then the code uses uninitialized memory somewhere.

Combine this with a fuzzer (e.g. [AFL](http://lcamtuf.coredump.cx/afl/), [honggfuzz](http://honggfuzz.com/)) to automatically discover cases when this happens. This is called "differential fuzzing", hence the name.

Naturally, this is conditional on the same operation run twice returning the same results normally. If that is not the case in your program and you cannot make it deterministic - you're out of luck.

## TL;DR: usage

 1. Clone this repository, run `cargo build --release`; this will build libdiffuzz.so and put it in `target/release`
 1. Make your code run the same operation twice in the same process and compare outputs.
 1. Run your code like this:
    - On Linux/BSD/etc: `LD_PRELOAD=/path/to/libdiffuzz.so /path/to/your/binary`
    - On macOS: `DYLD_INSERT_LIBRARIES=/path/to/libdiffuzz.so DYLD_FORCE_FLAT_NAMESPACE=1 /path/to/your/binary`
    - If you're fuzzing with [AFL](http://lcamtuf.coredump.cx/afl/): `AFL_PRELOAD=/path/to/libdiffuzz.so afl-fuzz ...` regardless of platform. If you're not fuzzing with AFL - you should!
 1. Wait for it to crash
 1. Brag that you've used differential fuzzing to find vulnerabilities in real code

## Quick start for Rust code

**Note:** Memory Sanitizer [now works with Rust](https://doc.rust-lang.org/unstable-book/compiler-flags/sanitizer.html#memorysanitizer). You should probably use it instead of libdiffuzz!

If your code does not contain `unsafe` blocks, you don't need to do a thing! Your code is already secure!

However, if you have read from [the black book](https://doc.rust-lang.org/nomicon/) and invoked the Old Ones...

 1. Clone this repository, run `cargo build --release`; this will build libdiffuzz.so and put it in `target/release`
 1. Make sure [this code](https://gist.github.com/Shnatsel/0c024a51b64c6e0b6c6e66f991904816) doesn't reliably crash when run on its own, but does crash when you run it like this: `LD_PRELOAD=/path/to/libdiffuzz.so target/release/membleed`
 1. If you haven't done regular fuzzing yet - do set up fuzzing with AFL. [It's not that hard.](https://fuzz.rs/book/afl/setup.html)
 1. In your fuzz target run the same operation twice and `assert!` that they produce the same result. See [example fuzz target for lodepng-rust](https://github.com/Shnatsel/lodepng-afl-fuzz-differential) for reference. [A more complicated example](https://github.com/Shnatsel/claxon-differential-fuzzing) is also available.
 1. Add the following to your fuzz harness:
 ```rust
// Use the system allocator so we can substitute it with a custom one via LD_PRELOAD
use std::alloc::System;
#[global_allocator]
static GLOBAL: System = System;
 ```
 6. Run the fuzz target like this: `AFL_PRELOAD=/path/to/libdiffuzz.so cargo afl fuzz ...`

## Auditing black-box binaries

Simply preload libdiffuzz into a binary (see "Usage" above), feed it the same input twice and compare the outputs. If they differ, it has exposes uninitialized memory in the output. 

If your binary only accepts one input and then terminates, set the environment variable `LIBDIFFUZZ_NONDETERMINISTIC`; this will make output differ between runs. Without that variable set libdiffuzz tries to be as deterministic as possible to make its results reproducible.

If the output varies between runs under normal conditions, try forcing the binary to use just one thread and overriding any sources of randomness it has.

## Limitations and future work

Stack-based uninitialized reads are not detected.

Unlike memory sanitizer, this thing will not make your program crash as soon as a read from uninitialized memory occurs. Instead, it lets you detect that it has occurred after the fact and only if the contents of uninitialized memory leak into the output. I.e. this will help you notice security vulnerabilities, but will not really aid in debugging.

## Trophy case

List of previously unknown (i.e. zero-day) vulnerabilities found using this tool, to show that this whole idea is not completely bonkers:

 1. [Memory disclosure](https://github.com/ruuda/claxon/issues/10) in [Claxon](https://github.com/ruuda/claxon)

If you find bugs using libdiffuzz, please open a PR to add it here.

## See also

[Valgrind](http://valgrind.org/), a perfectly serviceable tool to detect reads from uninitialized memory if you're willing to tolerate 20x slowdown and occasional false positives.

[MIRI](https://github.com/rust-lang/miri), an interpreter for Rust code that detects violations of Rust's safety rules. Great for debugging but unsuitable for guided fuzzing.

[libdislocator](https://github.com/mirrorer/afl/tree/master/libdislocator), a substitute for [Address Sanitizer](https://clang.llvm.org/docs/AddressSanitizer.html) that also works with black-box binaries.

For background on how this project came about, see [How I've found vulnerability in a popular Rust crate (and you can too)](https://medium.com/@shnatsel/how-ive-found-vulnerability-in-a-popular-rust-crate-and-you-can-too-3db081a67fb).
