extern crate hyper;
extern crate rustc_serialize;
extern crate time;
#[macro_use]
extern crate log;
extern crate url;

use std::error::Error;
use std::convert::AsRef;

use url::Url;
use url::SchemeData::{Relative, NonRelative};

use hyper::Client;
use hyper::client::IntoUrl;
use hyper::header::{Connection, UserAgent};

/// A Latency records the site which it is measuring and the latency of that site in milliseconds
#[derive(RustcEncodable, Debug, Clone)]
pub struct Latency {
    /// The url of the website being tested
    pub url: String,

    /// The time in milliseconds it took to retrieve ``url``
    pub latency_ms: i64, // convert to Option<i32> ?
}

/// Checks that a url is a valid http or https uri
///
/// # Examples
///
/// ```
/// let foo = httplatency::valid_http_url("www.google.com".to_string());
/// assert!(foo.is_none());
/// ```
///
/// ```
/// let bar = httplatency::valid_http_url("http://www.google.com".to_string());
/// assert_eq!(bar.unwrap(), "http://www.google.com".to_string());
/// ```
///
/// ```
/// let baz = httplatency::valid_http_url("https://www.google.com".to_string());
/// assert_eq!(baz.unwrap(), "https://www.google.com".to_string());
/// ```
///
/// ```
/// let qux = httplatency::valid_http_url("ftp://www.google.com".to_string());
/// assert!(qux.is_none());
/// ```
pub fn valid_http_url(s: String) -> Option<String> {

    let url = match s.into_url() {
        Ok(s) => s,
        Err(err) => {
            error!("Failed to parse url {}: {}", s, err.description());
            return None;
        }
    };

    if url.scheme=="http" || url.scheme=="https"{
        debug!("Valid url: {}", url.serialize());
        return Some(s);
    } else {
        error!("URL {} contains invalid scheme", s);
        return None;
    }
}

/// Given a HTTP(S) URL returns a fully qualified version
///
/// If ``s`` is already a fully qualified http(s) address, returns s.
/// If ``s`` is explicitly not http(s) or is not a parseable URL, returns None.
/// If ``s`` is missing a scheme will add http scheme unless port 443 is explicitly specified.
///
/// # Examples
///
/// ```
/// let foo = httplatency::canonicalize_http_address(&"www.google.com".to_string());
/// assert_eq!(foo.unwrap(), "http://www.google.com".to_string());
/// ```
///
/// ```
/// let foo = httplatency::canonicalize_http_address(&"http://www.google.com".to_string());
/// assert_eq!(foo.unwrap(), "http://www.google.com/".to_string());
/// ```
///
/// ```
/// let foo = httplatency::canonicalize_http_address(&"www.google.com:443".to_string());
/// assert_eq!(foo.unwrap(), "https://www.google.com:443".to_string());
/// ```
///
/// ```
/// let foo = httplatency::canonicalize_http_address(&"www.google.com:8080".to_string());
/// assert_eq!(foo.unwrap(), "http://www.google.com:8080".to_string());
/// ```
///
/// ```
/// let foo = httplatency::canonicalize_http_address(&"ftp://www.google.com".to_string());
/// assert!(foo.is_none());
/// ```
pub fn canonicalize_http_address(s: &String) -> Option<String> {
    /* Hyper's URL parser will treat a missing scheme as invalid.
       If the port is 443, we insert "https" as the scheme, otherwise we inster "http"
    */
    match s.into_url(){
        Ok(u) => canonicalize_http_url(u),
        Err(_) => {
            // IntoUrl borks at missing schemes, so...
            // we trick IntoUrl by giving the address a fake scheme
            let url = format!("fake://{}", s).into_url();
            match url {
                Ok(v) => {
                    // Having tricked IntoUrl into parsing we now set scheme back to ""
                    let u = Url {
                        scheme: "".to_string(),
                        scheme_data: v.scheme_data,
                        query: v.query,
                        fragment: v.fragment,
                    };
                    canonicalize_http_url(u)
                },
                Err(e) => {
                    warn!("Unable to parse URL: {} ({})", s, e);
                    None
                }
            }
        },
    }
}

