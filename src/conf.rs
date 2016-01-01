use std::borrow::Cow;
use std::fs::File;
use std::io::Read;
use toml;

const DEFAULT_HERBIE_SEED : &'static str = "#(1461197085 2376054483 1553562171 1611329376 2497620867 2308122621)";
const DEFAULT_DB_PATH : &'static str = "Herbie.db";
const DEFAULT_TIMEOUT : u32 = 120;

#[derive(Debug, RustcDecodable)]
pub struct UxConf {
    /// Path to the database.
    pub db_path: Option<String>,
    /// Maximum time in seconds that Herbie is allowed to play with an expression. If null, allow
    /// The seed use by Herbie. If not provided, a fixed seed will be used. Fixing the seed ensures
    /// deterministic builds.
    pub herbie_seed: Option<String>,
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
            timeout: ux.timeout.map_or(Some(DEFAULT_TIMEOUT), |t| if t == 0 { None } else { Some(t) }),
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

pub fn read_conf() -> Conf {
    let conf = {
        let mut conf = if let Ok(conf) = File::open("Herbie.toml") {
            conf
        }
        else {
            return Conf::default();
        };
        let mut buffer = String::new();
        conf.read_to_string(&mut buffer).unwrap();
        buffer
    };

    let conf : UxConf = toml::decode_str(&conf).unwrap();
    conf.into()
}
