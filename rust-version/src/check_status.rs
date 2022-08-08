use std::net::SocketAddr;
use std::time::Duration;
use std::fmt::{Display, Formatter};

// use std::fmt;
#[derive(Default)]
pub enum CheckError{
    #[default]
    EAGAIN,
    FIN,
    RESET,
    Readed,
    ReadWriteError(std::io::Error),
    TimedOUT,
    OtherErrno(nix::errno::Errno),
}

impl Display for CheckError {
    // fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckError::EAGAIN=> write!(f, "EAGAIN"),
            CheckError::FIN=> write!(f, "FIN"),
            CheckError::RESET=> write!(f, "RESET"),
            CheckError::Readed=> write!(f, "Readed"),
            CheckError::ReadWriteError(e)=> {
                write!(f, "ReadWriteError: {:?}", e.kind())
            },
            CheckError::TimedOUT => write!(f, "TimedOUT"),
            CheckError::OtherErrno(e )=> write!(f, "OtherErrno: {}", e),
        }
    }
}

// #[derive(Default)]
pub struct  WrapperMessage {
    pub addr: std::io::Result<SocketAddr>,
    pub content: String,
    pub thread_index: Option<usize>,
    pub probe_time: Option<Duration>,
    pub check_error: CheckError,
}

// SomeOptions { foo: 42, ..Default::default() }


// pub enum Message {
//     FIN(WrapperMessage),
//     RESET(WrapperMessage),
//     Read1thError(WrapperMessage),
//     Write1thError(WrapperMessage),
//     CheckError(WrapperMessage),
//     Probe(std::io::Result<SocketAddr>, errno::Errno, usize, Duration),
//
// }
//
// impl Message {
//     pub fn handling_result(self) {
//         let default_addr = SocketAddr::from_str("127.0.0.0:8001").unwrap();
//         match self {
//             Message::FIN(s) => {
//                 println!("[FIN] {}  {}", s.addr.unwrap_or(default_addr).to_string(), s.content)
//             },
//             Message::RESET(s) => {
//                 // s.addr 已经为空了
//                 println!("[RESET] {}  {}", "miss addr", s.content)
//             },
//             Message::Read1thError(s) | Message::Write1thError(s) => {
//                 println!("[FIN] {}  {} \nRead/Write error on init connection.",
//                          s.addr.unwrap_or(default_addr).to_string(), s.content)
//             },
//             Message::CheckError(s) => {
//                 println!("[check] {}  {}", s.addr.unwrap_or(default_addr).to_string(), s.content)
//             },
//             Message::Probe(addr, e, thread_index, probe_time) => {
//                 // 控制线程。。。？？？似乎不行了
//                 println!("addr: {}", addr.unwrap_or(default_addr).to_string());
//                 match e {
//                     errno::Errno::EAGAIN => {
//                         println!("[{}] {:?} \x1b[40;32mhas alive\x1b[0m [EAGAIN]", thread_index, probe_time)
//                     }
//                     errno::Errno::ECONNRESET => { // [R]
//                         println!("[{}]: {:?} connection \x1b[41;36mclosed [R]\x1b[0m some connection was killed!", thread_index, probe_time);
//
//                         std::process::exit(0);
//                     }
//                     errno::Errno::ETIMEDOUT => {
//                         println!("[{}]: {:?} \x1b[41;36m[TIMEOUT]\x1b[0m some connection was killed!", thread_index, probe_time);
//                         // todo recycle probe but now exit;
//                         std::process::exit(0);
//
//                     }
//                     others => { println!("[{}]: {:?} probe check other error: {}!", thread_index, probe_time, others); }
//                 }
//             }
//         }
//     }
// }