fn canonicalize_http_url(url: Url) -> Option<String> {
    match url.scheme.as_ref() {
        "http" | "https" => Some(url.serialize()),
        _ => match url.scheme_data {
            // Scheme set to something other than "http" or "https" AND
            // scheme_data == Relative - means a scheme other than "http" or "https" was specified
            Relative(_) => None,
            // Scheme set to something other than "http" or "https" AND
            // scheme_data == NonRelative - just means there is no scheme
            // In this case IntoUrl will put the domain in url.scheme so we can't
            // just check it's empty
            NonRelative(ref port) => match port.parse::<i32>() {
                Ok(443) => Some(format!("https://{}", url.serialize())),   // we assume any port other than 443 is http
                Ok(_)   => Some(format!("http://{}", url.serialize())),
                Err(_)  => {
                    let mut p = port.clone();
                    p.truncate(port.len()-1);
                    match p.parse::<i32>() {  // Sometimes the port will have a trailing slash. Let's remove it and try to match again
                        Ok(443) => Some(format!("https://{}", url.serialize())),
                        Ok(_)   => Some(format!("http://{}", url.serialize())),
                        _       => Some(format!("http{}", url.serialize())),   // we assume any port other than 443 is http
                    }
                }
            }
        }
    }
}

/// Makes a HTTP GET request for the given site
///
/// # Panics
///
/// This function panics when:
///  * given a url with a domain that can't be resolved, or
///  * given an invalid url
///
/// These panics are generated from within hyper. Unfortunately stable versions of rust have no way of
/// catching this panic.
///
/// # Failures
///
/// If a webserver holds the connection open, this function will block until the full repsonse is received.
fn fetch_url(url: &String) {
    // Create a client.
    let client = Client::new();
    // Creating an outgoing request.
    client.get(url)
        // set a header
        .header(Connection::close())
        // set a fake user agent
        .header(UserAgent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_10_5) AppleWebKit/537.36 \
                          (KHTML, like Gecko) Chrome/47.0.2526.106 Safari/537.36".to_string()))
        // let 'er go!
        .send().unwrap();
}

/// Requests the given url measuring the time taken and returning a Result
///
/// # Panics
///
/// This function panics when:
///  * given a url with a domain that can't be resolved, or
///  * given an invalid url
///
/// These panics are generated from within hyper. Unfortunately stable versions of rust have no way of
/// catching this panic.
///
/// # Failures
///
/// If a webserver holds the connection open, this function will block until the full
/// repsonse is received.
///
/// # Examples
/// ```
/// // Should get the site in >0 milliseconds
/// let foo = httplatency::record_latency(&"http://www.google.com".to_string()).unwrap();
/// assert!(foo.latency_ms > 0)
/// ```
///
/// ```should_panic
/// // Hyper will panic because this isn't a real url
/// let foo = httplatency::record_latency(&"www.google.com".to_string());
/// ```
///
/// ```should_panic
/// // Hyper will panic because this isn't a real url
/// let bar = httplatency::record_latency(&"abcdefgh".to_string());
/// ```
///
/// ```should_panic
/// // Hyper will panic because this URL isn't resolveable
/// let baz = httplatency::record_latency(&"thisdomainisgarbage-hfgvjfhgdkjhdsfjhgsdjh.com".to_string());
/// ```
pub fn record_latency(s: &String) -> Result<Latency, String>  {
    let start = time::now();
    fetch_url(&s);
    let duration = (time::now() - start).num_milliseconds();
    return Ok( Latency {url: s.clone(), latency_ms: duration} );
}

/// Requests the given URL measuring the time taken and returning an Option
///
/// # Panics
///
/// This function panics when:
///  * given a url with a domain that can't be resolved, or
///  * given an invalid url
///
/// These panics are generated from within hyper. Unfortunately stable versions of rust have no way of
/// catching this panic.
///
/// # Failures
///
/// If a webserver holds the connection open, this function will block until the full
/// repsonse is received.
///
/// # Examples
/// ```should_panic
/// // Hyper will panic because this URL doesn't have a scheme
/// let foo = httplatency::get_latency(&"www.google.com".to_string()).unwrap();
/// ```
///
/// ```should_panic
/// // Hyper will panic because this isn't a real URL
/// let foo = httplatency::get_latency(&"abcdefgh".to_string());
/// ```
///
/// ```should_panic
/// // Hyper will panic because this URL isn't resolveable
/// let bar = httplatency::get_latency(&"thisdomainisgarbage-hfgvjfhgdkjhdsfjhgsdjh.com".to_string());
/// ```
pub fn get_latency(site: &String) -> Option<Latency> {
    info!("Testing {}", site);
    let lat = record_latency(site);
    if lat.is_ok() {
        lat.ok()
    } else {
        error!("Couldn't retrieve {}: {}", site, lat.unwrap_err());
        None
    }
}


#[cfg(test)]
mod test {
    use super::*;

    /************* record_latency **************/

