#![feature(await_macro, async_await)]
#![feature(futures_api)]

extern crate byteorder;
extern crate bytes;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate futures;
extern crate httparse;
#[macro_use]
extern crate log;
extern crate net2;
#[macro_use]
extern crate nom;
#[macro_use] extern crate structopt;
#[macro_use]
extern crate tokio;
extern crate tokio_io;
extern crate treebitmap;
extern crate trust_dns;
extern crate trust_dns_resolver;
extern crate env_logger;
extern crate core;

use futures::future;
use futures::Future;
use structopt::StructOpt;
use tokio::runtime::Runtime;

mod relay;
mod resolver;
mod conf;
mod cmd_options;
pub mod util;

use crate::relay::run_with_conf;
use crate::conf::load_conf;

pub fn run()-> Result<(), i32> {
    env_logger::Builder::from_default_env()
        .default_format_timestamp(false)
        .init();
    let opt = cmd_options::Opt::from_args();
    let config_path = &opt.config;
    if !config_path.is_dir() {
        error!("The given configuration directory doesn't exist");
        return Err(99);
    }

    let conf = match load_conf(&config_path) {
        Ok(x) => x,
        Err(e) => {
            error!("Error in config: {}", e);
            return Err(100);
        }
    };
    let mut rt = Runtime::new().expect("Tokio Runtime failed to run");
    rt.spawn( future::lazy(move || {
        for r in conf.relays {
            info!("Starting {}", r);
            if let Err(e) = run_with_conf(r,
                                          conf.domain_matcher.clone(),
                                          conf.ip_matcher.clone(),
            ) {
                error!("Relay error: {:?}", e);
            }
        }

        if let Some(dns) = conf.dns {
            info!("Starting dns proxy");
            let ds = resolver::serve(dns, conf.domain_matcher.clone());
            if let Err(e) = ds {
                error!("Dns server error: {:?}", e);
            }
        }
        Ok::<_, ()>(())
    }));
    rt.shutdown_on_idle().wait().expect("Can't wait tokio runtime");
    Ok(())
}
