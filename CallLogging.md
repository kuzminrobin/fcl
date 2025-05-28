# TODO:
* Multithreaded implementation.  
  Latest:  
  * Writer.  
    Commit: thread_arbiter  
    * In a single-threaded case the decorator gets `Option<Box<dyn Write>>`. If `None` then uses `stdout()`.
      Otherwise uses `Box<dyn Write>` (file, socket, etc.) directly.
    * In a multithreaded case 
      * the decorator gets a ThreadSharedWriterAdapter (as an `Option<Box<dyn Write>>`) that 
        * has Arc<[RefCell<]ThreadSharedWriter[>]>, non mutex-protected, since mutex-protection happens earlier,
        * and forwards the calls from adapter to ThreadSharedWriter.  
      * There is a global, one for all the threads, `ThreadSharedWriter` that gets `Option<Box<dyn Write>>` (if `None` then uses stdout()). <!-- TODO: Consider WriterAccessThreadAribiter (Aribter of the access to the writer by different threads) -->
        * It is accessible through Arc<[RefCell<]ThreadSharedWriter[>]>, 
        * It is NOT mutex-protected, since mutex-protection happens earlier.
  * Flush need detection.  
    TODO: Rename `CallLogger` to `FunctionLogger`.  
    The FunctionLogger/ClosureLogger has a thread-local pointer to `CallLogger` trait
    `Box<dyn CallLogger>`. <!-- TODO: Consider FlushableCallLogger { log_call(), log_ret(), flush() } -->. 
    * Single-threaded.  
      Behind the `CallLogger` trait there is a thread-local infra. The `flush()` is never called (has a default impl that does nothing).
    * Multithreaded.  
      Behind the `CallLogger` trait there is a thread-local per-thread `CallLoggerAdapter`
      that has `Arc<>` to a global single-for-all-threads `Mutex<dyn CallLogger>` behind which (`dyn CallLogger`) there is a global single-for-all-threads `CallLoggerArbiter`.  
      The `CallLoggerArbiter` 
      * has a `HashMap<thead_id, Ptr<dyn CallLogger>>` that is filled in upon `register_call_logger()`.  
        Upon thread creation some thread-local instance (infra?) in its constructor calls the `register_call_logger()` (if `CallLoggerArbiter` is not `None`)
        and in its Dtor calls `unregister_call_logger()` (`CallLoggerArbiter` is memorized or not `None`).
      * It also has `last_write_thread`.  
        If the thread context has switched (`last_write_thread` != `thread::current().id()`)
        then the `CallLoggerArbiter` invokes the `flush()` for the `last_write_thread` (`HashMap[last_write_thread].flush()`)
        after which it transfers the calls to the new thread's call logger `HashMap[thread::current().id()].call..()`.

  Outdated:  
  For multithreaded cases, 
    * Single Writer (ThreadSharedWriter, impl Write for ThreadSharedWriter) for all decorators (Arc<[RefCell<]..[>]>), non mutex-protected, since mutex-protection happens earlier. Uses stdout() by default.
    * If decorators get None instead of Some(Arc<[RefCell<]ThreadSharedWriter[>]>) then the decorators use stdout().
  * Single mutex protected thread arbiter (Arc<Mutex<ThreadArbiter>>)
  * Per-thread thread agent, having a clone of Arc<Mutex<ThreadArbiter>>.

  Commit.LifetimeDeadend  
  * repeat_count{ overall, flushed } 
  * flush \n in CodeLikeDecorator if line_end_pending
  * Protect with a new mutex the threading invariants in CallGraph. Such that the flushing does not happen in the middle of `traverse_tree()` for example.
  <!-- `ThreadAwareWriter: last_output_thread_id and flushables -> last_flushable: Option<*mut dyn Flushable>`
  `register_flushable` -->
  ---
  ```
  Mutex-protected writer. Has {
    writer: <Common trait for stdout, stderr, file, socket, pipe>
    previous_thread_id: Option<ThreadID>,
    decorator_instance_by_thread_id: Map/Dictionary of thread_id to &instance
    output_is_being_flushed: bool
  }
  All the threads do (smth like) mutex.lock().write({thread_id, log_output: &str}).
  That write() then does the following:
  if previous_thread_id.is_some() 
      && thread_id != previous_thread_id.unwrap() { // Thread context has switched from previous_thread_id to thread_id
    // Flush the cache of the previous_thread_id
    decorator_instance_by_thread_id(previous_thread_id).flush_cache();
    previous_thread_id = thread_id;
  }
  // Output the log_output.
  ```


  Earlier:
  See a [note](https://docs.rs/proc-macro2/latest/proc_macro2/#thread-safety) (proc_macro2: Most types in this crate are `!Sync` because the underlying compiler types make use of thread-local memory).  
  Consider a similar note for "fcl" if applicable.  
  * Write the Mutithreaded code.
  * Try `#[loggable]`.
  * Design what should be.
    * Mutexed Writer.
  Have in mind:
  * Thread indent prefix.
  * [Thread log color]
  * Probably single-threaded (faster) and multi-threaded fcl.
* ---
* (User practice?) Enable logging globally for everything.  
  Gloobal `#![loggable]`. Log all. Also:  
  `#[loggable] impl ..`
  * Example:  
    ```rust
    #[loggable]
    impl MyStruct {
        // The `#[loggable]` above adds here `#[loggable(MyStruct::new)]`
        // unless `#[nonloggable]` is specified.
        pub fn new() -> Self {
            Self
        }
    }
    ```
  * Overall for `impl`.  
    By defualt none of the associated functions is loggable (log none).  
    `#[loggable] impl ..`: 100% of associated functions are loggable (log all).  
    Manual `#[loggable] fn ..`: for <=50% loggable (log some, "white list").  
    `#[loggable] impl ..`, manual `#[nonloggable] fn`: for >50% loggable (log all except some, "black list").
