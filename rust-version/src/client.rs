use std::time::{Duration, Instant};
use std::sync::mpsc;
use std::net::{TcpStream, SocketAddr};
use std::{thread, mem};
use nix::sys::socket::{setsockopt as nix_setsockopt, send, MsgFlags, recv };
use std::os::unix::io::AsRawFd;
use nix::errno;
use std::os::raw::{c_void, c_int};
use libc::setsockopt;
use crossbeam_channel::{Sender, Receiver, unbounded, tick, RecvError, TryRecvError, bounded};
#[cfg(target_family = "linux")]
use nix::sys::socket::sockopt::{TcpUserTimeout, KeepAlive};
use crate::check_unit::{stream_rw_unit, check_unit};
use crate::check_status::{Message, CheckError};
use std::str::FromStr;
use crossbeam_channel::internal::SelectHandle;
use std::ops::Index;
use std::thread::JoinHandle; // 参考nix sockopt 跨平台用法，再不行就用macro


pub enum Signal {
    Run(u32),
    Terminated
}

pub fn start_client(addr_port: &str) {

    // let addr_port: &str = "192.168.1.2:80018";
    // 起多个线程，做线程序列，[ 5min 10min 15m 30m 1h 2h 4h 8h 12h 18h 24h 28h 36h]
    // 如果正常断开，比如15分钟，那么在某一时间段，30m 1h 2h 4h 8h 12h 18h 24h 28h 36h] 这些连接都能收到reset包正常断开。
    // 如果不能正常，就按照时间序列探测。
    let mut threads_lists = [0u32; 255];
    let threads_lists_part1 = [15u32, 30, 5*60, 10*60];
    // todo
    for x in 0..threads_lists_part1.len() {
        threads_lists[x] = threads_lists_part1[x];
    }
    for x in 0..35 {
        threads_lists[x + threads_lists_part1.len()] = (x as u32) * 3600;
    }
    // let threads_lists = [15, 5u32*60, 10*60, 15*60, 30*60, 1*3600, 2*3600, 3*3600, 4*3600, 5*3600, 6*3600, 7*3600, 8* 3600,
    //     12 * 3600, 18* 3600, 24 * 3600, 28 * 3600, 36 * 3600, 7200*3600];
        // .map(|t| Duration::from_secs(t));

    let check_interval = Duration::from_millis(200);
    let addr_port_move = addr_port.to_string();
    let default_addr = SocketAddr::from_str("127.0.0.0:8001").unwrap();
    // 正常检测
    thread::spawn(move || {
        let mut stream = TcpStream::connect(addr_port_move).expect("connection failed!");
        // stream.set_write_timeout(Some(Duration::new(5, 0)));
        let (is_ok, e) = stream_rw_unit(&mut stream, false,  0);
        if !is_ok {
            println!("first read/wirte error, Exit");
            drop(stream); return;
        }
        let start_time = std::time::Instant::now();
        loop {
            let check_result = check_unit(&mut stream, check_interval, start_time);
            match check_result.check_error {
                CheckError::FIN => {
                    println!("[FIN] {} connection duration time: {:?}-------",
                             check_result.addr.unwrap_or(default_addr), check_result.probe_time);
                    drop(stream); std::process::exit(0); },
                CheckError::RESET => {
                    println!("[RESET] {} connection duration time: {:?}-------",
                             "miss addr", check_result.probe_time);
                    drop(stream); std::process::exit(0);;
                },
                CheckError::EAGAIN | CheckError::Readed => {} // checking...
                CheckError::OtherErrno(e) => {
                    println!("[check] others errno error: {:?}", e);
                    drop(stream); std::process::exit(0);;
                }
                CheckError::ReadWriteError(e) => {
                    println!("[check] others read error: {:?}", e.kind());
                    drop(stream); std::process::exit(0);;
                }
                _ => {}
            }

        }
    });


    let (ctrl_probe_st, ctrl_probe_rt) = unbounded::<Signal>();
    let (ctrl_sleep_st, ctrl_sleep_rt) = bounded::<bool>(2);
    let (probe_st, probe_rt) = unbounded::<(CheckError, u32, Duration)>();

    // probe list
    let ctrl_probe_rt_clone = ctrl_probe_rt.clone();
    let probe_st_clone = probe_st.clone();
    // 启动probe线程
    // let addr_port_c = addr_port.to_string();
    probe_timing_thread(addr_port, ctrl_probe_rt_clone, probe_st_clone, threads_lists.len());

    let threads_lists_copy = threads_lists.clone();
    // 定时，到时间以后通知
    let ctrl_sleep_rt_clone = ctrl_sleep_rt.clone();
    let ctrl_probe_st_clone = ctrl_probe_st.clone();
    // ------------------------ sleep thread
    let mut threads_lists_copy= [0u32; 255];
    for x in 0..threads_lists.len() {
        threads_lists_copy[x] = threads_lists[x];
    }
    sleep_timing_thread(threads_lists_copy, ctrl_sleep_rt_clone, ctrl_probe_st_clone, threads_lists.len());
    // 起一个沉睡线程////
    // let ticker = tick(Duration::from_secs(1));
    // let _ = ticker.recv();

    let mut has_probe_count = 0;
    loop {
        let probe_result = probe_rt.recv();
        has_probe_count += 1;
        let (e, thread_index, probe_time) = probe_result.unwrap_or_default();
        match e {
            CheckError::EAGAIN => {
                println!("[{}] {:?} \x1b[40;32mhas alive\x1b[0m [EAGAIN]", thread_index, probe_time);
                continue;
            }
            CheckError::RESET => { // [R]
                println!("[{}]: {:?} connection \x1b[41;36mclosed [R]\x1b[0m some connection was killed!", thread_index, probe_time);
                // then do recycle probe but now exit;
            }
            CheckError::FIN => { // [R] //never run....
                println!("[{}]: {:?} connection \x1b[41;36mclosed [FIN]\x1b[0m some connection was killed!", thread_index, probe_time);
                // then do recycle probe but now exit;
            }
            CheckError::TimedOUT => {
                println!("[{}]: {:?} \x1b[41;36m[TIMEOUT]\x1b[0m some connection was killed!", thread_index, probe_time);
                // then do recycle probe but now exit;
            }
            others => {
                println!("[{}]: {:?} probe check other error!", thread_index, probe_time);
                std::process::exit(0);}
        }


        ctrl_probe_st.send(Signal::Terminated);
        // anyway true or false
        ctrl_sleep_st.send(true);

        // todo recycle probe but now exit;
        let index_element = threads_lists
            .iter()
            .position(|&x| x == thread_index)
            .unwrap();
        let (mut a_time, mut b_time) = (0, 0);
        if index_element == 0 {
            println!("1 min 内断开");
            std::process::exit(0);
        } else {
             a_time = threads_lists[index_element-1];
             b_time = threads_lists[index_element];
            // send 第二轮探测
        }


    }

}

