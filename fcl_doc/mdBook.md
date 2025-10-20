## mdBook "Practicing Rust by developing a function call logger"

### Disadvantages Found

#### Feature on for some and off for the other binary crates of the same worksapce

Would be better if a library crate could be compiled with a feature on for some of the 
binary crates of the workspace, and with the feature off for the other binary crates of the same workspace.


# Parsing
The `syn` developers have done an incredible job.

# Traversing a Module
# Traversing an `impl` Block
# Advanced  
## Traversing a Function

A function can have local functions and closures. E.g.
```rs
fn f() {      // Enclosing function `f()`.
              // Local function `g()`
    fn g() {} // definition,
    g();      // invocation.

    Some(5).map(
                    // Closure
        || true     // definition and invocation.
    )

                                // Closure
    let my_closure = || false;  // definition,
    my_closure();               // invocation.
}
```

If we want the local functions and closures to automatically get annotated/instrumented with `#[loggable]` after we annotate the enclosing function `f()`, then the enclosing function `f()` needs to be traveresed.

Local functions (like `g()`) are relatively rare, that is why a mannual annotation for them is an option.  
But closures are a relatively frequent thing, and often they are not that visible in the code.
Annotating them all mannually will hardly make programmer happy, especially considering the fact that all this annotation can be a temporary measure during a short bug chase period. 
Annotation automation can be a thing paying well.

Let's shortly get back to modules and `impl` blocks discussed earlier.  
Inside of a [module](todo) there are about .. kinds of items, and .. of them need to be annotated with `#[loggable]`.  
Inside of an [`impl` block](todo) there are also a just few kinds of items that need the `#[loggable]` attribute.  
But inside of a function there can be dozens of kinds of the nested entities, nearly the whole Rust language grammar. And in most of those nested entities there can be places where a local function or a lambda can be defined<!-- and needs to be `#[loggable]` -->.
To make local functions and closures all automatically `#[loggable]` the code needs to traverse recursively the syntax tree of a function body and annotate all the items of interest <!-- with `#[loggable]` -->.
This task is comparable to traversing the whole language grammar.

This secition talks about lots of work that at the start promised to be tedious. If you are not interested in what's inside then feel free to skip. 
But if you have plans to do some grammar analysis in your own project, you may find this section useful since here 
I share some of my experience and provide the examples of the Rust language grammar traverse for you to reuse.
Analyzing the language constructs can also help you study the language in more details thus becoming a more effective language user (or a new language developer). 
E.g. before implementing this functionality I didn't think whether the following (or similar) language fragments are possible:
```rs
let Some(x) = y else return; // `else` without `if` (https://docs.rs/syn/latest/syn/struct.LocalInit.html).

impl Trait + use<'a, T> // https://docs.rs/syn/latest/syn/struct.PreciseCapture.html

break 'a |x, y| x + y

for<'a, 'b> const static async move |x, y, | -> u32 { f(x - y, |z| z * 8) } // Two closures are defined in one expression and both can be loggable.

for<'a, 'b, 'c> unsafe extern "C" fn(x: u8, i32, y: ... ,) -> 
    [f32; 
     'a: { 
        fn f() {} 
        for .. { 
            f() 
        } 
        || {5}()
     } 
    ]

// TODO: Couple more.
```

I also didn't know that a function or a closure can be defined (and/or called) inside of arguments and generic arguments, parameters and generic parameters, return types, type definitions, etc.  

In case you agree with the proverb "If you want to study a programming language then write a compiler for it", then this section is a step in that direction.

# Unresolved/Known Issues
```rs
MyTrait<T, U>::my_func::<char, u8>() {     // `<T, U>` are not resolved with the actual generic args.
    MyTrait<T, U>::my_func<T, U>()::closure{1,2:4,5} {}
```

# Synchronizing With `panic!()`
Let's imagine again that in our thread function we haved a function `f()` that calls funtion `g()` 10 times in a loop. And during iteration 8 the function `g()` panics (or calls something that panics).
```rs
fn f() {
    for i in 0..10 {
        g(i);
    }
}
fn g(i: i32) {
    if i == 8 {
        panic!("Testing the panic")
    }
}
```