* Logging the parameters and return values.
* Test
  * Testing
    * Log to string and compare.
    * Basics (from user/main.rs).
    * Output 
    * enable/disable.
  * Test the logging by logging oneself.  
    Or try logging oneself and see how it works.
    Preliminary result: Causes sophisticated circular dependencies.  
    Possible workaround: Create a copy with different name and use the copy to log the original.  
    Also: Test with the existing projects.
    * Update the instructions, how to enable func call logging in your project.
* Overall clean-up.
  * Refactor long functions.
  * Move privates down, publics up (in file).
* ---
* User practice: HTML-decorator (code-like, tree-like), XML-decorator.
* Consider removing all the occurrences of `unwrap()`.
* {Reader Practice: ?} Logging the async funcs.
* Document the `NOTE: Curious trick`.  
  What's the diff between `Rc::clone(&rc)` and `rc.clone()`? The latter works when casting `Rc<dyn SuperTrait>` to `Rc<dyn Trait>`?  
  ```rs
  trait MyTraitA {}
  trait MyTraitB {}
  trait SuperTrait: MyTraitA + MyTraitB {}
  struct S;
  impl SuperTrait for S;
  let sup: Rc<dyn SuperTrait> = Rc::new(S::new());
  let rca: Rc<dyn MyTraitA> = sup.clone(); // `Rc::clone(&sup)` fails.
  let rcb: Rc<dyn MyTraitB> = sup.clone(); // `Rc::clone(&sup)` fails.
  ```
* Optional. User practice, change to:
  ```
  | +-g (repeats 29 time(s).)
  | | +-f

    f() {} // f() repeats 9 time(s).
    g() { // g() repeats 29 time(s).
      f() {}
    } // g().
  ```
* [Graph clearing]
* Output outpaces the cached logging.
* `#[loggable(<MyStruct as MyPureTrait>::pure_method)]` is the same as  
  `#[loggable(MyPureTrait::pure_method)]`.  
  Undesirable.
* Move the thread_local use deeper into the call. Such that a {Call|Closure}Logger is created and that's all.
* `rc.cone()` -> `Rc::clone(&rc)`
* Stricter Terminology where possible.
  ```C++
  parent() { // caller of siblings (enclosing for siblings)
    sibling() {  // caller of children (enclosing for children), callee of parent (nested for parent)
        child() {} // callee of sibling (nested for sibling)
        child() {}
    }
    sibling() {}
  }
  ```
