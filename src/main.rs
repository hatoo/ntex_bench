use std::sync::{atomic::AtomicUsize, Arc};

use clap::Parser;
use futures::StreamExt;
use ntex::{
    http::{
        client::{error::SendRequestError, Client},
        ConnectionType,
    },
    rt::Arbiter,
};

#[derive(Parser)]
struct Opts {
    addr: String,
    #[clap(short = 'c', long, default_value = "500")]
    num_connection: usize,
    #[clap(short = 'n', long, default_value = "1000000")]
    num_works: usize,
}

#[ntex::main]
async fn main() -> Result<(), SendRequestError> {
    // std::env::set_var("RUST_LOG", "ntex=trace");
    // env_logger::init();

    let opts = Opts::parse();

    let counter = Arc::new(AtomicUsize::new(0));

    let cpus = num_cpus::get();

    let now = std::time::Instant::now();

    let arbiters = (0..cpus)
        .map(|i| {
            let arbiter = Arbiter::new();

            // Be careful about fractional
            let num_connection =
                opts.num_connection / cpus + (if opts.num_connection % cpus > i { 1 } else { 0 });
            (0..num_connection).for_each(|_| {
                let counter = counter.clone();
                let addr = opts.addr.clone();
                arbiter.spawn(Box::pin(async move {
                    ntex::rt::spawn(async move {
                        let client = Client::new();
                        while counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                            < opts.num_works
                        {
                            let mut response = client
                                .get(&addr)
                                .header("User-Agent", "ntex")
                                .set_connection_type(ConnectionType::KeepAlive)
                                .send()
                                .await
                                .unwrap();

                            while let Some(_) = response.next().await {}
                        }
                        Arbiter::current().stop();
                    });
                }));
            });
            arbiter
        })
        .collect::<Vec<_>>();

    for mut a in arbiters {
        a.join().unwrap();
    }

    let elapsed = now.elapsed();

    println!("elapsed: {:?}", elapsed);
    println!("rps: {}", opts.num_works as f64 / elapsed.as_secs_f64());

    Ok(())
}