What would be reasonable to see in the function call log? I as a user, running my code instrumented with FCL, would expect to see about the following.
```c
f() {
  g(i: 0) {}
  // g() repeats 7 time(s).
  g(i: 8) {
thread 'T1' panicked at user\src\main.rs:454:17:
T1: Testing the panic
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```
But having the current implementation I would expect that after logging the first-most iteration (`g(i: 0) {}`), the repeated calls to `g()` would be cached, then at iteration 8 the function `g()` would panic, the panic handler
would send the panic report to `stderr`. Since the `stderr` is redirected to a buffer, 
the panic report would be buffered. Since the panic handler is not instrumented with the FCL, the multiple lines sent to the `stderr` by the panic handler would not be interleaved with the calls to 
`maybe_flush()`. I.e. the whole panic report would stay in the buffer waiting for a flush. Then the panic handler would never return the control to the instrumented code (to function `g()`).
And the cached repeaded calls to `g()` would never get a chance to be flushed. I would expect to see a single iteration followed by nothing.
```c
f() {
  g(i: 0) {}
```
Such an output would definitely confuse the user about what happened after `g(i: 0) {}`, making an impression of a hanging thread.

But in practice I saw about the following in a few attempts with slight modifications.
```c
f() {
  g(i: 0) {}
  // g() repeats 7 time(s).
  g(i: 8) {
thread 'T1' panicked at user\src\main.rs:454:17:
T1: Testing the panic
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
  } // g().
} // f().  
```
Such logs confused me a bit for a while. 
* How could panic report end up in the log? 
* How could functions `g()` and `f()` (and potentially other enclosing ones) get a control back 
  from the panic handler all the way up to the most-enclosing thread function?

I have placed some more code (`println!()`) after `panic!()` and saw that the control 
was not returned by the panic handler. And I realized that the output 
```c
  } // g().
} // f().  
```
was logged by the destructors of the correspoinding `FunctionLogger` instances during stack unwinding. And the first-most destructor (printing the fragment `} // g().`) has noticed that 
there was some buffered `stderr` output - the panic report - and has flushed that panic report.

Such fake returns after the panic handler (the returns from `g()`, `f()`, and other enclosing functions all the way back to the 
thread function or to a function called first after logging enabling), will definitely confuse the users. That output needs to be suppressed. But the suppression can prevent the panic report flushing to `stderr`.

In addition to that, the observation above is only applicable to the _unwinding_ runtimes.
The user can potentially instruct the compiler to use the 
[_aborting_ runtime](https://rust-book.cs.brown.edu/ch09-01-unrecoverable-errors-with-panic.html?highlight=unwinding#unwinding-the-stack-or-aborting-in-response-to-a-panic), in which case 
the `FunctionLogger` destructors will not be run, and the panic report will not be flushed.

**Reader Practice.**  
Before proceeding to the next page come up with the solution ;-).  
How to suppress the false function returns logging (like `} // g().`) during the panic handler in the unwinding runtimes,  
and how to see the panic report in the right place (`stderr`) and at the right moment of the FCL log?