* Macro
  * [Decl Macro](https://veykril.github.io/tlborm/decl-macros/building-blocks/parsing.html#function)
  * Attr Proc-Macro.
* `generic_func < T, U >() {}` remove spaces.
* `MyStruct :: new() {}` remove spaces.
* toolchain stable
* Rename according to Rust (from C++-like). E.g. `Decorator` -> `Decorate`
* Documenting
  * .md
  * Book 
* Video
  * YT
  * SRUG talk.
* [Peeking](https://docs.rs/syn/latest/syn/parse/struct.ParseBuffer.html#method.peek) ([details](https://docs.rs/syn/latest/syn/token/index.html#other-operations)).
* [Report Compiler Error in proc macros]


# Prerequisites
This project has been developped based on the following knowledge.
* TRPL
  * [20.5. Macros](https://doc.rust-lang.org/book/ch20-05-macros.html)
* Learning Rust with Too Many Linked Lists
* [The Little Book of Rust Macros](https://veykril.github.io/tlborm/proc-macros/methodical.html)
  * proc_macro2
  * syn
    * [syn documentation for DeriveInput](https://docs.rs/syn/2.0/syn/struct.DeriveInput.html) (referred by TRPL/"20.5. Macros")
  * quote
    * [the quote crate’s docs](https://docs.rs/quote) (referred by TRPL/"20.5. Macros")

May be useful
* quote
  * Example - https://serde.rs/ (referred by https://docs.rs/quote/latest/quote/#example)
  * [quote_spanned!](https://docs.rs/quote/latest/quote/macro.quote_spanned.html) (referred by https://docs.rs/quote/latest/quote/#example).
  * [prettyplease](https://github.com/dtolnay/prettyplease) (referred by https://docs.rs/quote/latest/quote/#non-macro-code-generators)
* proc_macro
  * [impl FromStr for TokenStream](https://doc.rust-lang.org/proc_macro/struct.TokenStream.html#impl-FromStr-for-TokenStream)



# Making Functions Loggable
Functions can be marked as loggable with the [`proc_macro_attribute` macro](https://veykril.github.io/tlborm/proc-macros/methodical/attr.html).

Further function instrumenting can be done with the [Third-Party Crates](https://veykril.github.io/tlborm/proc-macros/third-party-crates.html).

TODO:


# Call Graph
Theoretically the call graph shoudl be a tree, where the root is `main()`.
But if the logging is enabled after the `main()` started,
then the subsequent function calls do not form a tree but a sequence of subtrees of `main()`.
E.g., actual calls:
```c++
main() {
    // Call logging gets enabled here.
    f() {
        g() {}
    }
    h() {}
}
```
The information the call graph collects (after the logging gets enabled):
```c++
f() {
    g() {}
}
h() {}
```
This infomation is not a tree but is a sequence of subtrees `[f, h]`.
To unify the functionality, the code turns the call graph to a tree
by initializing the graph with a pseudo-node that will serve as a call tree root,
and the subsequent calls `f`, `h`, and on will be added as nested calls (children) of that pseudo-node.
The actual information stored in the call graph will be
```c++
pseudo_node {
    f() {
        g() {}
    }
    h() {}
}
```
The pseudo-node will stay at the bottom of the call stack until the call graph destruction.

# Call Stack
The call stack is intended 
* for an efficient return to the parent of the current node in the call tree
* while keeping the call tree singly linked (the links point from the root towards the leaves, but not vice versa).

The following diagrams provide an example.
```c++
// Actual calls
main() {
    // Call logging gets enabled here.
    f() {
        // TODO: Document the log if logging gets enabled here ("g() {} <Return from f() is ignored> h() { i() {} }").
        g() {}
    }
    h() {
        i() {}
    }
}
```
```
Moment | The information after that moment, contained
       |---------------------------------------------------------------
       | in the call graph   | in the call stack (p is the pseudo-node)
========================================================================
     0 | f() {               | [p, f]
     1 |     g() {           | [p, f, g]
     2 |          }          | [p, f]
     3 | }                   | [p]
     4 | h() {               | [p, h]
     5 |     i() {           | [p, h, i]
     6 |          }          | [p, i]
     7 | }                   | [p]
```

# Call Caching
TODO: What caching is.

## Caching Start Detection
Caching start is detected upon call only, when the name of the current call (name of a new child of the current parent)
repeats the name of the previous call (name of the previous child of the current parent). In this case the previous call
becomes the model for caching (`caching_model`) and the current call, including its nested calls, 
starts being cached instead of being logged. E.g.
```c++
// Data
parent() {
    ...
    child() {}                                  // 2. Becomes the caching model (caching_model = Some(&<this child>)). 
    child() { // 1. The call being handled.     // 3. Start caching this child (and nested calls) instead of logging.
}
```
Another example.
```c++
// Data
parent() {
    ...
    child() {                                   // 2. Becomes the caching model (caching_model = Some(&<this child>)). 
        ...
        nested_call() {
            ...
            more_nested_call() {}
            ...
        }
        ...
    }
    child() { // 1. The call being handled.     // 3. Start caching this child (and nested calls) instead of logging.
}
```

If caching start is not detected (no previous child, or 
the called function's name differes from the previous child's name) 
then, if the previous child is present and its repeat count is non-zero, flush the previous child's repeat count.
E.g.
```c++
// Data
parent() {
    ...
    previous_child() { /* Optional nested calls */ }
    // previous_child repeats 99 time(s).           // 2. Log this.
    new_child() {   // 1. The call being handled.   // 3. Log this.
}
```
## Caching Continuation
If during caching another caching start is detected among the nested calls then caching continues.
The `caching_model` stays the same.
That is why during caching it makes no sense to try to detect the caching start.
```rust
// Rust Code
if !caching_model.is_some() { // If cahcing is not active,
    // try to detect the caching start.
}
```

## Caching End Detection
Caching end can only be detected upon return.
```
(Handling the Return)

If caching is not active {
    Log the repeat count, if non-zero, of the latest child, if present.
    Log the return (of the current node that is a parent of that child).
}
```
E.g.
```c++
    last_child() { /* Optional nested calls */ }
    // last_child() repeats 99 time(s).                 // 2. Log this.
} // parent()       // 1. The return being handled.     // 3. Log this.
```
```
Otherwise (caching is active) {
    If there exists a previous sibling of the returning function, then {
        The call subtree of the returning function is compared recursively
        to the previous sibling call subtree.
```
E.g. 
```c++
    child() {                           // 3.
        // Optional nested calls.
    }     
    // Optionally repeats.
    child() {                           // 2.
        // Optional, potentially different, nested calls.
    } // 1. The return being handled.   // 4. The subtrees 2 and 3 are compared recursively.
```
```        
        If equal {
            the previous sibling's (3) repeat count is incremented
            and the currently returning function's call subtree (2) is removed from the call graph.
            If the previous sibling (3) is the caching_model then caching is over, 
            i.e. the `caching_model` becomes `None` (Side Thought: what if we continue caching and set a cahcing flag to false?).
        }
        Otherwise (subtrees 2 and 3 differ) {
            // Caching is active, previous sibling (3) exists but differs (and potentially has the non-zero repeat count).
            // If the difference between 2 and 3 was by name 
            // then the caching start would not be detected after 3 starting with 2
            // (the previous sibling would not be the `caching_model`).
            // But the caching is active, which means that 
            // * either the caching start has been detected 
            //   at the level of a parent (of the returning function) or above, in which case we continue caching,
            // * or the difference between 2 and 3 is by nested calls, in which case the the previous sibling
            //   can be the `cahing_model`, in which case we detect the caching end.
            If the previous sibling (3) is the cahing_model then {
                Log the previous sibling's (3's) repeat count, if non-zero,
                Log the subtree of the returning function (2).
                Stop caching. `caching_model = None`
            }
            otherwise (caching has been detected at a parent level or above) {
                // Do nothing, continue caching.
            }
        } // Otherwise (subtrees 2 and 3 differ)
    } // If there exists a previous sibling (3) of the returning function (2).
    Otherwise (the returing func 2 is the only child) {
        // Continue caching, do nothing. Caching end cannot be detected upon return from the only child.
    }
} // Otherwise (caching is active)
```
# Practice For a Reader
(In the end of sections)
Write you own implementation of what has been described above and compare to my implementation below.

## Log like this instead
See the left column below.
```
Log                             Explanation
-----------------------------------------------
f                               // f() {
+-g                             //   g() {}
+-h                             //   h() {}
| h repeats 99 time(s)          //   // h() repeats 99 time(s).
+-i                             //   i() {
| +-j                           //     j() {}
| | j repeats 9 time(s)         //     // j() repeats 9 time(s).
| +-k                           //     k() {}
|   k repeats 5 time(s)         //     // k() repeats 5 time(s).
                                //   } // i()
| i repeats 100 time(s)         //   // i() repeats 100 time(s).
+-L                             //   L() {}
  L repeats 1 time(s)           //   // L() repeats 1 time(s)
                                // } // f()
```

### Multithreading Case
```
Thread main()                   Thread T1                       Notes
----------------------------------------------------------------------------------

f
+-g
+-h
| h repeats 30 time(s)                                          Switch to T1 happens between the calls to h().
                                m
                                +-n
                                | +-o
                                |   o repeats 3 time(s)
                                |   o {                         Switch to main() happens in the middle of o().
| h repeats 48 time(s)
| h {                                                           In the middle of h().
                                |   } // o                      Back to the middle of o().
                                |   o repeats 5 time(s)
| } h                                                           Back to the middle of h().
| h repeats 20 time(s)
+-i {                                                           In the middle of i
                                |   o                           A single call to o().
                                +-p
                                | p repeats 80 time(s)
                                +-p                             Differs by nested calls from the previous calls to p().
                                | +-q
                                | +-r
                                | p repeats 7 time(s)           p() with nested calls repeats 7 times.
                                +-p                             p() without nested calls.
                                | p repeats 9 time(s)
                                | p
                                | +-q                           Here logging gets disabled for T1 in the middle of q().
                                      {                         In the middle of q back to the middle of i.
| +-j
| | j repeats 9 time(s)
| +-k
|   k repeats 5 time(s)
| i repeats 100 time(s)
+-L
  L repeats 1 time(s)
  L                                                             Logging gets disabled for main() in the middle of L().
```

## Consider Using `Box` instead of `Rc`
Consider Using `Box<RefCell<Node>>` instead of `Rc<RefCell<Node>>` for the 
* call graph nodes 
* and the pointer to the pseudo-node (`root`).

Everywhere else consider using refs (`&`). In particular, for the `CallGraph`'s
* `call_stack` (`Vec<&?>`),
* `current` (`&RefCell<Node>`),
* `caching_model` (`Option<&Node>`).

## Consider Logging the Parameters and Returned Values
E.g.
```c++
main() {
    f(a: 1, b: false) {
        g(c: "OK", d: [1, 2]) {} // g() returned Some(5).
    } // f() returned true.
    h(x: 1, y: 8) {} // h() returned MyEnum::MyValue.
}
```
Hint: As a result of the proc macro expansion 
* the instrumented function logs the call and params,
* invokes the body as an expression and saves the result in the local varaible,
* logs that result,
* returns that result.

E.g.
The original (instrumented) code:
```rust
#[loggable(params, ret)]
fn f(x: i32, y: i32, flag: bool) -> usize {
    if g(x, y) > 5 && flag {
        8
    } else {
        0
    }
}
```
The result of the macro expansion:
```rust
fn f(x: i32, y: i32, flag: bool) -> usize {
    log_call!(x, y, flag); // Macro. Logs the call and params: "f(x: 1, y: -3, flag: false) {".
    let ret_val = {
        // Fucntion body.
        if g(x, y) > 5 && flag {
            8
        } else {
            0
        }
    }
    log_ret_and_val!(ret_val); // Macro. Logs the return and the returned value: "} // f() returned 8."
    ret_val
}
```

# Known Issues
* Output outpaces the cached log
* Logging attempt for `const` functions results in a compile error:  
  `cannot call non-const method ``..::with`` in constant functions`
* Currently the function names in `impl` blocks are not prefixed with `MyStruct::`.

# What is Missing or Wrong in Rust

## `rust-analyzer`: Macro Expansion Fails for Closures
`rust-analyzer` (`<Ctrl+Shift+p>`, "rust-analyzer: Expand macro recursively at caret"):  
  Macro expansion for an instrumented closure shows nothing.
  ```rs
  #[loggable] // Macro expansion shows nothing.
  |x| !x      // Macro expansion shows nothing.
  ```
  But for an unstrumented function
  ```rs
  #[loggable]
  fn f() {}
  ```
  shows the macro expansion
  ```rs
  // Recursive expansion of loggable macro
  // ======================================
  
  fn f() {
      use fcl::call_log_infra::CALL_LOG_INFRA;
      let mut _l = None;
      CALL_LOG_INFRA.with(|infra| {
          if infra.borrow_mut().is_on() {
              _l = Some(CallLogger::new("f"))
          }
      });
      {}
  }  
  ```
  ### Solution
  ```ps
  cargo install cargo-expand
  # Nightly toolchain is default.
  cargo exapnd --bin consumer_bin
  ```


## `quote::quote!()`: Interpolation using the Struct Fileds is not Implemented
  ```rust
  .. = quote!{
    #mystruct.my_field // This works unexpectedly.
  }
  ```
  (the whole `#mystruct` structure is interpolated, and then `.my_field` is added verbatim)  
  The source of `quote!()` makes an impression that only a single identifier is expected after the `#`.

  # Dead Ends

  ## Using `-Zunpretty=expanded` to See the Macro Expansion
  (See solution at the end of the section)  
  [This section](https://veykril.github.io/tlborm/syntax-extensions/debugging.html#debugging)
  recommends `-Zunpretty=expanded` flag of the Rust compiler `rustc` to see the result of the macro exansion.  
  (A number of smaller dead ends aside) I ended up in the following in the PowerShell prompt of my VSCode:
  ```ps
  $env:RUSTFLAGS="-Zunpretty=expanded"
  pushd consumer_bin
  # Nightly toolchain is active/defualt.
  cargo rustc --bin consumer_bin
  ```
  This caused the following error that I failed to work around in couple of hours of reasonable time.
  ```
  error: failed to run custom build command for proc-macro2 v1.0.95
  Caused by:
  could not execute process fcl_workspace\target\debug\build\proc-macro2-f6d5a347646a87e0\build-script-build (never executed)

  Caused by:
    The system cannot find the file specified. (os error 2)
  warning: build failed, waiting for other jobs to finish...
  ```
  The reason could be the following.  
  The `proc_macro` crates are built first (to a dynamic library or a separate binary linked/invoked by the compiler at the subsequent compilation stages). The `proc_macro` crates define the procedural macros that the compiler uses during the subsequent actual compilation of the main code. In particular, during the preprocessing of the main code the compiler takes the input of procedural macro invocations, feeds that input (in the form of the `TokenStream`) to the procedural macros, gets the macro output (again `TokenStream`), inserts that output to the translation unit and passes the eventual translation unit (after the macro expansion) to the compilation stage.  
  Having the `RUSTFLAGS="-Zunpretty=expanded"` environment variable (or a `-Zunpretty=expanded` flag) the Rust compiler `rustc` does a macro expansion instead of compilation. As a result the `proc_macro` crates fail to build before the compilation of the main code. And the `proc_macro` crates binaries (or dynamic libraries) are not found by the compilation scripts while compiling the main code.

  I got a conclusion that the `-Zunpretty=expanded` is not the right tool when workgin with the procedural macros. Moreover...  
  **If you forget to remove the env var `RUSTFLAGS="-Zunpretty=expanded"` from your environent, then the subsequent cargo commands can fail with unclear reasons**. In particular `cargo install cargo-expand` will fail.

  ### Solution
  ```ps
  cargo install cargo-expand
  # Nightly toolchain is default.
  cargo exapnd --bin consumer_bin
  ```

# Scrap-Heap. Preserved for Learning Purposes and My Own Reference
## Trailing Comma After a Closure
**Has Been Fixed** (by using `ExprClosureComma`).  
  [`syn::ExprClosure`](https://docs.rs/syn/latest/syn/struct.ExprClosure.html). If a closure is passed as the last argument to a function then the VSCode's Rust Code Formatter (`<Alt+Shift+f>`) adds an optional trailing comma after a closure (Rust grammar permits the trailing comma in the list of function arguments (and [function parameters](https://doc.rust-lang.org/reference/items/functions.html?highlight=parameter#functions) in the function definition)), which causes a confusing compilation error (for an instrumented closure): `expected expression, found keyword 'fn'`.
  ```rs
  None.map(     // Closure is the last argument of `map()`.
    #[loggable] // 2. Error: expected expression, found keyword 'fn'
    |x| !x      // 1. <Alt+Shift+f> adds comma after `!x`.
  )
  ```
  The error likely happens becuase the parser for `ExprClosure` does not consume the optional trailing comma from the `TokenStream`.
  Temporary workaround - `#[rustfmt::skip]` ([details](https://doc.rust-lang.org/reference/attributes.html#tool-attributes)) - for an item the closure belongs to:
  ```rs
  #[rustfmt::skip]
  None.map(
    #[loggable]
    |x| !x      // <Alt+Shift+f> does NOT add comma after `!x`.
  )
  ```
  Placing `#[rustfmt::skip]` immediately after `#[loggable]` causes the same confusing compilation error despite the fact that `syn::ExprClosure` (looks like) covers the attributes.
  ```rs
  None.map(
    #[loggable] // Error: expected expression, found keyword 'fn'
    #[rustfmt::skip]
    |x| !x
  )
  ```
  The result of macro expansion (`cargo expand --bin consumer_bin`) is 
  ```rs
  None.map((/*ERROR*/))
  ```
  Generating `#[rustfmt::skip]` as a result of a macro expansion does not help since `<Alt+Shift+f>` adds comma at edit time, before the compile time (before the result of the macro exapnsion is known).

  After the final fix placing the `#[rustfmt::skip]` immediately before or after `#[loggable]` causes compilation error `expected '|'`.
  ```rs
  #[rustfmt::skip]  // The closest right place for `#[rustfmt::skip]` (or at an enclosing item).
  None.map(
    #[loggable]
    |x| !x      // <Alt+Shift+f> does NOT add comma after `!x`.
  )
  ```
  After the fix the `#[rustfmt::skip]` is not needed. The optional trailing comma is consumed normally (at least if the closure is the last argument of a function ;-).


## Done
* Closure. For a more qualified closure name `"f()::closure<..>())"` the enclosing function's name can be taken from the call stack.
* Closure name  
  Consider naming: for a closure inside of function `f()`: `"f()::closure_at_<start line number>[_<end line number>]"`, where `<end line number>` is used for multi-line closures. Or `"f()::closure{<start line number>:<column number>..<end line number>:<column number>}"`. 
* Functions
* Generic Functions (template functions).  
  Naming: `"generic<T, U>"`  
  (currently, at the prerocessing stage, unknown is the way to substitute the generic parameters (`T`, `U`) with the actual generic arguments (like `u32`, `bool`) to get the multiple concretizations of a generic function like `generic<u32, bool>`, `generic<String, usize>`).
* Closures (lambdas).  
  Where and how to get the line and column numbers? Should be something related to spans. See 
  * proc_macro2/Span/{[start()](https://docs.rs/proc-macro2/latest/proc_macro2/struct.Span.html#method.start), end()}. Can help: `[local_]file()` (source file path).
  * [proc_macro/Structs](https://doc.rust-lang.org/proc_macro/#structs)/{[Span](https://doc.rust-lang.org/proc_macro/struct.Span.html), [SourceFile](https://doc.rust-lang.org/proc_macro/struct.SourceFile.html)},
  Consider naming: for a closure inside of function `f()`: `"f()::closure_at_<start line number>[_<end line number>]"`, where `<end line number>` is used for multi-line closures. Or `"f()::closure{<start line number>:<column number>..<end line number>:<column number>}"`. 
* Local functions. The enclosing function is instrumented (and probably the local one too). Naming: `"enclosing()::nested"`.
* unsafe, async
* Trait 
  * OK. Functions (default implementations in the trait), `"Trait::TraitMethod"`
  * OK (manual naming). Implementations (of the pure virtual functions with no definition, no body), `"<MyStruct as Trait>::TraitMethod"`
  * Added separately to TODO. Overrides (overrides of the default implementation, if possible). `"<MyStruct as Trait>::TraitMethod"`
  * Tested. Pure virtual is not loggable.
    ```rs
    trait MyPureTrait {
        // #[loggable]      // Error: expected `|`
        fn pure_method(&self); // No defualt behavior. Pure virtual function with no def-n.
    }
    ```
* `#[loggable(name="MyStruct::new")]`. Assoc funcs prefix `MyStruct::`.
* Not applicable (Happens before mangling). Demangling
* Outputing to stream (stdout, stderr, file, socket/pipe, [`mcsp::channel`]).


