extern crate httplatency;
extern crate rustc_serialize;
#[macro_use]
extern crate log;
extern crate getopts;

use std::io;
use std::io::BufRead;
use std::io::Write;
use std::fs::File;
use std::env;
use std::process;
use std::error::Error;

use getopts::Options;
use rustc_serialize::json;

use httplatency::Latency;

mod logger;

const DEFAULT_OUTPUT: &'static str = "output.json";

/// Start or the program.
///
/// Co-ordinates the command line arguments and library functions
fn main() {
    match logger::init_console_logger() {
        Err(err) => panic!(format!("Logging setup error : {}", err.description())),
        _ => (),
    }
    println!("HTTP(S) Latency tool");

    let (input, output) = get_args();
    match save_latencies(input, output) {
        Ok(_) => println!("Exiting.."),
        Err(_) => error!("Error writing to file!")
    }
}

/// Manages the command line arguments
///
/// Sets and checks the valid command line arguments. Prints usage and exits if the command line
/// arguments are not valid.
fn get_args() -> (String, Option<String>) {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("i", "input", "set the input filename", "NAME");
    opts.optopt("o", "output", &format!("set the output filename. '{}' will be used if none is provided", DEFAULT_OUTPUT), "NAME");
    opts.optflag("h", "help", "print this help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(f) => { panic!(f.to_string()) }
    };
    if matches.opt_present("h") {
        print_usage(&program, opts);
        process::exit(0);
    }
    let input = if !matches.free.is_empty() {
        matches.free[0].clone()
    } else {
        print_usage(&program, opts);
        process::exit(1);
    };
    let output = matches.opt_str("o");
    (input, output)
}

/// Print the program's instructions
fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} FILE [options]", program);
    print!("{}", opts.usage(&brief));
}

/// Read from file, measure latencies and write results to file
///
/// Maps over a list of strings (taken from input file),
/// checks they're valid http urls,
/// makes a GET request recording the times,
/// and writes results as JSON to file
fn save_latencies(infile: String, outfile: Option<String>) -> io::Result<()>{
    let urls = match get_urls(&infile) {
        Ok(u) => u,
        Err(err) => {
            error!("Unable to open file: {}. {}", infile, err);
            process::exit(1);
        }
    };
    let results : Vec<Latency> = urls.iter()                                      // Get iterator
                                     .map(httplatency::canonicalize_http_address) // Make sure all urls contain a scheme
                                     .filter_map(|s| s)                           // Remove all None options
                                     .map(|s| httplatency::get_latency(&s))       // Make all requests and time measurements
                                     .filter_map(|s| s)                           // Remove all None options
                                     .collect();                                  // Convert to Vec for serialization
    debug!("All HTTP requests complete");
    let outfilename = match outfile {
        Some(f) => f,
        None => DEFAULT_OUTPUT.to_string()
    };
    let mut out = try!(File::create(&outfilename));
    debug!("Writing output to {}", outfilename);
    let json = format!("{}\n", json::as_pretty_json(&results));
    out.write_all(json.as_bytes())
}

/// Given a file will return all the lines as a vector
///
/// Given a filename this function will try to open the file and then return each line one element
/// of a vector.
///
/// # Failures
///
/// This function will return an Err if the file cannot be opened. (e.g. bad permissions, missing
/// file etc.)
fn get_urls(filename: &String) -> Result<Vec<String>, String> {
    let file = try!(File::open(filename).map_err(|e| e.to_string()));
    let reader = io::BufReader::new(file);
    Ok(reader.lines().map(|l| l.unwrap()).collect())
}
