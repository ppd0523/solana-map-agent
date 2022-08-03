use std::io::Read;
use std::thread;
use std::time;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::atomic::{
    AtomicUsize, Ordering,
};
use env_logger;
use log::{
    LevelFilter, trace, debug, info, warn, error,
};
use clap::{App, Arg};
use zmq::Context;


fn main() {
    let env = env_logger::Env::default()
            .filter_or("LOG_LEVEL", "trace");

    env_logger::Builder::from_env(env)
            .init();

    let args = App::new("agent")
            .version(env!("CARGO_PKG_VERSION"))
            .about(env!("CARGO_PKG_DESCRIPTION"))
            .arg(
                Arg::with_name("limit")
                        .help("max request LIMIT per period. default 10 [req]")
                        .takes_value(true)
                        .required(false)
            )
            .arg(
                Arg::with_name("period")
                        .help("max request limit per PERIOD. default 40 [sec]")
                        .takes_value(true)
                        .required(false)
            )
            .get_matches();

    let limit: usize = args.value_of("limit")
            .unwrap_or("10")
            .parse::<usize>()
            .unwrap_or(10);
    let period: u64 = args.value_of("period")
            .unwrap_or("40")
            .parse::<u64>()
            .unwrap_or(40);

    trace!("{} {}", limit, period);

    let ctx = Context::new();
    let url_receiver = ctx.socket(zmq::SocketType::PULL)
            .expect("Failed to create zmq push socket");
    if let Err(e) = url_receiver.connect("tcp://localhost:55551") {
        error!("{e}");
    }

    let (tx, rx) = mpsc::channel::<String>();
    let limit_producer = Arc::new(AtomicUsize::new(limit));
    let limit_consumer = limit_producer.clone();

    let hConsumer = thread::spawn(move || {
        let mut cnt = 0;

        loop {
            let raw = url_receiver.recv_bytes(0).expect("zmq receive failed");
            trace!("recv {cnt:03}: {raw:?}");
            while limit_consumer.load(Ordering::Acquire) == 0 {}
            limit_consumer.fetch_sub(1, Ordering::Release);

            // request to url
            let received = String::from_utf8_lossy(raw.as_slice());
            debug!("utf8: {}", received);

            unsafe {
                cnt += 1;
            }
        }
    });

    let hProducer = thread::spawn(move || {
        loop {
            thread::sleep(time::Duration::from_secs(period));
            limit_producer.store(limit, Ordering::Release);
            trace!("Update limit")
        }
    });

    loop {
        let mut buff = String::new();
        let c = std::io::stdin().read_line(&mut buff).unwrap();
        if buff.starts_with("q") {
            break;
        }

        tx.send(buff.clone()).unwrap();
    }
}
