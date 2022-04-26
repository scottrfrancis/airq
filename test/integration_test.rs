use airq;

#[test]
fn tests_are_active() {
    assert!(true == !false);
}

#[test]
fn returns_first_arg() {
    let args = ["a"];
    let result = parse_config(&args);
    assert!(args[1] == result);
}
