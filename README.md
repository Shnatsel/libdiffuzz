## libdiffuzz: poor man's Memory Sanitizer

This is a drop-in replacement for OS memory allocator that can be used to detect uses of uninitialized memory. It is designed to be used in case [Memory Sanitizer](https://clang.llvm.org/docs/MemorySanitizer.html) is not available for some reason, such as:

 * Your code contains inline assembly or links to proprietary libraries that cannot be instrumented by MSAN
 * You're debugging code that is specific to an exotic CPU architecture where MSAN is not available
 * You're debugging code that is specific to a freaky operating system such as macOS where no sane development tools are available
 * You want to check Rust code for memory disclosure vulnerabilities and [Rust standard library still doesn't support MSAN](https://github.com/rust-lang/rust/issues/39610)
 * You want to check if the bug MSAN found is actually exploitable, i.e. if the uninitialized memory contents actually show up in the output
 * You want to find vulnerabilities in black-box binaries that you do not have the source code for (not always straightforward, see below)

**This is not a drop-in replacement for Memory Sanitizer!** It will likely require changes to your code or your testing setup, see below.

## How it works

When injected into a process, this library initializes every subsequent allocated region of memory to different values. Using this library you can detect uses of uninitialized memory simply by running a certain operation twice *in the same process* and comparing the outputs; if they differ, then the code uses uninitialized memory somewhere.

Combine this with a fuzzer (e.g. [AFL](http://lcamtuf.coredump.cx/afl/), [honggfuzz](http://honggfuzz.com/)) to automatically discover cases when this happens. This is called "differential fuzzing", hence the name.

Naturally, this is conditional on the same operation run twice returning the same results normally. If that is not the case in your program and you cannot make it deterministic - you're out of luck.

## TL;DR: usage

 1. Clone this repository, run `make`; this will build libdiffuzz.so
 1. Make your code run the same operation twice in the same process and compare outputs.
 1. Run your code like this: `LD_PRELOAD=/path/to/libdiffuzz.so /path/to/your/binary`.
 1. If you're fuzzing with [AFL](http://lcamtuf.coredump.cx/afl/), use `AFL_PRELOAD=/path/to/libdiffuzz.so afl-fuzz ...` instead. If you're not fuzzing with AFL - you should!
 1. Brag that you've used differential fuzzing to find vulnerabilities in real code

## Quick start for Rust code

 1. Clone this repository, run `make`; this will build libdiffuzz.so
 1. Make sure [this code](https://gist.github.com/Shnatsel/0c024a51b64c6e0b6c6e66f991904816) doesn't reliably crash when run on its own, but does crash when you run it like this: `LD_PRELOAD=/path/to/libdiffuzz.so target/release/membleed`
 1. If you haven't done regular fuzzing yet - do set up fuzzing with AFL. [It's not that hard.](https://fuzz.rs/book/afl/setup.html)
 1. In your fuzz target run the same operation twice and `assert!` that they produce the same result. See [example code for Claxon](https://github.com/Shnatsel/claxon-differential-fuzzing) for reference.
 1. Add the following to your fuzz harness:
 ```rust
// Use the system allocator so we can substitute it with a custom one via LD_PRELOAD
use std::alloc::System;
#[global_allocator]
static GLOBAL: System = System;
 ```
 6. Run the fuzz target like this: `AFL_PRELOAD=/path/to/libdiffuzz.so cargo afl fuzz ...`

## Auditing black-box binaries

If your target binary lets you feed it the same input several times - stellar! Simply preload libdiffuzz-numbering into a binary, feed it the same input twice and compare the outputs.

However, if your binary only accepts one input and then terminates, you will have to change the `u16 alloc_clobber_counter = 0;` in libdiffuzz-numbering to something unique to each process, such as milliseconds from system time, replace `alloc_clobber_counter++` in memset call with `alloc_clobber_counter`, then run the entire process twice and compare the outputs from the two runs. If they differ - congratulations, you've found a memory disclosure vulnerability!

Oh - if the output is inherently non-deterministic, you're out of luck.

## Limitations and future work

Stack-based uninitialized reads are not detected.

Unlike memory sanitizer, this thing will not make your program crash as soon as a read from uninitialized memory occurs. Instead, it lets you detect that it has occurred after the fact and only if the contents of uninitialized memory leak into the output. I.e. this will help you notice security vulnerabilities, but will not really aid in debugging.

I have no idea how the global counter in C inside `malloc()` will behave in multi-threaded programs. FWIW I have just as much idea about what would happen if I made the counter atomic. So for now, be warned that in multi-threaded programs it may or may not actually detect uninitialized memory access. Contributions are welcome. For now you can work around this by applying the same hack as for black-box binaries.

This may miss single-byte uninitialized reads because the counter is `u16`; if you need to detect those, change it to `u8`, but be warned that it will be more likely to miss uninitialized reads that way.

## Trophy case

List of previously unknown (i.e. zero-day) vulnerabilities found using this tool, to show that this whole idea is not completely bonkers:

 1. [Memory disclosure](https://github.com/ruuda/claxon/issues/10) in [Claxon](https://github.com/ruuda/claxon)

## See also

[libdislocator](https://github.com/mirrorer/afl/tree/master/libdislocator), poor man's [Address Sanitizer](https://clang.llvm.org/docs/AddressSanitizer.html) that also works with black-box binaries. libdiffuzz is based on libdislocator code.
