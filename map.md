# Target Map

## Dir/File Map
TODO: { Consider grouping in one crate `fcl`:
* fcl_traits
* fcl_decorators
* fcl_infra

Overall:
* call_graph
* fcl
* fcl_proc_macros
* use[r]

} // TODO.

```
fcl (Workspace)
    fcl_traits/lib.rs (lib package)
        pub struct ClosureInfo {
            pub start_line: usize,
            pub start_column: usize,
            pub end_line: usize,
            pub end_column: usize,        
        pub enum CalleeName
            Function(&'static str),
            Closure(ClosureInfo)
        pub trait CodeRunNotifiable
            notify_call() {}    // ToDo: Why defualt behavior? `{}` Because the tree-like decorator does nothing during return. The other decor-rs can also do nothing for call and rep.count.
            notify_return() {}
            notify_repeat_count() {}
        macro_rules! CLOSURE_NAME_FORMAT
        pub trait CodeRunDecorator
            get_indent_string() = 0
            get_callee_name_string() { .. }
    fcl_decorators (lib package)
        CommonDecorator
            line_end_pending
            writer
            CLOSURE_NAME_FORMAT!
            impl
                new()
                get_callee_name_string()                
            macro_rules! decorator_write
        pub CodeLikeDecorator
            common: CommonDecorator
            indent_step
            impl
                new()
            impl CodeRunDecorator
                get_indent_string() 
            impl CoderunNotifiable
                notify_call() {}
                notify_return() {}
                notify_repeat_count() {}
        pub TreeLikeDecorator
            indent_step_call          `+-`  f() {}
            indent_step_noncall       `  `  Repeats ..
            indent_step_parent        `| `  Prepends multiple times those above.
            impl
                new()
            impl CodeRunDecorator
                get_indent_string() 
            impl CoderunNotifiable
                notify_call() {}
                notify_return() {}
                notify_repeat_count() {}
    call_graph (lib package)
        pub CallGraph
            pub new
            pub add_call()
            pub add_ret()

            call_stack
            current
            caching_model   (Node)
            coderun_notifiable: dyn trait CoderunNotifiable
    {fcl | fcl_infra} (lib package)
        lib.rs
            pub CallLogger  // TODO: -> FunctionLogger
                pub new()
                pub Drop::drop()
            pub ClosureLogger
                pub new()
                pub Drop::drop()
            closure_logger!()   // TODO: Group together with fcl_proc_macros::call_logger!()
        call_log_infra.rs
            pub CallLogInfra
                pub log_call()
                pub log_ret()

                pub new()
                pub [(push|pop|set)_]is_on()
            thread_local! {
                pub static CALL_LOG_INFRA: RefCell<CallLogInfra>
    fcl_proc_macros (proc-macro package)
        pub call_logger!()
        pub loggable!()

        quote_as_itemfn()   // TODO: -> quote_as_function
        quote_as_implitemfn()   // TODO: -> quote_as_assoc_func
        // Closure
        quote_as_closure()
        struct ExprClosureWOptComma
            impl Parse
                parse()
        // #[loggable] args (MyTrate::my_func)
        struct AttrArgs
        parse_attr_args()
    user (bin package)
```

## The `use` Map

(The less indented one uses (`use`) the more indented ones below it)

```
pub CodeLikeDecorator
    CommonDecorator
        pub trait CodeRunDecorator
            get_indent_string() = 0
            get_callee_name_string() { .. }
    get_indent_string() 
pub TreeLikeDecorator
    CommonDecorator
        pub trait CodeRunDecorator
            get_indent_string() = 0
            get_callee_name_string()  { .. }
    get_indent_string()

pub CallLogger
    CallLogInfra
        CodeRunNotifiable
        CallGraph
            CodeRunNotifiable
        {CodeRunDecorator}
            ...
pub CodeRunDecorator
    CodeRunNotifiable
```
