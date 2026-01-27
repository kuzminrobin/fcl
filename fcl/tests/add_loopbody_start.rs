use fcl_proc_macros::loggable;

#[test]
fn basics() {
    #[loggable]
    fn f() {}

    #[loggable]
    fn loop_container() {
        let mut result = 0;
        let loop_result = for i in 0..4 {
            result += i;

            // In some iterations
            if i & 1 != 0 {
                f(); // generate some call log.
            }
        };
        // Assert: Behavior didn't change.
        assert_eq!(result, 6);
        assert_eq!(loop_result, ());
    }

    loop_container();
}
