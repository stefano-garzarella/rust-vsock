use nix::sys::socket;
use std::time::{Duration, Instant};
use vsock::{Vsock, VsockCid};
use clap::{Arg, ArgGroup, App, value_t, crate_authors, crate_version};

extern crate bincode;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate bytefmt;

const IPERF_DEFAULT_PORT: u32 = 5201;

enum IperfMsgOpCode {
    Start = 1,
    Data,
    End,
}

#[derive(Serialize, Deserialize)]
struct IperfMsg {
    opcode: u64,
    len: u64,
}

enum IperfMode {
    None,
    Server,
    Client,
}

struct IperfState {
    mode: IperfMode,
    time: u64,
    buf_len: usize,
    bytes: u64,
    start_time: Instant,
    end_time: Instant,
}

impl Default for IperfState {
    fn default () -> IperfState {
        IperfState {
            mode: IperfMode::None,
            time: 0,
            buf_len: 0,
            bytes: 0,
            start_time: Instant::now(),
            end_time: Instant::now(),
        }
    }
}

fn iperf_sender(istate: &mut IperfState, vsock: &Vsock) {
    let buf: Vec<u8> = vec![42; istate.buf_len];
    let mut msg = IperfMsg {opcode: 0, len: 0};

    msg.len = istate.buf_len as u64;
    msg.opcode = IperfMsgOpCode::Start as u64;
    let msg_serialized = bincode::serialize(&msg).unwrap();
    vsock.send(&msg_serialized, socket::MsgFlags::empty()).unwrap();

    msg.opcode = IperfMsgOpCode::Data as u64;
    let msg_serialized = bincode::serialize(&msg).unwrap();

    loop {
        let mut sent;

        let elapsed = istate.start_time.elapsed().as_secs();

        if elapsed >= istate.time {
            break;
        }

        sent = vsock.send(&msg_serialized, socket::MsgFlags::empty()).unwrap();
        sent += vsock.send(&buf[0 .. (istate.buf_len - sent)],
                           socket::MsgFlags::empty()).unwrap();

        istate.bytes += sent as u64;
    }

    msg.opcode = IperfMsgOpCode::End as u64;
    msg.len = 0;
    let msg_serialized = bincode::serialize(&msg).unwrap();

    vsock.send(&msg_serialized, socket::MsgFlags::empty()).unwrap();
}

fn iperf_receiver(istate: &mut IperfState, vsock: &Vsock) {
    let mut msg = IperfMsg {opcode: 0, len: 0};
    let mut msg_serialized: Vec<u8> = bincode::serialize(&msg).unwrap();

    vsock.recv(&mut msg_serialized, socket::MsgFlags::empty()).unwrap();
    msg = bincode::deserialize(&msg_serialized[..]).unwrap();

    assert_eq!(msg.opcode, IperfMsgOpCode::Start as u64);

    let buf_len = msg.len as usize;
    let mut buf: Vec<u8> = vec![0; buf_len];

    loop {
        let mut received;

        received = vsock.recv(&mut msg_serialized, socket::MsgFlags::empty()).unwrap();
        msg = bincode::deserialize(&msg_serialized[..]).unwrap();

        if msg.opcode == IperfMsgOpCode::End as u64 {
            break;
        }

        assert_eq!(msg.opcode, IperfMsgOpCode::Data as u64);

        while received < buf_len {
            received += vsock.recv(&mut buf [0 .. buf_len - received],
                                   socket::MsgFlags::MSG_WAITALL).unwrap();
        }

        istate.bytes += received as u64;
    }
}

fn iperf_stats(bytes: u64, time: &Duration) {
    let duration = time.as_millis() as f64 / 1000.0;
    let transfer = bytefmt::format(bytes);
    let bitrate = bytefmt::format(((bytes as f64) * 8.0 / duration) as u64);

    println!("Duration\tTransfer\tBitrate");
    println!("{0} sec \t{1}ytes \t{2}its/sec", duration, transfer, bitrate);
}

fn iperf_vsock(istate: &mut IperfState, vsock: &Vsock) {

    istate.start_time = Instant::now();
    match istate.mode {
        IperfMode::Server => {
            iperf_receiver(istate, vsock);
        }
        IperfMode::Client => {
            iperf_sender(istate, vsock);
        }
        _ => {
            panic!("Unknown mode!");
        }
    }
    istate.end_time = Instant::now();

    iperf_stats(istate.bytes, &istate.end_time.duration_since(istate.start_time));
}

fn main() {
    let cmd_args = App::new("iperf-vsock")
        .version(crate_version!())
        .author(crate_authors!())
        .about("VSOCK Rust demo app -  iperf like tool")
        .group(ArgGroup::with_name("mode")
                .required(true))
        .arg(
            Arg::with_name("server")
                .long("server")
                .short("s")
                .group("mode")
                .help("run in server mode"),
        )
        .arg(
            Arg::with_name("client")
                .long("client")
                .short("c")
                .takes_value(true)
                .group("mode")
                .help("run in client mode, connecting to <client>"),
        )
        .arg(
            Arg::with_name("port")
                .long("port")
                .short("p")
                .takes_value(true)
                .help("server port to listen on/connect to"),
        )
        .arg(
            Arg::with_name("time")
                .long("time")
                .short("t")
                .takes_value(true)
                .requires("client")
                .help("time in seconds to transmit for (default 10 secs)"),
        )
        .arg(
            Arg::with_name("length")
                .long("length")
                .short("l")
                .takes_value(true)
                .requires("client")
                .help("length [KiB|MiB] of buffer to read or write"),
        )
        .get_matches();

    let vsock = Vsock::new();
    let mut istate = IperfState::default();

    let port = value_t!(cmd_args.value_of("port"), u32)
                .unwrap_or(IPERF_DEFAULT_PORT);

    if cmd_args.is_present("server") {
        istate.mode = IperfMode::Server;

        vsock.bind(VsockCid::any(), port).unwrap();
        vsock.listen(1).expect("Unable to listen");

        loop {
            println!("-----------------------------------------------------------");
            println!("Server listening on port {}", port);
            println!("-----------------------------------------------------------");

            let client_vsock = vsock.accept().unwrap();

            let (client_cid, client_port) = client_vsock.getpeername().unwrap();
            println!("Accepted connection from {0}, port {1}", client_cid,
                     client_port);
            iperf_vsock(&mut istate, &client_vsock);

            istate = IperfState::default();
            istate.mode = IperfMode::Server;
        }
    } else {
        istate.mode = IperfMode::Client;

        let cid = value_t!(cmd_args.value_of("client"), u32).unwrap();
        let length = cmd_args.value_of("length").unwrap_or("128KiB");
        istate.time = value_t!(cmd_args.value_of("time"), u64).unwrap_or(10);
        istate.buf_len = bytefmt::parse(length).unwrap() as usize;

        vsock.connect(cid, port).expect("Unable to connect");

        println!("Connecting to host {0}, port {1}", cid, port);
        iperf_vsock(&mut istate, &vsock);
    }

}
