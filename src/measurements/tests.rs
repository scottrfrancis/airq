mod tests;

mod measurements_tests {
    use create::measurements::*;

    /**
     * Tests the Measurements Observable struct/impl
     * backlog:
     * 
     *  [ ] has an interface to add/remove observers
     *  [ ] notifies observers when the underlying data store changes/is ready
     *  [ ] has an interface to receive a single set of measurements to add to accumulator
     *  [ ] is configured on construction with a number of slots to average
     *  [ ] observers are only notified when the configured number of slots has been accumulated and averaged
     *  [ ] when the accumulator is averaged (and observers notifiied), the accumulator is emptied/fully consumed
     * 
     */
    
    fn create_measurements() {
        let m = Measurements();
    }
}