    #[test]
    #[should_panic]
    /// Should panic and fail to get google because of missing scheme
    /// Known issue within the hyper library
    fn record_google_no_scheme() {
        record_latency(&"www.google.com".to_string()).is_err();
    }

    #[test]
    /// Should fetch google over http
    fn record_http_google() {
        let lat = record_latency(&"http://www.google.com".to_string());
        assert!(lat.is_ok(), "Failed to get google (http)");
    }

    #[test]
    /// Should fetch google over http and check for sane latency
    fn record_http_google_latency() {
        let lat = record_latency(&"http://www.google.com".to_string());
        assert!((lat.unwrap().latency_ms > 0), "Failed to get latency for google");
    }

    #[test]
    /// Should fetch google over https
    fn record_https_google() {
        let lat = record_latency(&"https://www.google.com".to_string());
        assert!(lat.is_ok(), "Failed to get google (https)");
    }

    #[test]
    /// Should fetch google with query string
    fn record_https_google_with_query_string() {
        let lat = record_latency(&"https://www.google.com.au/?q=rust+lang".to_string());
        assert!(lat.is_ok(), "Failed to get google (with query string)");
    }

    /************* get_latency **************/


    #[test]
    #[should_panic]
    /// Should panic and fail to get google because of missing scheme
    /// Known issue within the hyper library
    fn get_google_no_scheme() {
        get_latency(&"www.google.com".to_string());
    }

    #[test]
    /// Should fetch google over http
    fn get_http_google() {
        let lat = get_latency(&"http://www.google.com".to_string());
        assert!(lat.is_some(), "Failed to get google (http)");
    }

    #[test]
    /// Should fetch google over http and check for sane latency
    fn get_http_google_latency() {
        let lat = get_latency(&"http://www.google.com".to_string());
        assert!((lat.unwrap().latency_ms > 0), "Failed to get latency for google");
    }

    #[test]
    /// Should fetch google over https
    fn get_https_google() {
        let lat = get_latency(&"https://www.google.com".to_string());
        assert!(lat.is_some(), "Failed to get google (https)");
    }

    #[test]
    /// Should fetch google with query string
    fn get_https_google_with_query_string() {
        let lat = get_latency(&"https://www.google.com.au/?q=rust+lang".to_string());
        assert!(lat.is_some(), "Failed to get google (with query string)");
    }

