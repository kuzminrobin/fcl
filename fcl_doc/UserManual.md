## User Manual

### Troubleshooting

#### Panic Message "already borrowed: BorrowMutError ..."
If you see a panic in an FCL's source file with the message containing a fragment like this
`already borrowed: BorrowMutError`, 
in particular a broader panic message could look like this
```
While FCL was busy (arbiter borrowed) one of the threads has panicked: 'panicked at fcl\src\call_log_infra.rs:612:36:
already borrowed: BorrowMutError'.
FCL failed to synchronize its cache and buffers with the panic report below. If the panic report is not shown, attach the debugger to see the panic details.
```
or like this
```
(stderr) While FCL was busy (arbiter and writer borrowed) one of the threads has panicked: 'panicked at fcl\src\call_log_infra.rs:612:36:
already borrowed: BorrowMutError'.
FCL failed to synchronize its cache and buffers with the panic report below. If the panic report is not shown, attach the debugger to see the panic details.
(stdout) While FCL was busy (arbiter and writer borrowed) one of the threads has panicked: 'panicked at fcl\src\call_log_infra.rs:612:36:
already borrowed: BorrowMutError'.
FCL failed to synchronize its cache and buffers with the panic report below. If the panic report is not shown, attach the debugger to see the panic details.

thread 'T1' panicked at fcl\src\call_log_infra.rs:612:36:
already borrowed: BorrowMutError
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```
(possibly interleaved with the output by non-panicking thread(s) 
and/or `(stderr)` output interfering with the {`(stdout)` output or panic report}),  
then this likely means that the FCL is compiled with the "singlethreaded" feature turned on,
but is used in a multithreaded application. Typically this happens in the Rust workspaces
with multiple binary crates using FCL. If at least one binary crate turns the FCL's
"singlethreaded" feature on (or the "minimal_writer" feature that turns the "singlethreaded" feature on), then FCL will be compiled with the "singlethreaded" feature on 
for all the binary crates of the workspace. 

At the moment of writing this seems as an evident disadvantage of Rust workspaces.
Would be better if a library crate could be compiled with a feature on for some of the 
binary crates of the workspace, and with feature off for the other binary crates of the same workspace.

To work around, you can use multiple copies of the FCL named differntly in your workspace. Use one copy with the feature on for one set of binary crates, and another copy with feature off for the remaining set.
