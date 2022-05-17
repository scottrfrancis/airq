use mockall::*;
use mockall::predicate::*;
use super::*;


mock! {
    ConcreteObserver {}
    impl IObserver for ConcreteObserver {
        fn update(&self);
    }
}


#[cfg(test)]
mod observer_tests {
    use crate::observer::{ConcreteObserver, Subject, ISubject};

    use super::MockConcreteObserver;


    #[test]
    fn test1() {
        assert!(true);
    }

    #[test]
    fn create_subject_add_observer() {
        let mut sub = Subject::new();

        let obs_a = ConcreteObserver { id: 100 };
        let mock_a = MockConcreteObserver { id: 101 };

        sub.attach(&obs_a);
        sub.notify_observers();

        sub.attach(mock_a.);
        sub.notify_observers();
        mock_a.expect_update().times(1);

        let obs_b = ConcreteObserver { id: 101 };
        sub.attach(&obs_b);
        sub.notify_observers();

        sub.detach(&obs_a);
        sub.notify_observers()
    }


}