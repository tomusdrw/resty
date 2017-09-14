//! URL params.

use hyper;
use error;

/// Params definition
pub struct Params<'a, P: Parser = StdParser> {
    /// URL parser
    pub parser: P,
    /// Prefix
    pub prefix: &'a str,
}

impl<'a> Into<Params<'a>> for &'a str {
    fn into(self) -> Params<'a> {
        match self.find('{') {
            None => {
                Params {
                    parser: StdParser::default(),
                    prefix: self,
                }
            },
            Some(pos) => {
                let (prefix, params) = self.split_at(pos);
                Params {
                    parser: StdParser::params(params),
                    prefix,
                }
            }
        }
    }
}

/// Error while parsing URL for parameters.
#[derive(Debug, PartialEq)]
pub enum Error {
    /// A requested dynamic paremeter name was not declared.
    UnknownParameter(String),
    /// Expected segment or parameter was not found in the path.
    NotFound,
    /// Cannot parse path to expected type.
    InvalidType {
        /// Parameter name
        param: String,
        /// Path segment
        path: String,
        /// Parsing error
        error: String,
    },
    /// Unexpected path segment.
    InvalidSegment {
        /// Got
        got: String,
        /// Expected
        expected: String,
    }
}

impl From<Error> for error::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::UnknownParameter(param) => error::Error::internal(
                "Tried to access non-existent parameter. That's most likely a bug in the handler.",
                param,
            ),
            Error::InvalidType { param, path, error } => error::Error::bad_request(
                format!("Error while parsing parameter {:?} from {:?}", param, path),
                error
            ),
            Error::NotFound => error::Error::not_found(
                "The resource exists, but expects a parameter."
            ),
            Error::InvalidSegment { got, expected } => error::Error::not_found(
                format!("The resource exists, but the path is invalid. Got {:?}, expected {:?}", got, expected)
            ),
        }
    }
}

/// Params Parser
pub trait Parser: Send + Sync + 'static {
    /// Returned Parameters type.
    type Params;

    /// Returns number of expected params and param names.
    fn expected_params(&self) -> (usize, String);

    /// Parser URL and return params
    fn parse(&self, uri: &hyper::Uri, skip: usize) -> Result<Self::Params, Error>;
}

/// A standard parser which processes params dynamically.
#[derive(Debug, Default)]
pub struct StdParser {
    params: Vec<(usize, String)>,
    segments: Vec<(usize, String)>,
    expected: usize,
}

impl StdParser {
    /// Create new standard params and parse given string for params patterns.
    pub fn params(params: &str) -> Self {
        let mut it = params.split('/');
        let mut params = vec![];
        let mut segments = vec![];
        let mut pos = 0;

        while let Some(param) = it.next() {
            let len = param.len();
            if len > 0 && &param[0..1] == "{" && &param[len - 1..] == "}" {
                let name = &param[1 .. len-1];
                params.push((pos, name.to_owned()));
            } else {
                segments.push((pos, param.to_owned()));
            }
            pos += 1;
        }

        StdParser {
            params,
            segments,
            expected: pos,
        }
    }
}
impl Parser for StdParser {
    type Params = DynamicParams;

    fn expected_params(&self) -> (usize, String) {
        (self.expected, self.params.iter().fold(String::new(), |acc, param| acc + "/{" + &param.1 + "}"))
    }

    fn parse(&self, uri: &hyper::Uri, skip: usize) -> Result<Self::Params, Error> {
        let path = &uri.path()[skip..];
        if self.expected == 0 && !path.is_empty() {
            Err(Error::NotFound)
        } else {
            DynamicParams::validate(
                self.params.clone(),
                self.segments.clone(),
                path.into(),
            )
        }
    }
}

/// Dynamic parameters.
pub struct DynamicParams {
    params: Vec<(usize, String)>,
    path: String,
}

impl DynamicParams {
    /// Create new dynamic params and validate segment positions.
    pub fn validate(params: Vec<(usize, String)>, segments: Vec<(usize, String)>, path: String) -> Result<Self, Error> {
        {
            let mut it = path.split('/');
            let mut current_pos = 0;
            for (pos, segment) in segments {
                // Consume params
                while current_pos < pos {
                    it.next();
                    current_pos += 1;
                }

                current_pos += 1;
                // validate
                match it.next() {
                    Some(seg) if seg == &segment => {},
                    Some(seg) => return Err(Error::InvalidSegment {
                        expected: segment,
                        got: seg.into(),
                    }),
                    None => return Err(Error::NotFound),
                }
            }
        }

        Ok(DynamicParams {
            params,
            path,
        })
    }

    fn find(&self, name: &str) -> Result<usize, Error> {
        for &(pos, ref v) in &self.params {
            if v == name {
                return Ok(pos);
            }
        }

        Err(Error::UnknownParameter(name.into()))
    }

    /// Retrieve a string value of a parameter by given name.
    pub fn get_str(&self, name: &str) -> Result<&str, Error> {
        let pos = self.find(name)?;
        self.path.split('/').nth(pos).ok_or_else(|| Error::NotFound)
    }

    /// Retrieve a value of a parameter by given name.
    pub fn get<T>(&self, name: &str) -> Result<T, Error> where
        T: ::std::str::FromStr,
        T::Err: ::std::fmt::Debug,
    {
        let path = self.get_str(name)?;
        path.parse().map_err(|e| Error::InvalidType {
            param: name.into(),
            path: path.into(),
            error: format!("{:?}", e),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Params, Parser};

    #[test]
    fn should_parse_string_to_std_parser() {
        let params: Params = "/{id}".into();
        assert_eq!(params.prefix, "/");
        let uri = "http://localhost/5".parse().unwrap();
        let parsed = params.parser.parse(&uri, params.prefix.len()).unwrap();
        assert_eq!(parsed.get_str("id").unwrap(), "5");
        assert_eq!(parsed.get::<usize>("id").unwrap(), 5);
        assert_eq!(parsed.get::<f64>("id").unwrap(), 5.0f64);
        assert_eq!(parsed.get::<u32>("id").unwrap(), 5u32);
        assert_eq!(parsed.get::<u64>("id").unwrap(), 5u64);

        let params: Params = "/test/{id}".into();
        assert_eq!(params.prefix, "/test/");
        let uri = "http://localhost/test/5".parse().unwrap();
        let parsed = params.parser.parse(&uri, params.prefix.len()).unwrap();
        assert_eq!(parsed.get_str("id").unwrap(), "5");
        assert_eq!(parsed.get::<usize>("id").unwrap(), 5);
        assert_eq!(parsed.get::<f64>("id").unwrap(), 5.0f64);
        assert_eq!(parsed.get::<u32>("id").unwrap(), 5u32);
        assert_eq!(parsed.get::<u64>("id").unwrap(), 5u64);

        let params: Params = "/test/{id}/xxx".into();
        assert_eq!(params.prefix, "/test/");
        let uri = "http://localhost/test/5/xxx".parse().unwrap();
        let parsed = params.parser.parse(&uri, params.prefix.len()).unwrap();
        assert_eq!(parsed.get_str("id").unwrap(), "5");
        assert_eq!(parsed.get::<usize>("id").unwrap(), 5);
        assert_eq!(parsed.get::<f64>("id").unwrap(), 5.0f64);
        assert_eq!(parsed.get::<u32>("id").unwrap(), 5u32);
        assert_eq!(parsed.get::<u64>("id").unwrap(), 5u64);
    }
}