pub fn sleep_timing_thread(threads_lists: [u32; 255],ctrl_sleep_rt: Receiver<bool>,
                           ctrl_probe_st: Sender<Signal>,
                           threads_list_real_len: usize) -> JoinHandle<()> {
    // ------------------------ sleep thread
    let sleep_probe_thread =  thread::spawn(move || {
        let mut thread_index = 0;
        loop {
            if 0 <= thread_index && thread_index < threads_list_real_len  {
                let mut target_sleep_time= 0u32;
                if thread_index == 0 {
                    target_sleep_time = threads_lists[thread_index];
                } else {
                    target_sleep_time = threads_lists[thread_index] - threads_lists[thread_index-1];
                }
                thread::sleep(Duration::from_secs(target_sleep_time as u64));
                // 醒来看看自己是否要终止。
                if ctrl_sleep_rt.is_ready() {
                    ctrl_sleep_rt.recv();
                    break;  // Err(TryRecvError::Empty) => {}
                }
                // probe the connection
                ctrl_probe_st.send(Signal::Run(target_sleep_time));
                thread_index += 1;
            } else {
                println!("out probe time, the connection still alive. maybe never active disconnection."); // todo
                std::process::exit(0);
            }

        }
    });// ------------------------
    return sleep_probe_thread;
}

