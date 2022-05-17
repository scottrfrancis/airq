#[cfg(test)]

mod observer;

/***
 * Measurements
 * 
 *  The measurements object is an Observable. As such, it
 *      - has an interface to add/remove observers
 *      - notifies observers when the underlying data store changes/is ready
 *  This particular implementation has some additional interfaces/behaviors
 *      - has an interface to receive an updated set of measurements
 *  When a configurable number of readings have been accumulated, each of the 
 * measures is averaged and observers notified.
 * 
 *  Configuration is passed into the constructor and is immutable.
 */