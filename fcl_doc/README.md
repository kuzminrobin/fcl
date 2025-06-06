# fcl
Rust Function Call Logger

# Why
Learning material to practice. I perceive it as a test covering the material that the young minds grasp in one semester.  
To get a record in my resume since (at the moment of writing) I'm unemployed, I live at the expense of my savings, 
and soon I will need to apply for jobs (after I study Rust to a level sufficient to pass (or bypass) an interview;-).


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

