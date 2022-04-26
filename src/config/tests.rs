use super::*;


mod config_tests {
    #[test]
    fn returns_first_arg() {
        // the 'first' arg is index 1 -- AFTER the command name
        let args = [
            String::from("exec-name"),
            String::from("first-arg")
         ];
        let result = super::parse_config(&args);

        assert!(args[1] == result);
    }

    #[test]
    fn returns_only_first_arg() {
        let args = [
            String::from("exec-name"),
            String::from("first-arg"),
            String::from("extra-arg")
         ];    
         
         let result = super::parse_config(&args);
 
         assert!(args[1] == result);
    }

    #[test]
    fn panics_with_no_args() {
        let args = [
            String::from("exec-name")
        ];
        let result = std::panic::catch_unwind(|| super::parse_config(&args));
        assert!(result.is_err());
    }
}
