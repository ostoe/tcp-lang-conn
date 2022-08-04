use std::net::{TcpListener, SocketAddr, TcpStream};
use std::time::Duration;
use std::thread;
use crate::check_unit::{stream_rw_unit, check_unit};
use std::str::FromStr;
use crate::check_status::CheckError;


pub fn start_server(addr_port: &str) {
    // addr
    let listener = TcpListener::bind(addr_port).expect("bind failed!");
    println!("Listener started");
    let check_interval = Duration::from_millis(200);
    for stream  in listener.incoming() {
        let mut stream: TcpStream = stream.unwrap();
        thread::spawn(move || {
            if !stream_rw_unit(&mut stream, true,  0).0 { return; };
            // thread::sleep(Duration::from_secs(5));
            let start_time = std::time::Instant::now();
            let default_addr = SocketAddr::from_str("127.0.0.0:8001").unwrap();
            loop {
                thread::sleep(check_interval);
                let check_result = check_unit(&mut stream, check_interval, start_time);
                match check_result.check_error {
                    CheckError::FIN => {
                        println!("[FIN] {} connection duration time: {:?}-------",
                                 check_result.addr.unwrap_or(default_addr), check_result.probe_time);
                        drop(stream); return; },
                    CheckError::RESET => {
                        println!("[RESET] {} connection duration time: {:?}-------",
                                 "miss addr", check_result.probe_time);
                        drop(stream); return;
                    },
                    CheckError::EAGAIN | CheckError::Readed => {} // checking...
                    CheckError::OtherErrno(e) => {
                        println!("[check] others errno error: {:?}", e);
                        drop(stream); return;
                    }
                    CheckError::ReadWriteError(e) => {
                        println!("[check] others read error: {:?}", e.kind());
                        drop(stream); return;
                    }
                    _ => {}
                }
            }

        });

    }

}