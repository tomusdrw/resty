//! A macro to simplify parameters parsing.

#[macro_export]
macro_rules! url {
    ($($tail:tt)+) => {
        url_internal!(
            { $($tail)+ }
            ,
            no_params
            ,
            ""
            ;
            ;
        )
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! url_internal {
    ( { } , $done:ident, $prefix:expr ; $($param:ident : $type:ty,)* ; $($data:tt)* ) => {{
        #[derive(Default, Debug, Clone)]
        struct MyParams {
            $($param: $type,)*
        }

        impl $crate::request::params::Parser for MyParams {
            type Params = MyParams;

            fn parse(&self, uri: &$crate::Uri, skip: usize) -> Result<Self::Params, $crate::request::params::Error> {
                let mut it = uri.path()[skip..].split('/');
                // Skip leading slash.
                it.next();
                parser!(it, $($data)*);
                Ok(MyParams { $($param,)* })
            }
        }

        $crate::request::Params {
            parser: MyParams::default(),
            prefix: $prefix,
        }
    }};
    ({ /$p:ident$(/$tail:tt)* } , has_params, $prefix:expr ; $($param:ident : $type:ty,)*; $($data:tt)*) => {
        url_internal!(
            { $(/$tail)* }
            ,
            has_params
            ,
            concat!($prefix, "/", stringify!($p))
            ;
            $($param : $type,)*
            ;
            $($data)*
            segment $p,
        )
    };
    ({ /$p:ident$(/$tail:tt)* } , no_params, $prefix:expr ; $($param:ident : $type:ty,)*; $($data:tt)*) => {
        url_internal!(
            { $(/$tail)* }
            ,
            no_params
            ,
            concat!($prefix, "/", stringify!($p))
            ;
            $($param : $type,)*
            ;
            $($data)*
        )
    };
    ({ /{$p:ident : $t:ty}$(/$tail:tt)* } , $d:ident, $prefix:expr ; $($param:ident : $type:ty,)*; $($data:tt)*) => {
        url_internal!(
            { $(/$tail)* }
            ,
            has_params
            ,
            $prefix 
            ;
            $($param : $type,)*
            $p : $t,
            ;
            $($data)*
            param $p,
        )
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! parser {
    ($it:expr , ) => {};
    ($it:expr , segment $x:ident , $($tail:tt)*) => {
        let path = $it.next();
        match path {
            Some(stringify!($x)) => Ok(()),
            None => Err($crate::request::params::Error::NotFound),
            Some(other) => Err($crate::request::params::Error::InvalidSegment {
                got: other.into(),
                expected: stringify!($x).into()
            }),
        }?;
        parser!($it, $($tail)*);
    };
    ($it:expr , param $param:ident , $($tail:tt)*) => {
        let path = $it.next().ok_or_else(|| $crate::request::params::Error::NotFound)?;
        let $param = path.parse().map_err(|e| $crate::request::params::Error::InvalidType {
            param: stringify!($param).into(),
            path: path.into(),
            error: format!("{:?}", e),
        })?;
        parser!($it, $($tail)*);
    };
}

#[test]
fn url_parser() {
    use request::params::Parser;

    let url = url!(/v1/test/{id:usize}/{a:String});

    assert_eq!(url.prefix, "/v1/test");
    let uri = "http://localhost:3000/v1/test/5/3".parse().unwrap();
    let parsed = url.parser.parse(&uri, url.prefix.len()).unwrap();
    assert_eq!(parsed.id, 5);
    assert_eq!(parsed.a, "3".to_owned());
}
