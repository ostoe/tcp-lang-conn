use std::net::{TcpListener, SocketAddr, TcpStream};
use std::time::Duration;
use std::thread;
use crate::check_unit::{stream_rw_unit, check_unit};
use std::str::FromStr;
use crate::check_status::CheckError;

use crossbeam_channel::{unbounded, RecvError, tick, select};
use std::io::Write;


pub fn start_server(addr_port: &str) { // -> Result<(), Box<dyn std::error::Error>> {
    // addr
    let listener = TcpListener::bind(addr_port).expect("bind failed!");
    println!("Listener started");
    let check_interval = Duration::from_millis(100);
    let (st, rt) = unbounded::<bool>();
    // count thread;
    thread::spawn(move|| {
        let ticker = tick(Duration::from_millis(100));
        let mut not_release_tcpstream = 0u32;
        let mut stdout = std::io::stdout();
        loop {
            select! {
                recv(ticker) -> _ => {
                    print!("\rcurrent connection: \x1b[40;32m{}\x1b[0m", not_release_tcpstream);
                    stdout.flush().unwrap();
                },
                recv(rt) -> result => {
                        match result {
                    Ok(r) => {
                        if r {not_release_tcpstream += 1;} else {not_release_tcpstream -= 1;}
                    },
                    Err(RecvError) => {break;}
                    }
                }
            }
        }
    });
    for stream  in listener.incoming() {
        let mut stream: TcpStream = stream.unwrap();
        st.send(true).unwrap();
        let st_c = st.clone();
        thread::spawn(move || {
            // first
            if !stream_rw_unit(&mut stream, true,  0).0 {
                 st_c.send(false).unwrap(); return; 
            };
            // thread::sleep(Duration::from_secs(5));
            let start_time = std::time::Instant::now();
            let default_addr = SocketAddr::from_str("127.0.0.0:8001").unwrap();
            loop {
                thread::sleep(check_interval);
                let check_result = check_unit(&mut stream, start_time);
                match check_result.check_error {
                    CheckError::FIN => {
                        println!("[FIN] {} connection duration time: {:?}-------",
                                 check_result.addr.unwrap_or(default_addr), check_result.probe_time.unwrap_or(Default::default()));
                    },
                    CheckError::RESET => {
                        println!("[RESET] {} connection duration time: {:?}-------",
                                 "miss addr", check_result.probe_time.unwrap_or(Default::default()));
                    },
                    CheckError::EAGAIN | CheckError::Readed => {continue;} // checking...
                    CheckError::OtherErrno(e) => {
                        println!("[check] others errno error: {:?}", e);
                    }
                    CheckError::ReadWriteError(e) => {
                        println!("[check] others read error: {:?}", e.kind());
                    }
                    _x => { println!("____");continue; }
                }
                drop(stream);
                st_c.send(false).unwrap();
                return;
            }

        });

    }

}