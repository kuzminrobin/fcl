# Practicing Rust by Developing a Function Call Logger

## How the Function Logging Works

### A Little Bit of History
The first implementation of the function call logging was done by me in C programming language in the hardware simulation environment.
I was developing a code that simulated the hardware. In other words, I was developing virtual hardware. That virtual hardware was used to test the firmware developed by the other teams before the real hardware was available. 

The firmware function calls were logged by a hook in the virtual hardware. That hook was triggered upon execution of the firmware's `call` machine instruction - the instruction that calls a function (by pushing the return address from the program counter to the stack and placing to the program counter the function address specified in the instruction). The hook was extracting the function address from the machine instruction, was using the firmware debugging information to map the address to the function name, and was logging the function call (with the indent based on the call depth).
```rs
firmware_function() {
```
Upon the `ret` machine instruction - the instruction that returns from a function (by popping from the stack to the program counter the return address) the hook was logging the function return (by getting the function name from one's own interanl call stack, and again with the indent based on the call depth). 
```rs
} // firmware_function()
```
Typical software doesn't have such hooks. So, how can a piece of code, that logs the function calls and returns, be triggered upon every function call and return? 

### Back to the Present

Let's say a user (that might be you) wants to get their function's calls and returns logged.

User's code:
```rs
fn users_func() {
  . . .
}
```
Log:
```rs
users_func() {
  . . .
} // users_func().
```
One of the ways to achieve that is to create in the beginning of the `users_func()` an instance, let's name it `function_logger`, 
* whose constructor 
  * will be gettign the name of the function (`"users_func"`), 
  * logging the function call, 
* and whose destructor will be logging the function return.
```rs
fn users_func() {
  let function_logger = FunctionLogger::new(&"users_func");
  . . .
  // Here the function_logger's destructor gets triggered.
}
```
For such functionality to work the constructor can 
* log the function by using `println!()`, 
* memorize the function name in the instance's field. 

Then that name can be reused in the destructor when logging the return, again with `println!()`.

And this approach can be used in every logged function. We just need to instrument every function of interest with the `function_logger` instance creation. More details about how to do that will be provided later.

### The First Issues

The first issue that arizes is the nested function calls and indents. Let's say the user has function `f()` that calls function `g()`.
```rs
fn g() {
  . . .
}

fn f() {
  . . .
  g();
  . . .
}
```
It makes sense to log the call to `g()` indented by one level.
```rs
f() {
  g() {}
} // f().
```
In order to save the space in the log, it also makes sense 
* to log the return from `g()` in the same line as the call: `g() {}`, 
* whereas the return from `f()` looks better when logged in a line separate from the call:
```rs
f() {
} // f().
```

To get that, the instances of the `function_logger` in `f()` and `g()` can share the global call depth. For example, the instance of the `function_logger` in `f()` 
* can log the call to `f()` with the indent level equal to the current value `0` of the global call depth,
* increment the call depth to `1`.
```rs
f() {
```
The instance of the `function_logger` in `g()`
* can log the call to `g()` with the indent level equal to the current call depth `1`,
* increment the call depth to `2`,
```rs
f() {
  g() {
```
* the `g()`'s `function_logger`'s destructor would then decrement the call depth back to `1`,
* and log the return from `g()` with the indent level of `1` (but since `g()` has no nested calls, the return would be logged in the same line as return).
```rs
f() {
  g() {}
```
Then the destructor of `function_logger` in `f()`
* would decrement the call depth back to `0`,
* and log the return from `f()` with the indent level equal to the call depth of `0`.
```rs
f() {
  g() {}
} // f().
```

### The Next Issues

What if the function `g()` is called by `f()` in a loop with 100 iterations? Would user like to see `g()` logged 100 times? Or would the user prefer those calls to be grouped?  
What if the function `g()` has its own nested calls that repeat in some iterations and differ in the others?

When answering all those questions we eventually come to the concept of the call graph.

## The Call Graph

The call graph consists of the two parts. 

The first part is the _call tree_. For example, if we have 
* function `a()`, 
* that calls functions `b()`, `c()`, and `d()`, 
* function `c()` in its turn calls functions `e()` and `f()`, 
then the call tree will look like this.
```
     a()
   /  |  \
b()  c()  d()
    /  \
  e()  f()
```
The tree is singly linked (unidirectional), that is the links point from the root towards the leaves.

Periodically the need arises to jump from a leave towards the root. For example, when a function returns to its caller (parent). The links towards the root (making the tree doubly linked or bidirectional) would easily solve the problem. But they would add an overhead to the tree.

The experience from the "future" shows that the need to jump towards the root arises at specific moments and at specific places of the call tree. In particular, as mentioned above, when the algorithm needs to jump from the current function to its caller - parent. For example, when the function `e()` returns, the algorithm needs to jump to function `c()`, such that the subsequent call to function `f()` is added as a child to the function `c()` but not to function `e()`.

For such jumps to the parent the algorithm uses the second part of the call graph — the _call stack_. The call stack contains pointers to the corresponding call tree nodes on the path from the root to the current function. For example, if function `e()` is the current function, then the call stack has pointers to `a()`, `c()`, and `e()`. 
```
The Call Tree during      The Call Stack during
the call to `e()`         the call to `e()`
     a() <-----------------[ ]  Bottom
   /  |                     |
b()  c() <-----------------[ ]
    /                       |
  e() <--------------------[ ]  Top
```
When the function `e()` returns, the algorithm pops from the call stack the node with the pointer to `e()`, and the node with the pointer to `c()` becomes the top of the call stack, thus making `c()` a current function. The node `e()` stays in the call tree, pointed to by the node `c()` only.

### The Repeated Calls

Let's imagine that a function `f()` calls function `g()` 100 times in a loop. Does the user want to see 100 repeating calls to `g()` in the log?
```rs
f() {
  g() {} // (iteration 0)
  g() {} // (iteration 1)
  . . .
  g() {} // (iteration 99)
} // f().
```
I personally as a user would prefer to see in the log about the following.
```rs
f() {
  g() {}
  // g() repeats 99 time(s).
} // f().
```
For such a functionality the FCL algorithm needs to use _caching_.

#### Caching

If after the first call to `g()`
```rs
f() {
  g() {}
```
another call to `g()` is done, then the FCL algorithm, instead of logging the repeated call to `g()`, needs to hold that call in the call tree without logging it. That is, the repeated call to `g()` needs to be _cached_. We can say that the caching starts upon the second call to `g()`.
```rs
f() {
  g() {}
  g() { // This call is being cached. Not logged yet.
```
When the second call to `g()` returns, the algorithm needs to compare the second call to `g()` with the first call to `g()`, including the nested calls (children), their order, and their numer of repeated calls. That is, the algorithm needs to compare the two subtrees rooted in the first and the second `g()` correspondingly.

```rs
f() {
  g() { . . . }
  g() { . . . } // This call is cached. Not logged yet.
```
If the two subtrees are identical, then the second subtree gets removed from the call tree, and the repeat count for the first call to `g()` gets incremented (from 0 to 1 at this moment).
```rs
f() {
  g() { . . . }
  // g() repeats 1 time(s).  // (This is not logged yet)
```

This algorithm is applied as long as the repeated call's subtree (the later `g()` on the pictures above) is identical to the preceding call's subtree (the earlier `g()`).

If the repeated call's subtree is different, then the caching stops, which means that
* the latest call stays in the call tree,
* the preceding call's repeat count is flushed to the log,
* and the latest call's subtree is also flushed.
```rs
f() {
  g() { . . . }
  // g() repeats 3 time(s). // (This repeat count is flushed upon return of the subsequent call to g())
  g() {
    h() {} // (This nested call is the difference from the previous calls to g())
  } // (The whole latest subtree of g() is flushed to the log upon this return and is retained in the call tree)
```
Making a step back, let's imagine that in the picture above after the first 4 identical calls to `g()`, caching has started upon the fifth (latest) call to `g()` (when the repeat count of the previous `g()` is 3), and that call has nested repeated calls to `h()`. 
```rs
f() {
  g() {}
  // g() repeats 3 time(s).
  g() {
    h() {}
    h() {
      . . .
    }
  } // g()
```
The caching continues until the fifth (latest) call to `g()` returns. This means that if the second call to `h()` has a subtree different form the subtree of the first call to `h()`, then the second call to `h()` will stay in the call tree, without incrementing the repeat count of the first call to `h()`, but none of the calls to `h()`, their subtrees, and their repeat counts will be flushed to the log until caching stops upon return from the latest `g()`.

### The Loops Calling Multiple Functions

Let's imagine that function `f()` has a loop with 100 iterations, and the loop body calls the functions `g()`, `h()`, and `i()`. Having the algorithm so far, the log will contain all the iterations of the loop since the sequence of `g()`, `h()`, and `i()` is not a sequence of repeated calls to the same function:
```rs
f() {
  g() {} // (Iteration 0)
  h() {}
  i() {}
  . . .
  g() {} // (Iteration 99)
  h() {}
  i() {}
} // f()
```
I as a user would prefer to see about the following:
```rs
f() {
  { // Loop body starts.
    g() {}
    h() {}
    i() {}
  } // Loop body ends.
  // Loop body repeats 99 time(s).
} // f()
```
For the repeated loop bodies the similar considerations are applicable as for the repeted calls. If the later loop body's subtree is identical to that of the earlier loop body, then the later subtree gets removed from the call graph and the repeat count for the earler loop body gets incremented.

But if it turns out that the loop body does not make any calls, then I would prefer that loop body to not be logged at all.
```rs
f() {} // The loop does not make any calls even though those calls 
       // can be present in the loop body, but the condition to make those calls is not satisified.
```
To summarize, for the loop bodies, as opposed to the repeated function calls, the caching starts with the first loop body. That loop body is called _initial_ in the algorithm. If the initial loop body ends without calling any function then it is removed from the call fraph, and the subsequent loop body, if any, becomes initial.

As soon as a function call happens in the initial loop body, the initial loop body's caching stops, that loop body's subtree is flushed:
```rs
f() {
  { // Loop body starts.
    g() { // (The first function call in the initial loop body)
```
The remaining part of the initial loop body is logged without caching.
```rs
f() {
  { // Loop body starts.
    g() {}
    h() {}
    i() {}
  } // Loop body ends.
```
The subsequent loop body, if any, is cached similar to the repeated function calls. If, upon end, that loop body's subtree is identical to that of the preceeding loop body, then the repeating loop body is removed from the call graph and the repeat count of the preceding loop body is incremented. If, upon end, the subsequent loop body's subtree differs, then 
* it is retained in the call tree,
* the repeat count of the preceding loop body is flushed to the log,
* the latest loop body's subtree is flushed.
```rs
f() {
  { // Loop body starts.
    g() {}
    h() {}
    i() {}
  } // Loop body ends.
  // Loop body repeats 2 time(s). // (This repeat count is flushed)
  { // Loop body starts. // (This loop body differs from the previos ones)
    g() {
      j() {} // (This is the first difference from the earlier loop bodies)
    }
  } // Loop body ends.
```

### The Pseudonode (TODO: Consider -> pseudoroot)
Let's imagine that the FCL is used for logging a program having the following picture of the function calls
```
        main()
    /    |     \
init()  ...      ...
 / \    / \      / \
 ...    ...      ...
```
(the `main()` function calls a number of functions each of which is a subtree of calls)

Now let's imagine that the logging is enabled after the call to `main()` but before the call to `init()`.
The node for `init()` will be created as a root of the call tree, and the pointer to that root will be added to the bottom of the call stack. 
```
The Call Tree             The Call Stack       
     init() <-----------------[ ]  Bottom
```
Upon return the node at the bottom of the call stack, pointing to the root in the call tree, will be popped from the call stack. 

Since none of the call stack nodes and none of the call tree nodes will be pointing to the root node of `init()`, the node will be destroyed in the call tree (if it is not destroyed then upon adding more sibling-level nodes the call graph will stop being a tree, it will become a sequence of call trees, which complicates the algorithm).

The subsequent call after `init()` will be added as a root again.

If the subsequent call is `init()` again (a repeated call), in order to make a decision about whether to start caching, the algorithm will have no the first `init()`'s node in the call tree. The algorithm will have to fully log every repeated `init()`.

To retain the repeat counting of the top-level calls, and to keep the algorithm simple and unified, the pseudonode is always added as the first node to the call graph.
```
The Call Tree             The Call Stack       
  pseudonode <--------------[ ]  Bottom
```
All the subsequent nodes, including `main()`, are added as children, grandchildren, and other successors of the pseudonode. 

Thus the topology of calls is _always a tree_, even if the logging gets enabled after the call to `main()`, in which case the first call to `init()` is added as a child of pseudonode, upon return it is retained in the call tree, and the subsequent repeated calls to `init()` are cached and can end up in incrementing the first `init()`'s repeat count. 

In my experience there were embedded systems where `main()` was returning and was called again. In such systems if the logging gets enabled before the call to `main()` then the repeated calls to `main()` will be cached and may end up in incrementing the repeat count of the preceding call to `main()`. 

### The Logging Indent

The algorithm uses the length of the call stack (or call depth) to determine the _indent_ used for logging the function calls and returns. For an example, let's consider the following call tree.
```
  pseudonode
      |
     a()
   /  |  \
b()  c()  d()
    /  \
  e()  f()
```
When logging the call and return for function `a()`, the length of the call stack is 2 (the call stack contains the pseudonode and `a()`). For logging the funcitons `b()`, `c()`, and `d()`, the length is 3, and so on. 

The actual indent level is a value 2 less than the length of the call stack, i.e. the indent level is
* 0 for logging `a()`, 
* 1 for logging `b()`, `c()`, and `d()`, and so on.
```rs
a() {       // The indent level is 0.
  b() {}    // The indent level is   1.
  c() {     // The indent level is   1.
    e() {}  // The indent level is     2.
    f() {}  // The indent level is     2.
  }         // The indent level is   1.
  d() {}    // The indent level is   1.
}           // The indent level is 0.
```

### The Multithreading Aspect

Since each thread has a separate stack (provided by the runtime and/or operating system), for each thread the FCL creates a separate instance of the call graph (consisting of the call tree and the call stack). A repeated sequential spawning of threads with the same thread function will not result in caching since those are different threads with different call graphs. Upon thread termination its call graph, after flushing all the cached data, is destroyed.

If some bizarre multithreading mechanism does not destroy the thread context (and thread id) upon the thread function termination, and later reinitializes and reuses that thread context (and thread id) to invoke the same thread function, and the thread-local data (containing the call graph) survive during that "thread restart" then caching of the thread function repeated call will start. But so far such implemetations have never been met.

## Logging Endlessly

In some environments or cases it is hard to catch a failure in the debugger or to generate a core (crash) dump. In those occurrences the developer needs to see what was going on shortly before the failure. The FCL can help in such cases.

The situation becomes a bit tricky when the software, such as a daemon or an embedded firmware, is running endlessly, and the failure happens rarely, like once a month.

As for the log storage, the FCL, _customized by the user_, can log the function calls (interleaved with the binary's own debugging output) not only to a terminal but to a file or memory in a circular manner, such that the oldest log entries are overwritten with the new ones. When the failure happens the developer can see the log of the last 3 days or 2 weeks, etc., depending on the settings and available storage.

But what does happen to the dynamic memory occupied by the call tree? Based on the logic so far the call tree grows endlessly. This will exhaust the memory. How can FCL log endlessly but still retain all the functionality?

Let's consider a simple `main()` function that calls `init()`, after which it calls `work()` in a loop. During logging the `main()` gets added as a child to the pseudonode. Then `init()` is added as a child of `main()`, then the `work()` (with its children) is added after `init()`, the repeated `work()` increments the repeat count of the first `work()` and gets removed from the call graph. The `work()` with the subtree different from the one of the first `work()`, stays in the call tree, and so on.

During the loop the FCL algorithm analyses the two latest calls to `work()` in order to make a decision whether to leave the latest `work()` in the call tree and log it, or to remove it and increment the repeat count of the preceding `work()`.

In that decision-making the `init()` is not needed in the call tree and can potentially be removed (it is already logged). 
But if we remove it, then the list of children in `main()` will be distorted. 
This will affect the rare implementations where `main()`, upon certain condition or periodically, can return, and then get called again. 
And upon return from the second (repeated) `main()` the FCL needs to compare the second `main()`'s subtree with that of the first `main()` and either log the second `main()`'s subtree or remove it and increment the repeat count of the first `main()`. 
But if the FCL's algorithm removes the `main()`'s child `init()`, then the comparison of the two adjacent subtrees of `main()` will be distorted becuase those subtrees can differ in nested `init()` subtrees or their repeat count.

To summarize, for the common case we cannot remove `init()` from the call graph (when we proceed to handling the calls to `work()`), because the `init()` particiaptes in comparing the adjacent subtrees of `main()`.

But what if `main()` is not added to the call graph? What if logging starts after the call to `main()` but before the call to `init()`? In that case the `init()` will be added to the call graph as the first child of the pseudonode and logged. Then upon the first call to `work()` the algorighm will see that the `work()` has a name different from `init()`, thus the caching will not be triggered, the `work()` will be added to the call graph, logged without caching, and the `init()` will not be needed in the call graph starting with the call to `work()`. That is why, upon the first call to `work()`, the `init()` can be removed from the list of pseudonode's children , and the `work()` can be added as the first child instead.  
(TODO: Not yet implemented. Reader Practice?)

The repeated calls to `work()` will increment the repeat count of the first `work()`.

Any different call to `work()`, the one having subtree different from that of the first `work()`, will cause
* caching stop, 
* the first `work()`'s repeat count flush, 
* removal of the first `work()` from the list of pseudonode's children, 
* and adding the latest `work()` as the fist child of the pseudonode.

(TODO: Not yet implemented. Reader Practice?)

This can continue endlessly. At any moment the pseudonode will have at most 2 latest children: 
* either 1 that is being added to the call tree and logged without caching, 
* or 1 that is fully added and logged, plus 1 that is being added and cached. 

For that to work the `main()` must not be logged. That is (TODO: Requires familiarity with `#[loggable]`, `#[non_loggable]`, automatic unstrumentation), 
* either the `main()` must not be instrumented with `#[loggable]`, 
* or the logging must be enabled after the call to `main()`,
* or, if `main()` is inside of a module marked as `#[loggable]`, the automatic unstrumentation of `main()` must be suppressed with `#[non_loggable]`.

The same is applicable to the nested functions of `main()` _running for a long time_, comparable to `main()`.

### Logging the Threads Endlessly

The same is also applicable to the _thread functions_ (and nested functions of those) _running for a long time_ (comparable to `main()`). 

A thread function is the top-level function that is invoked when a thread is spawned (it's like `main()` for a thread). The thread function, if logged, stays in the call graph until it returns. Upon the thread function return (thread termination) the thread's call graph is destroyed as part of the thread-local data and the dynamic memory is deallocated.

The more threads are spawned and logged in parallel, the higher is the chance of exhausting the dynamic memory. If the thread function is logged then its subtree stays in memory until the thread function return. But if the thread function is not logged, but its children are, then at most 2 latest children's subtrees stay in memory.

To summarize, 
* if the long-living thread functions and their long-living successors (children, grandchildren, etc.) are not logged (but their short-living nested calls are),
* or the threads terminate quickly

then the thread logging can last endlessly.

TODO: Unneeded node removal from the pseudonode children is not yet implemented.

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

## The Architecture

The default architecture is shown on the chart below.

(Full Architecture Chart)

It is _full_ architecture. It supports multiple threads and the output synchronization between the threads 
and the user code's `stdout`/`stderr` output. For brevity it shows the data for 2 threads only,
one of who is the `main()` thread.

The chart has 2 horizontal dashed lines breaking the chart up into 3 layers.
The middle layer shows the global data shared by all the threads, 
the upper layer shows the data for the `main()` thread, 
and the lower layer shows the per-thread data for each spawned thread. 

The `main()` thread's data (the upper layer) have nothing specific to `main()`. 
The `main()`'s data are the same as for any other thread (see the lower layer).

The full architecture is supposed to work for all the cases but 
* it may distort the multithreaded log,
* does not provide the highest parallelization, 
* and for some cases it is redundant,

all of which is described below.

### The Single-Threaded Optimization

If the user's program is single-threaded then the full architecture can be minimized down to the 
_single-threaded_ architecture shown on the chart below (includes the red blocks and arrows but does 
not include the dotted arrow).

(Single-Threaded Architecture Chart)

This architecture 
* does not have `ThreadGateAdapter`, `ThreadGatekeeper` (with mutex),
* and the `THREAD_LOGGER` is connected directly to the `CallLoggerArbiter`.

It supports a single thread (`main()`) and the function call log synchronization with the user code's 
`stdout`/`stderr` output.

This optimizaton is chosen if the feature `"singlethreaded"` is on for both `fcl` and `fcl_proc_macros` crates.

TODO: What if `fcl`'s one is on, but `fcl_proc_macros`' one is not? And vice versa.


### The Minimal Writer Optimization

If the user's program is single-threaded and does not use `stdout`/`stderr` output 
then the output synchronization mechanism, see the red blocks and arrows, can be excluded from the FCL architecture, and the decorator (`CodeLikeDecorator` on the chart) can have 
a direct pointer to the output stream (`stdout()` on the chart), see the dotted line from the
`CodeLikeDecorator` to `stdout()`.

TODO: The `CallLoggerArbiter` should be red too, 
and the `THREAD_LOGGER` should have a dotted arrow to the `CallLogInfra` directly.

This optimizaton is chosen if 
* the `fcl` crate is used with the feature `"minimal_writer"` on
(that includes the feature `"singlethreaded"` and automatically turns it on too),
* the `fcl_proc_macros` crate is used with the feature `"singlethreaded"` on.

TODO: What if `fcl`'s `"minimal_writer"` is on, but `fcl_proc_macros`' `"singlethreaded"` is not?

## How the Logging Works

Just in case I'll remind that if in the user's Rust code the attribute `#[fcl_proc_macros::loggable]`
has been added to an item (module, trait implementation, function, etc.) then before compilation 
that item is parsed recursively and all the functions, closures, and loop bodies in it 
get instrumented for logging. That is, by example of a function `f()`, in the beginning of `f()` 
the code is added that creates an instance of the `FunctionLogger`. 
What happens during the execution of `f()` is easier to understand while looking at the chart.

When `f()` gets called, 
* the constructor of `FunctionLogger` instance in the beginning of `f()` uses the thread-local name 
  `THREAD_LOGGER` (top-left box on the chart) to call `ThreadGateAdapter::log_call()`
  (i.e. the associated function `log_call()` that is a part of the `CallLogger` trait 
  implemented by `ThreadGateAdapter`).
* The `ThreadGateAdapter::log_call()` acquires the mutex and forwards the call 
  to the `ThreadGatekeeper::log_call()`.
* The `ThreadGatekeeper::log_call()` forwards the call to the `CallLoggerArbiter::log_call()`,
* who forwards the call to `CallLogInfra::log_call()`.

Here the node for `f()` is added to the call graph and the `CallLogInfra` calls 
the `CodeLikeDecorator::notify_call()` to log the call to `f()`. 
The `CodeLikeDecorator` generates the line `"f() {"` prepended with the corresponding indents 
(thread-dependent and call-depth-dependent) and calls the `WriterAdapter::write()`,
who forwards the call to `ThreadSharedWriter::write()`, who forwards that call to the output stream 
that is `std::io::stdout()` by defualt.

When `f()` returns, the destructor of `FunctionLogger` uses `THREAD_LOGGER` again
to call `ThreadGateAdapter::log_ret()` and in a similar chain of calls the return from `f()` 
is added to the call graph in the `CallLogInfra` and logged by the `ThreadSharedWriter`.

The repeated calls to `f()` increment the `f()`'s repeat count in the call graph in the `CallLogInfra`
without logging. The other calls cause `CallLogInfra` to flush to the `CodeLikeDecorator` 
the `f()`'s repeat count and the call to the new function. 

## The Logging Infrastructure Creation

The middle layer of the chart (the gloabl data) is created first.

Then, upon thread start the thread-local data are created for that thread. See the upper layer of the chart.  
The `THREAD_SHARED_WRITER` is cloned and passed to the constructor of `WriterAdapter`. 
The instance of the `WriterAdapter` is wrapped into a `Box` and passed to the `CodeLikeDecorator` constructor,
whose instance is wrapped into `Rc` and saved under the thread-local name `THREAD_DECORATOR`.

Then the `THREAD_DECORATOR` is cloned and passed to the `CallLogInfra` constructor, 
the instance of which is wrapped into a `Box` and passed to the `CallLoggerArbiter`'s container 
(through the `ThreadGatekeeper` after acquiring the `THREAD_GATEKEEPER`'s mutex).

Then the `THREAD_GATEKEEPER`'s internal `Arc`-pointer is cloned, 
passed to the `ThreadGateAdapter`'s constructor, instance of which is wrapped into a `Box`
and saved under the thread-local name `THREAD_LOGGER`.

## The Logging Infrastructure Destruction

Look at the multithreaded chart. There are 2 horizontal dashed lines on it. Below the lower of them 
there are the data of the spawned thread.

Upon thread termination its thread-local data are destroyed, including the `THREAD_LOGGER` 
(bottom left on the chart). 
Its destructor calls `Drop::drop()` of the `ThreadGateAdapter`, that calls (through the `ThreadGatekeeper`)
the `CallLoggerArbiter::remove_thread_logger()` (TODO: Double-check the name). 
That removes from the container the `Box` pointer to the thread's `CallLogInfra` and destroys it. 
The `CallLogInfra`'s destructor destroys the decorator (`CodeLikeDecorator`), 
who destroys the `WriterAdapter` that detaches from the `ThreadSharedWriter`.

To summarize, upon thread termination during the destruction of the thread-local data 
everything below the lower dashed line 
* (in particular `CallLogInfra`) stops being pointed to from the middle layer of the chart, 
* stops pointing to the middle layer (`ThreadGatekeeper` on the left and `ThreadSharedWriter` on the right)
* and gets destroyed.

The same is applicable to the `main()` thread, whose data are shown above the upper dashed line.

Upon program termination the middle layer (the global data) is destroyed.

## The Log Distortion During the Thread Switch 

(Unsorted. Requires knowledge of the architecture, in particular the single mutex approach)

The single mutex can significantly distort the thread constext switch picture. 

For example, the first thread locks the mutex and starts updating the call graph.
The thread context switches to the second thread (or the second thread is running in parallel on a different CPU core).
The second thread makes a function call and tries to lock the mutex to log that call,
but the mutex is already locked, so the second thread starts waiting for the mutex,
returning the execution back to the first thread. 
Depending on the operating system's approach to the thread synchronization, 
in some scenarios as soon as the first thread releases the mutex the thread context can immediately be switched to the second thread waiting for the mutex.
But in other scenarios the first thread after releasing the mutex can continue the execution
until the expiration of the time quantum, can log a few calls more, and again get interrupted 
while the mutex is locked. The second thread will have to wait for the mutex again without moving forward. 
This can repeat an unpredictable number of times.

To summarize, the log can show that the thread switch happened later relative to the first thread's log, and with one attempt, but in reality there could be a number of unsuccessful attempts to switch the thread context earlier than what the log shows. The user should be ready for such a distortion of the thread context switch picture and understand that certain timing-dependent scenarios can be affected by the FCL.

How large is the distortion? Depends on the thread synchrinization approach of the operating system and on the code being logged. If there are lengthy fragments without loops and calls, then the distortion is minimal. But the more often the code execution passes through the starts and ends of the loops, functions, and closures the larger is the distortion (and the slow-down because of logging. TODO: explain the slow-down in the Performance Impact chapter).

### Minimizing the Log Distortion
(Raw, draft)  
Consider placing the thread synchronization mechanism (ThreadGatekeeper and CallLoggerArbiter)
after the CallLogInfra, such that the threads can access their own CallLogInfra in parallel.
Then, when a CallLogInfra tries to flush, it locks the ThreadGatekeeper/CallLoggerArbiter mutex, 
and through the ThreadGatekeeper/CallLoggerArbiter flushes all the threads' cache 
(in all the CallLogInfra instances) in the order of updates.
For that to work, 
* each CallLogInfra instance needs to have an extra mutex that synchronizes the access between the 
  thread (updating the call graph) and the flush by the ThreadGatekeeper/CallLoggerArbiter running 
  in the context of the other thread (Footnote A);
* every update to the call graph needs to have an update count/ID acquired from the global thread-shared 
  atomic counter.  

Algorithm (see picture/chart below).
* Thread 0 accesses its CallLogInfra instance (CallLogInfra 0). For this the thread acquires the mutex 0.  
  The CallLogInfra 0 gets from the global thread-shared atomic counter the update count `n` (and the counter
  increments). The CallLogInfra 0 adds an update with the update count of `n` to its call graph
  and releases the mutex 0.
* In parallel to that the thread 1 acquires mutex 1 and adds the update with the update count of `n + 1` 
  to its call graph in CallLogInfra 1. Then releases the mutex 1.
* Then the thread 0 adds the update `n + 2` to its call graph.
* Then the thread 1 adds the update `n + 3` to its call graph.
* Then the thread 0 adds an update `n + 4` which results in a flush. The thread 0's CallLogInfra
  acquires the ThredGateKeeper/CallLoggerArbiter mutex, and passes to the arbiter the updates `n`, `n + 2`, `n + 4` 
  (the updates since the last flush of thread 0), the ThredGateKeeper/CallLoggerArbiter polls all the other 
  instances of CallLogInfra and asks to provide their flush data - the updates since the last flush. The 
  CallLogInfra instance of thread 1 responds with the updates `n + 1` and `n + 3`. 
  The ThredGateKeeper/CallLoggerArbiter orders all those updates by ID (n, n + 1, n + 2, n + 3, n + 4) and flushes those to the corresponding decorators (the access to those is exclusive). That is  
  * update n goes to decorator 0,
  * update n + 1 goes to decorator 1,
  * update n + 2 goes to decorator 0,
  * update n + 3 goes to decorator 1,
  * update n + 4 goes to decorator 0.

  (different decorators can use different colors for different threads if they add up to an HTML writer, for example)

(Requires the knowledge of output sync)  
Every std output (in the user's code) will still need to trigger the overall flush.

How to handle the situation when the threads enter the long loops with the repeating iterations? 
That is, the updates to their call graphs take place in parallel, updating the repeat counts without a flush.  
The currently implemented algo shows all those thread context switches, and flushes the cache upon every switch
(even if the new thread does not log anything, in which case instead of `f() {}` we can see the picture
```rs
f() {
}
```
That is, between the call `{` and the return `}` the thread conext switches to another thread, that logs nothing, and back).

But if we use the algo described above, then some of the updates (or groups of them) will get removed 
together with the node being cached, when 
incrementing the repeat count. Every repeat count inc needs to have a separate update count?  
In that case upon nearest flush the output is expected to be  
(thread 0 calls `thread_0_f()` repeatedly, thread 1 calls `thread_1_g()` repeatedly)
```
. . .                          
                              . . .
thread_0_f() {}
// thread_0_f() repeats 3 time(s).
                              // thread_1_g() repeats 5 time(s).
// thread_0_f() repeats 4 time(s).
                              // thread_1_g() repeats 6 time(s).
```
instead of the actual
```
. . .                          
                              . . .
thread_0_f() {}
// thread_0_f() repeats 3 time(s).
thread_0_f() {
                              } // thread_1_g().
                              // thread_1_g() repeats 4 time(s).
                              thread_1_g() {
} // thread_0_f().
// thread_0_f() repeats 3 time(s).
                              } // thread_1_g().
                              // thread_1_g() repeats 5 time(s).
```
This will be a different distortion but a higher share of the threads' code will be running 
in parallel without blocking [each other]. I.e. the performance will be higher.

Footnote A.  
The chart fragment may look suspicious. The thread 0 has a pointer to mutex 0 to access the CallLogInfra 0. 
The CallLogInfra 0 has a pointer to mutex of the ThreadGatekeeper/CallLoggerArbiter.
The ThreadGatekeeper/CallLoggerArbiter has a poiner back to the mutex 0.  
The same for every thread.  
To summarize, there are 2 mutexes in the same loop of pointers, which is grounds for 
* an attempt to lock the same mutex (0) twice by the same thread for _mutable_ access 
  (for an update by the thread, and for a flush by the ThreadGatekeeper/CallLoggerArbiter).
  At a glance one might think that the recursive mutex can solve the problem, but 
  the recursive mutex does not provide the _mutable_ access in Rust.  

This means that an extra knowledge needs to be kept in mind that when the thread 0 has acquired the mutex 0
and accessed CallLogInfra 0, the flush, accessing the ThreadGatekeeper/CallLoggerArbiter, must already
provide to the ThreadGatekeeper/CallLoggerArbiter the cached updates of the CallLogInfra 0
(n, n + 2, n + 4), such that 
the ThreadGatekeeper/CallLoggerArbiter asks for the updates (n + 1, n + 3) from all other CallLogInfra instances, but does not access the mutex of CallLogInfra 0 while that mutex is already locked.
