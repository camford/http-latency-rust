# httplatency

A tool for checking the latency when performing HTTP GET requests.

## Building the project

You'll need to have Rust (https://www.rust-lang.org/) installed. Rust comes
with a tool for managing projects called cargo. To build the httplatency just
use:

```bash
$ cargo build --release
```

## Using the tool

To run the tool you can either use ``cargo run`` or you can run the binary from
``$PROJECT_HOME/target/release/httplatency``

## Running the tests

To run tests, we simply use cargo again: ``cargo test``.  This runs bothe the unit tests and the tests that appear in the documentation.

## Generating documentation

To generate and see the api documentation run ``cargo doc --no-deps --open``

## Design assumptions

Some assumptions have been made in the development of this project:

 * The tool should measure latency regardless of the HTTP Status code returned
 * If a url in the file doesn't specify a scheme or port 443 HTTP is assumed
 * If an existing filename is specified for output we will overwrite the file without prompting

## Known Issues

Due a lack of maturity in some Rust libraries the following issues exist and are known.

### Issues resulting from bugs in ``hyper`` library
 * If a webserver refuses a request hyper will panic! on ``client.get`` and the program exit prematurely
 * If a url contains a non-existant domain hyper's DNS lookup will fail on ``client.get`` and the program will exit prematurely - this is captured as a (currently failing) test
 * HTTP requests through a proxy are not currently supported by the hyper library
 * If a webserver holds the connection open then the program will block until an entire response is received. While hyper claims to accept a timeout in my testing this does not behave as expected. As such, I have removed the ability for the user to set a flag specifiying a timeout for requests in this program.
