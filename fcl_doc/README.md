# fcl
Rust Function Call Logger

# Why
Learning material to practice. I perceive it as a test covering the material that the young minds grasp in one semester.  
To get a record in my resume since (at the moment of writing) I'm unemployed, I live at the expense of my savings, 
and soon I will need to apply for jobs (after I study Rust to a level sufficient to pass (or bypass) an interview;-).

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
And the cached repeaded calls to `g()` would never get a chance to be flushed. I would expect to see a single iteration followed by nothing
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
  from the panic handler all the way back to the most-enclosing thread function?

I have placed some more code (`println!()`) after `panic!()` and saw that the control 
was not returned by the panic handler. And I realized that the output 
```c
  } // g().
} // f().  
```
was logged by the destructors of the correspoinding instances of the `FunctionLogger` 
during stack unwinding. And the first-most destructor (printing `} // g().`) has noticed that 
there was some bufferd `stderr` output - the panic report - and has flushed that panic report.

Such an output of returns after the panic handler (the returns from `g()`, `f()`, and other enclosing functions all the way back to the 
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
* transfer the control (of the current thread) to the saved previous panic handler 
  that never returns the control to the current thread,
  but in case of an unwinding runtime still runs the `FunctionLogger` destructors whose output is to be suppressed based on the thread's "panic" flag;  
  that panic handler will also send the panic report to the `stderr`  
  that in a single-threaded case will be sent directly to the original unbuffered `stderr`,  
  and in multithreaded case will be buffered and then flushed upon the activity of the other threads, unless they all hang ;-).

It would be reasonable to summarize with the follwing words.  
Upon thread panic all of the thread's data are to be detached from the FCL data structures such that the thread's data (including the thread-local ones) can be destroyed during stack unwinding, safely for FCL and without any memory leaks in FCL. So that if an instrumented program endlessly spawns the threads that by design terminate by panicking, the FCL can work endlessly (if the call graph does not grow or gets periodically cleared, if that is implemented).

**Reader Practice.**  
If not yet, implement the solution.