<!-- Next page -->
## The `std::panic::{take_hook,set_hook}()`
The solution to the problem consists of 
* [`std::panic::take_hook()`](https://doc.rust-lang.org/std/panic/fn.take_hook.html) and 
* [`std::panic::set_hook()`](https://doc.rust-lang.org/std/panic/fn.set_hook.html)

FCL is to save the previous panic handler with `take_hook()`, and set its own with `set_hook()`.  

The FCL's handler is to
* flush the whole FCL cache and std output buffers,
* detach the current thread's infra from the per-thread list (`HashMap`) of infras; 
  which serves as the "panic" flag for the current thread (in the `CallLoggerArbiter`),
  which will subsequently suppress logging by the `FunctionLogger` destructors during stack unwinding, if any, for the current thread,
* in a single-threaded case cancel the std output buffering,
* transfer the control (within the context of the panicking thread) to the
  saved previous panic handler that will never return the control to the caller,
  but in case of unwinding runtimes will still run the `FunctionLogger` destructors whose output is to be suppressed based on the thread's "panic" flag;  
  that saved previous panic handler will also send the panic report to the `stderr`  
  that in a single-threaded case will be sent directly to the original unbuffered `stderr`,  
  and in multithreaded case will be buffered and then flushed upon the activity of the other threads, unless they all hang ;-).

It would be reasonable to summarize with the follwing words.  
Upon thread panic all of the thread's data are to be detached from the FCL data structures such that the thread's data (including the thread-local ones) can be destroyed during stack unwinding, safely for FCL and without any memory leaks in FCL. So that if an instrumented program endlessly spawns the threads that by design terminate by panicking, the FCL can work endlessly (if the call graph does not grow or gets periodically cleared, if that is implemented).

**Reader Practice.**  
If not yet, implement the solution.

<!-- Next page -->
## The panic in `main()`
The implementation has shown that if the `main()` thread panics while the other threads are still running,
then the other threads continue running during the `main()` thread's panic handler (panic hook).
And some of the threads can come to a normal completion and terminate, destroying their thread-local data, while the panic handler is running for the `main()` thread.  

The FCL's panic handler `CallLoggerArbiter::panic_hook()`, within the context of the `main()` thread, manipulates the FCL's data structures by accessing the `CallLoggerArbiter` instance protected by `Mutex`. The other threads log their function calls (or destroy/unregister their own data) by 
also locking the `Mutex` protecting the `CallLoggerArbiter` instance. In combination with the buffered 
std output the situation sometimes ends up in a freezing, that looks like a deadlock, with no panic report, that does not return the control to the command line, and cannot be interrupted with `<Ctrl+c>`. 
In the debugger the problem either does not show itself, or, when shows, what I see in the halted debugger confuses me: 
* The `main()` thread is running (in a loop?) somewhere in the internals of the saved original panic handler,
invoked from the FCL's panic handler _after dropping the `Mutex` lock_, 
* and another thread is waiting, trying to lock the `Mutex` (totally two threads).

Continuing the execution (or stepping) in the debugger just magically moves forward 
to the expected return to the command line, showing the expected output (plus the stack backtrace in the panic report). 

At the moment of writing I wasn't able to resolve, and even understand, the issue. The suspicion is that, if the other 
thread tries to lock the `Mutex` when the `Mutex` is locked by the panic handler in the context of the `main()` thread, the 
situation is close to the [documentation fragment](https://doc.rust-lang.org/std/sync/struct.Mutex.html#poisoning) "a mutex is considered poisoned whenever a thread panics while holding the mutex".
However the "recover from a poisoned mutex" (on that same documentation page) does not help. 

Feels like such a simultaneous 
access to the `Mutex` by another thread and the panic handler within the `main()` thread, prevents  
* either the panic handler's normal release of the `Mutex`  
* or the other thread's normal acquisition of the `Mutex`.  

But canceling the std output buffering 
in the FCL's panic handler, if the handler is called for the `main()` thread, lowers down the probability of output freezing, at the expence of desynchronized output, concurrent between 
the other thread's function log and the panic report. See the "ggnote" fragment 
in the output below (the `main()` thread output is on the left, and the other thread output is on the right. The std output should be on the left, but in practice it interferes with the log output).

> TODO: Provide a code exmaple and a better output log.

```c
  g(i: 0, ) {}
} // f().
                                  gg(i: 6, ) {
                                    ff() {
// f() repeats 7 time(s).
f(i: 8, ) {
  g(i: 8, ) {
Sample stderr output
                                    } // ff().

thread 'main' panicked at user\src\main.rs:168:17:
main(): Panicking voluntarily
                                  } // ggnote: run with `RUST_BACKTRACE=1` environment variable to display a backtrace().
error: process didn't exit successfully: `target\debug\user.exe` (exit code: 101)
```
The other run of the same binary can show the panic moment more clearly:
```c
f(i: 0, ) {
  g(i: 0, ) {}
} // f().
                                  } // gg().
                                  gg(i: 4, ) {
                                    ff() {
// f() repeats 7 time(s).
f(i: 8, ) {
  g(i: 8, ) {
Sample stderr output
                                    } // ff().

thread 'main' panicked at user\src\main.rs:168:17:
main(): Panicking voluntarily
                                  } // gg().
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
error: process didn't exit successfully: `target\debug\user.exe` (exit code: 101)
```

**Reader Practice.**  
Try to solve the problem or come up with an explanation of it.  
Ideally the output 
* should not freeze (deadlock), 
* should clearly demonstrate what's going on,
* should go 
  * to the right destination (whatever is sent to `stderr` or `stdout` or other, 
    should end up in `stderr` or `stdout` or other correspondingly)
  * and in the right order (whatever is sent earlier, should land earlier), such that 
    if the `stderr` and `stdout` are both forwarded to the same place 
    (screen, or redirected to the same file), the order is preserved.

We can try to discuss your approach in your pull request but I don't promise that.

## Wrapping Up the Output Synchronization

Now I can state that I came up with an output synchronization solution that most readers can live with!!! ;-DDD.

And some of you will _generously_ complement my words: "That's right! The only tiny exception is when my 
instrumented Rust function calls a native function in C or C++, that calls a forest
of functions that pipe a few waterfalls to `stderr` and `stdout` in an arbitrary order ;-)".

And I will _hospitably_ reply: "For that case I have an excellent solution below ;-)..."

**Reader Pracice.**  
You understand what I mean... what should happen to C and C++ code... ;-DDD

## Logging Endlessly
(Unsorted. Requires familiarity with pseudonode, caching, repeat count increment and removal of the repeating node)
In some environments or cases it is hard to generate a core (crash) dump or catch a failure in the debugger. In those occurrences the developer needs to see what was going on shortly before the failure. The FCL can help in such cases.

The situation becomes a bit tricky when the software, such as a daemon or an embedded firmware, is running endlessly, and the failure happens rarely, like once a month.

As for the log storage, the FCL can log the function calls (interleaved with the binary's own output) not only to a terminal but to a file or memory in a circular manner, such that the oldest log entries are overwritten with the new ones. When the failure happens the developer can see the log of the last 7 or 30 days, etc., depending on the settings and available storage.

But what does happen to the dynamic memory? Based on the logic so far the call graph grows endlessly. This will exhaust the memory. How can FCL log endlessly but still retain all the functionality?

Let's consider a simple `main()` function. Let's imagine that it calls `init()`, then in a loop it calls `work()`, then `deinit()`. During logging the `main()` gets added as a child to the pseudonode. Then `init()` is added as a child of `main()`, then the `work()` (with its children) is added after `init()`, the repeated `work()` increments the repeat count of the first `work()` and gets removed from the call graph. The `work()` having children different from the children of the first `work()`, gets added to the call graph, and so on.

At any moment the FCL algorithm analyses the two latest calls to `work()` in order to make a decision whether to leave the latest `work()` in the graph and log it, or to remove it and increment the repeat count of the preceding `work()`.

In that decision-making the `init()` is not needed in the call graph and can potentially be removed (it is already logged). But if we remove it, then the list of children of `main()` will be distorted. This will affect the rare implementations where `main()`, upon certain condition or periodically, can return, and then get called again, and upon return from the second `main()` the FCL needs to compare the second `main()` with the earlier `main()` and either log the second `main()` or remove it and increment the repeat count for the first `main()`. But if the FCL's algorithm removes `init()` (with its repeat count), then the comparison of the two adjacent calls to `main()` will be distorted too.

To summarize, for the common case we cannot remove `init()` from the call graph (when we proceed to hadling the calls to `work()`), because the `init()` particiaptes in comparing the adjacent calls to `main()`.

But what if `main()` is not added to the call graph? What if logging starts after the call to `main()` but before the call to `init()`? In that case the `init()` will be added to the call graph as the first child of the pseudonode. Upon the first call to `work()` the algorighm will see that the `work()` has a name different from `init()`, thus the caching is not about to be triggered, the `work()` will get logged right away without caching, and the `init()` will not be needed any more in the call graph. That is why the `init()` can be removed from the list of children of the pseudonode, and the `work()` can be added as the first child instead.  
(TODO: Not yet implemented. Reader Practice?)

The repeated calls to `work()` will increment the repeat count of the first `work()`.

Any different call to `work()`, the one having children or their repeat counts different from those of the first `work()`, will cause caching stop, the first `work()`'s repeat count flush, removal of the first `work()` from the list of pseudonode's children, and making the different `work()` a fist child of the pseudonode.  
(TODO: Not yet implemented. Reader Practice?)

This can continue endlessly. For that to work the `main()` must not be logged. I.e. either the `main()` must not be instrumented with `#[loggable]`, or the logging must be enabled after the call to `main()`.

_The same is even more applicable to the thread functions_. A thread function is the top-level function that is invoked when a thread is spawned. It is quite typical for the applications to spawn a thread, wait for its termination, spawn again, and so on in the loop endlessly. For an endless logging the thread function should not be logged.

## Thread Switch Distortion
(Unsorted. Requires knowledge of the architecture, in particular the single mutex approach)

The single mutex can significantly distort the thread constext switch picture. 

E.g. the first thread locks the mutex and starts updating the call graph. The thread context switches to the second thread. The second thread makes a call and tries to lock the mutex to log that call, but the mutex is already locked, so the second thread starts waiting for the mutex, returning the execution back to the first thread. Depending on the operating system's approach to the thread synchronization, in some scenarios as soon as the first thread releases the mutex the thread context can immediately be switched to the second thread waiting for the mutex. But in other scenarios the first thread after releasing the mutex can continue the execution until the expiration of the time quantum, can log a few calls more, and again get interrupted while the mutex is locked. The second thread will again have to wait for the mutex. This can happen an unpredictable number of times.

To summarize, the log can show that the thread switch happened later relative to the first thread's log, and with one attempt, but in reality there could be a number of unsuccessful attempts to switch the thread context earlier than what the log shows. The user should be ready for such a distortion of the thread context switch and understand that certain timing-dependent scenarios can be affected by the FCL.

How large is the distortion? Depends on the thread synchrinization approach of the operating system and on the code being logged. If there are lengthy fragments without loops and calls then the distortion is minimal. But the more often the code execution passes through the starts and ends of the loops, functions, and closures the larger is the distortion (and the slow-down because of logging. TODO: explain the slow-down in the Performance Impact chapter).