pub fn probe_timing_thread(addr_port: &str, ctrl_probe_rt: Receiver<Signal>, probe_st: Sender<(CheckError, u32, Duration)>, threads_lists_length: usize) {
    // let addr_port_move = addr_port;
    let addr_port_string: String = String::from(addr_port);
    thread::spawn( move || {
        let mut probe_stream: Vec<(TcpStream, Instant)> = Default::default();
        for thread_index in 0..threads_lists_length {
            let mut stream = TcpStream::connect(&addr_port_string).expect("connection failed!");
            // stream.set_write_timeout(Some(Duration::new(5, 0)));
            let (is_ok, e) = stream_rw_unit(&mut stream, false, thread_index);
            if !is_ok {
                println!("first read/wirte error, Exit");
                drop(stream); return;
            }
            let start_time = std::time::Instant::now();
            probe_stream.push((stream, start_time));
        }
        let mut thread_index = threads_lists_length;
        loop {
            match ctrl_probe_rt.recv() {
                Ok(s) => {
                    match s {
                        Signal::Run(probe_time) => {
                            let (stream, conn_start_time) = probe_stream.pop().unwrap();

                            let write_content = "heartbeat".as_bytes();
                            println!("------[{}]will probe------", thread_index);
                            stream.set_write_timeout(Some(Duration::new(1, 0))); // 无效参数，仅仅针对本地写到缓存，而不是完整的链路
                            // 根据平台区分

                            let tcp_user_timeout = 1u32; // 重传超时时间，不是次数，大概是发四次包的样子，0x18 tcp_user_timeout macos tcpxxx: 0x80
                            // 都可用，但是单位不一样，linux millens ref: https://man7.org/linux/man-pages/man7/tcp.7.html：
                            //  it specifies the maximum
                            //  amount of time in milliseconds that transmitted data may
                            //  remain unacknowledged, or bufferred data may remain
                            //  untransmitted (due to zero window size) before TCP will
                            //  forcibly close the corresponding connection and return
                            //  ETIMEDOUT to the application.  If the option value is
                            //  specified as 0, TCP will use the system default.
                            //
                            // macos为second
                            if cfg!(target_os = "linux") {
                                // unsafe {
                                //     let a = setsockopt(stream.as_raw_fd(), 0x06, 0x12,
                                //                        &tcp_user_timeout as *const u32 as *const c_void,  mem::size_of::<c_int>() as u32);
                                //     println!("lib set sockopt error: {}", a);
                                // }
                                // 两种方法都可以；


                                // //  这玩意只不过写了个 宏调用 libc，做了一些封装
                                #[cfg(target_family = "linux")]
                                match nix_setsockopt(stream.as_raw_fd(), TcpUserTimeout, &(tcp_user_timeout as u32)) {
                                    Ok(_) => {},
                                    Err(e) => {println!("nix_lib set sockopt error: {:?}", e)}, // ???? 遗留
                                }
                            } else if cfg!(target_os = "macos") {
                                unsafe {
                                    let a = setsockopt(stream.as_raw_fd(), 0x06, 0x80,
                                                       &tcp_user_timeout as *const u32 as *const c_void,  mem::size_of::<c_int>() as u32);
                                    println!("lib set sockopt error: {}", a);
                                }
                            } else {
                                println!("Unsupported platform!");
                                std::process::exit(1);
                            }



                            match send(stream.as_raw_fd(), format!("[{}]one hello", thread_index).as_bytes(), MsgFlags::empty()) {
                                Ok(size) => {
                                    if size == 0 {
                                        println!("send --> closed[FIN]")
                                    } else {
                                        println!("send --> peek: {}", size);
                                    }
                                }
                                Err(e) => {
                                    match e {
                                        errno::Errno::EAGAIN | errno::Errno::EWOULDBLOCK => {
                                            println!("operation would block, Try again, [EAGAIN]");
                                        }
                                        errno::Errno::ECONNRESET => {
                                            // tx.send(Message::FIN(WrapperMessage{addr: stream.peer_addr(), content: "reached EOF,".to_string() } ))
                                            //     .unwrap_or_default();
                                            println!("read closed. reached EOF, maybe[FIN]");
                                            // return;
                                        }
                                        others => { println!("[{}]: {:?} probe send other error: {}!", thread_index, probe_time, others); }
                                    }
                                }
                            }

                            println!("[{}]-----check after probe----", thread_index);

                            thread::sleep(Duration::from_secs(3));
                            // 如果检测时，tcp孩子重试，则此处的错误为：EAGAIN！！！所以一定要确保检测时，已经重试完毕。
                            // 从抓包情况来看，重试的完如果不同系统就直接发reset包，而程序结束时发[FIN]包，
                            // 至于先发reset还是[FIN]，如果正常通信的情况下，互相发完fin，就完了，不会发reset包。
                            // 非正常情况，程序的fin和系统的reset各发各的，互不影响。但是先发reset就不发fin了，反过来不成立
                            // 似乎linux不发送reset--
                            {
                                let mut peek_buf = [0u8; 1];
                                match recv(stream.as_raw_fd(), &mut peek_buf, MsgFlags::MSG_PEEK | MsgFlags::MSG_DONTWAIT) {
                                    Ok(size) => {
                                        // never run here
                                        if size == 0 {
                                            println!("check closed[FIN]")
                                        } else {
                                            println!("check peek:{}", size); // never run
                                        }
                                    }
                                    Err(e) => {

                                        match e {
                                            errno::Errno::EAGAIN | errno::Errno::EWOULDBLOCK => {
                                                probe_st.send((CheckError::EAGAIN, probe_time,
                                                               std::time::Instant::now().duration_since(conn_start_time))).unwrap();
                                            }
                                            errno::Errno::ECONNRESET => {
                                                probe_st.send((CheckError::RESET, probe_time,
                                                               std::time::Instant::now().duration_since(conn_start_time))).unwrap();
                                            }
                                            errno::Errno::ETIMEDOUT => {
                                                probe_st.send((CheckError::TimedOUT, probe_time,
                                                               std::time::Instant::now().duration_since(conn_start_time))).unwrap();
                                            }
                                            others => {
                                                println!("[{}]: {:?} probe check other error!", thread_index, probe_time);
                                                std::process::exit(0);}
                                        }
                                    }
                                }
                            }



                            thread_index -= 1;

                        },
                        Signal::Terminated => {
                            probe_stream.clear(); // close all stream; # todo re probe;
                        }
                    }

                },
                Err(_) => {panic!("recv closed...")}
            }
        }
    });
}
