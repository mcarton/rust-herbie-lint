use std;
use std::borrow::Cow;
use std::fs::File;
use std::io::{Error as IOError, Read};
use toml;

// That's the default seed used by the Haskell plugin
const DEFAULT_HERBIE_SEED: &'static str = "#(1461197085 2376054483 1553562171 1611329376 \
                                           2497620867 2308122621)";
const DEFAULT_DB_PATH: &'static str = "Herbie.db";
const DEFAULT_TIMEOUT: u32 = 120;

#[derive(Debug, RustcDecodable)]
pub struct UxConf {
    /// Path to the database. Defaults to "Herbie.db".
    pub db_path: Option<String>,
    /// The seed use by Herbie. If not provided, a fixed seed will be used. Fixing the seed ensures
    /// deterministic builds.
    pub herbie_seed: Option<String>,
    /// Maximum time in seconds that Herbie is allowed to play with an expression. If null, allow
    /// Herbie to run indefinitely. Default is two minutes.
    pub timeout: Option<u32>,
    /// Allow the plugin to call Herbie on unknown expressions. Positive results from Herbie will
    /// be cached in the database.
    /// If ‘true’, the plugin will fail if it cannot find the executable.
    /// If ‘false’, the plugin will not try to run Herbie.
    /// By default, the plugin will call the executable only if it's found, but won't complain
    /// otherwise.
    pub use_herbie: Option<bool>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum UseHerbieConf {
    Default,
    No,
    Yes,
}

#[derive(Debug)]
pub struct Conf {
    pub db_path: Cow<'static, str>,
    pub herbie_seed: Cow<'static, str>,
    pub timeout: Option<u32>,
    pub use_herbie: UseHerbieConf,
}

impl Default for Conf {
    fn default() -> Conf {
        Conf {
            db_path: DEFAULT_DB_PATH.into(),
            herbie_seed: DEFAULT_HERBIE_SEED.into(),
            timeout: Some(DEFAULT_TIMEOUT),
            use_herbie: UseHerbieConf::Default,
        }
    }
}

impl From<UxConf> for Conf {
    fn from(ux: UxConf) -> Conf {
        Conf {
            db_path: ux.db_path.map_or(DEFAULT_DB_PATH.into(), Into::into),
            herbie_seed: ux.herbie_seed.map_or(DEFAULT_HERBIE_SEED.into(), Into::into),
            timeout: ux.timeout.map_or(Some(DEFAULT_TIMEOUT), |t| {
                if t == 0 {
                    None
                }
                else {
                    Some(t)
                }
            }),
            use_herbie: ux.use_herbie.map_or(UseHerbieConf::Default, |u| {
                if u {
                    UseHerbieConf::Yes
                }
                else {
                    UseHerbieConf::No
                }
            }),
        }
    }
}

#[derive(Debug)]
pub enum ConfError {
    Io {
        error: IOError,
    },
    Parse,
}

impl std::fmt::Display for ConfError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match *self {
            ConfError::Io{ ref error } => write!(f, "Error reading Herbie.toml: {}", error),
            ConfError::Parse => write!(f, "Syntax error in Herbie.toml"),
        }
    }
}

impl From<IOError> for ConfError {
    fn from(err: IOError) -> ConfError {
        ConfError::Io { error: err }
    }
}

pub fn read_conf() -> Result<Conf, ConfError> {
    if let Ok(mut conf) = File::open("Herbie.toml") {
        let mut buffer = String::new();
        try!(conf.read_to_string(&mut buffer));

        if let Some(conf) = toml::decode_str::<UxConf>(&buffer) {
            Ok(conf.into())
        }
        else {
            Err(ConfError::Parse)
        }
    }
    else {
        Ok(Conf::default())
    }
}
