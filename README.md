## Poor Man's Memory Sanitizer

This is a trivial fork of [AFL](http://lcamtuf.coredump.cx/afl/)'s libdislocator that can be used to detect uses of uninitialized memory. It is designed to be used in case [Memory Sanitizer](https://clang.llvm.org/docs/MemorySanitizer.html) is not available for some reason, such as:

 * Your code contains inline assembly or links to proprietary libraries that cannot be instrumented by MSAN
 * You're debugging code that is specific to an exotic CPU architecture where MSAN is not available
 * You want to check Rust code for memory disclosure vulnerabilities and [Rust standard library still doesn't support MSAN](https://github.com/rust-lang/rust/issues/39610)
 * You're debugging code that is specific to a freaky operating system such as macOS where no sane development tools are available
 * You want to find vulnerabilities in black-box binaries that you do not have the source code for (not always straightforward, see below)

**This is not a drop-in replacement for Memory Sanitizer!** It will likely require changes to your code or your testing setup, see below.

## How it works

When injected into a process, this library initializes every subsequent allocated region of memory to different values. Using this library you can detect uses of uninitialized memory simply by running a certain operation twice *in the same process* and comparing the outputs; if they differ, then the code uses uninitialized memory somewhere.

Naturally, this is conditional on the same operation run twice returning the same results normally. If that is not the case in your program, and you cannot make it deterministic, you're out of luck.

This library also inherits all the checks that upstream libdislocator performs; see README.dislocator for details.

## TL;DR: usage

 1. Clone this repository, run `make`; this will build libdislocator.so
 1. Make your code run the same operation twice in the same process and compare outputs.
 1. Run your code like this: `LD_PRELOAD=/path/to/libdislocator.so /path/to/your/binary`.
 1. If you're fuzzing with [AFL](http://lcamtuf.coredump.cx/afl/), use `AFL_PRELOAD=/path/to/libdislocator.so afl-fuzz ...`. If you're not fuzzing with AFL - you should!
 1. Brag that you've used differential fuzzing to find vulnerabilities in real code

## Quick start for Rust users

 1. Clone this repository, run `make`; this will build libdislocator.so
 1. Make sure [this code](https://gist.github.com/Shnatsel/0c024a51b64c6e0b6c6e66f991904816) doesn't reliably crash when run on its own, but does crash when you run it like this: `LD_PRELOAD=/path/to/libdislocator.so target/release/membleed`
 1. If you haven't done regular fuzzing yet - do set up fuzzing with AFL. [It's not that hard.](https://fuzz.rs/book/afl/setup.html) Make sure you use the in-process mode, i.e. the `fuzz!` macro.
 1. In your fuzz target run the same operation twice and `assert!` that they produce the same result. **TODO:** example
 1. Run the [AFL.rs](https://github.com/rust-fuzz/afl.rs) fuzz target like this: `AFL_PRELOAD=/path/to/libdislocator.so cargo afl fuzz ...`

## Auditing black-box binaries

If your target binary lets you feed it the same input several times - stellar! Simply preload libdislocator-numbering into a binary, feed it the same input twice and compare the outputs.

However, if your binary only accepts one input and then terminates, you will have to change the `u16 alloc_clobber_counter = 0;` in libdislocator-numbering to something unique to each process, such as milliseconds from system time, replace `alloc_clobber_counter++` in memset call with `alloc_clobber_counter`, then run the entire process twice and compare the outputs from the two runs. If they differ - congratulations, you've found a memory disclosure vulnerability!

Oh - if the output is inherently non-deterministic, you're out of luck.

## Limitations and future work

Unlike memory sanitizer, this thing will not make your program crash as soon as a read from uninitialized memory occurs. Instead, it lets you detect that it has occurred after the fact and only if the contents of uninitialized memory leak into the output. I.e. this will help you notice security vulnerabilities, but will not really aid in debugging.

Stack-based uninitialized reads are not detected.

I have no idea how the global counter in C inside `malloc()` will behave in multi-threaded programs. FWIW I have just as much idea about what would happen if I made the counter atomic. So for now, be warned that in multi-threaded programs it may or may not actually detect uninitialized memory access. Contributions are welcome. For now you can work around this by applying the same hack as for black-box binaries.

This may miss single-byte uninitialized reads because the counter is `u16`; if you need to detect those, change it to `u8`, but be warned that it will be more likely to miss uninitialized reads that way.

Since this is a fork of libdislocator, which is a poor man's [Address Sanitizer](https://clang.llvm.org/docs/AddressSanitizer.html), this is now poor man's ASAN+MSAN in one package. If you're interested in just the MSAN-like bits (e.g. if you did get the real ASAN running already), decoupling the "initialize buffer to specific values" bit into a separate library and dropping the rest of libdislocator might be worthwhile. For my purposes this all-in-one thing has been fast enough, but your mileage may vary.

Also, if you go down the "just MSAN" road, you might as well RIIR and/or write it as a Rust-specific allocator, now that the relevant traits are stabilized. Rust-specific allocator sure is going to be more ergonomic, but I'd much rather fix MSAN.

Also, I should either upstream this into AFL at some point or, failing that, hand this over to @rust-fuzz folks.

## Trophy case

List of bugs found using this tool, just to show that this whole idea is not completely bonkers:

 1. [Memory disclosure](https://github.com/ruuda/claxon/issues/10) in [Claxon](https://github.com/ruuda/claxon)