    /************* canonicalize_http_address **************/

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn add_scheme() {
        let url = super::canonicalize_http_address(&"www.google.com".to_string());
        assert_eq!(url.unwrap(), "http://www.google.com")
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn add_scheme_trailing_slash() {
        let url = super::canonicalize_http_address(&"www.google.com/".to_string());
        assert_eq!(url.unwrap(), "http://www.google.com/")
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn add_scheme_port_80() {
        let url = super::canonicalize_http_address(&"www.google.com:80".to_string());
        assert_eq!(url.unwrap(), "http://www.google.com:80")
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn add_scheme_port_80_trailing_slash() {
        let url = super::canonicalize_http_address(&"www.google.com:80/".to_string());
        assert_eq!(url.unwrap(), "http://www.google.com:80/")
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn add_scheme_port_8080() {
        let url = super::canonicalize_http_address(&"www.google.com:8080".to_string());
        assert_eq!(url.unwrap(), "http://www.google.com:8080")
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn add_scheme_port_8080_trailing_slash() {
        let url = super::canonicalize_http_address(&"www.google.com:8080/".to_string());
        assert_eq!(url.unwrap(), "http://www.google.com:8080/")
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn add_scheme_port_443() {
        let url = super::canonicalize_http_address(&"www.google.com:443".to_string());
        assert_eq!(url.unwrap(), "https://www.google.com:443")
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn add_scheme_port_443_trailing_slash() {
        let url = super::canonicalize_http_address(&"www.google.com:443/".to_string());
        assert_eq!(url.unwrap(), "https://www.google.com:443/")
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn http_canonicalized() {
        let url = super::canonicalize_http_address(&"http://www.google.com".to_string());
        assert_eq!(url.unwrap(), "http://www.google.com/")
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn http_canonicalized_trailing_slash() {
        let url = super::canonicalize_http_address(&"http://www.google.com/".to_string());
        assert_eq!(url.unwrap(), "http://www.google.com/")
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn http_canonicalized_port_80() {
        let url = super::canonicalize_http_address(&"http://www.google.com:80".to_string());
        assert_eq!(url.unwrap(), "http://www.google.com/")
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn http_canonicalized_port_80_trailing_slash() {
        let url = super::canonicalize_http_address(&"http://www.google.com:80/".to_string());
        assert_eq!(url.unwrap(), "http://www.google.com/")
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn http_canonicalized_port_8080() {
        let url = super::canonicalize_http_address(&"http://www.google.com:8080".to_string());
        assert_eq!(url.unwrap(), "http://www.google.com:8080/")
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn http_canonicalized_port_8080_trailing_slash() {
        let url = super::canonicalize_http_address(&"http://www.google.com:8080/".to_string());
        assert_eq!(url.unwrap(), "http://www.google.com:8080/")
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn https_canonicalized_port_443() {
        let url = super::canonicalize_http_address(&"https://www.google.com:443".to_string());
        assert_eq!(url.unwrap(), "https://www.google.com/")
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn https_canonicalized_port_443_trailing_slash() {
        let url = super::canonicalize_http_address(&"https://www.google.com:443/".to_string());
        assert_eq!(url.unwrap(), "https://www.google.com/")
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn https_canonicalized_port_80() {
        let url = super::canonicalize_http_address(&"https://www.google.com:80".to_string());
        assert_eq!(url.unwrap(), "https://www.google.com:80/")
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn https_canonicalized_port_80_trailing_slash() {
        let url = super::canonicalize_http_address(&"https://www.google.com:80/".to_string());
        assert_eq!(url.unwrap(), "https://www.google.com:80/")
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn https_canonicalized_port_8080() {
        let url = super::canonicalize_http_address(&"https://www.google.com:8080".to_string());
        assert_eq!(url.unwrap(), "https://www.google.com:8080/")
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn https_canonicalized_port_8080_trailing_slash() {
        let url = super::canonicalize_http_address(&"https://www.google.com:8080/".to_string());
        assert_eq!(url.unwrap(), "https://www.google.com:8080/")
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn http_canonicalized_port_443() {
        let url = super::canonicalize_http_address(&"http://www.google.com:443".to_string());
        assert_eq!(url.unwrap(), "http://www.google.com:443/")
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn http_canonicalized_port_443_trailing_slash() {
        let url = super::canonicalize_http_address(&"http://www.google.com:443/".to_string());
        assert_eq!(url.unwrap(), "http://www.google.com:443/")
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn non_http_canonicalized_trailing_slash() {
        let url = super::canonicalize_http_address(&"ftp://www.google.com/".to_string());
        assert!(url.is_none())
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn non_http_canonicalized_port_21_trailing_slash() {
        let url = super::canonicalize_http_address(&"ftp://www.google.com:21/".to_string());
        assert!(url.is_none())
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn non_http_canonicalized() {
        let url = super::canonicalize_http_address(&"ftp://www.google.com".to_string());
        assert!(url.is_none())
    }

    #[test]
    /// Should add 'http://' to URL with missing scheme
    /// More strict requirement than missing_scheme test
    fn non_http_canonicalized_port_21() {
        let url = super::canonicalize_http_address(&"ftp://www.google.com:21".to_string());
        assert!(url.is_none())
    }

    /************* valid_http_url **************/

    #[test]
    /// Won't accept URLs with missing scheme
    fn missing_scheme() {
        let url = super::valid_http_url("www.google.com".to_string());
        assert!(url.is_none(), "Hyper's IntoUrl trait is magically accepting missing schemes now");
    }

    #[test]
    /// Should error on non-http scheme
    fn bad_scheme() {
        let url = super::valid_http_url("ftp://ftp.iinet.net.au/".to_string());
        assert!(url.is_none(), "ftp scheme should be rejected")
    }

    #[test]
    /// Should treat scheme as case insensitive
    fn capitalised_scheme() {
        let url = super::valid_http_url("HTTP://www.google.com".to_string());
        assert!(url.is_some(), "Cannot handle capitalised schemes");
    }

    #[test]
    /// Hyper currently considers this a valid url
    fn bad_scheme_delimiter() {
        let url = super::valid_http_url("http:/www.google.com".to_string());
        assert!(url.is_some(), "Hyper now rejects bad schema delimiters")
    }

    /************* fetch_url **************/

    #[test]
    #[should_panic]
    /// Currently passes because hyper cannot resolve the domain and panic!s
    /// In future we could try to resolve the domain in a separate step and 
    /// then pass it to hyper only if successful - almost as a guard.
    /// This would be susceptible to an (unlikely) race condition however where
    /// ther domain becomes unresolvable between our guard and the call to hyper.
    fn nonexistant_domain() {
        super::fetch_url(&"http://ksdjfghlkdfsjhgfdskjghfdg.com".to_string());
    }
    /*
     *
     */
}
