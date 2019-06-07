use nix::errno::Errno;
use nix::Result;
use nix::sys::socket::{AddressFamily, SockType, SockFlag, MsgFlags};
use nix::sys::socket::{socket, listen, accept, send, recv};
use std::mem;
use std::os::unix::io::RawFd;

unsafe fn sockaddr_vm(cid: u32, port: u32) -> (libc::sockaddr_vm, libc::socklen_t) {
    let mut addr: libc::sockaddr_vm = mem::zeroed();
    addr.svm_family = libc::AF_VSOCK as libc::sa_family_t;

    addr.svm_port = port;
    addr.svm_cid = cid;

    (addr, mem::size_of::<libc::sockaddr_vm>() as libc::socklen_t)
}

pub struct VsockCid {}

impl VsockCid {

    pub fn any() -> u32 {
        libc::VMADDR_CID_ANY
    }

    pub fn hypervisor() -> i32 {
        0
    }

    pub fn host() -> i32 {
        0
    }
}

pub struct Vsock {
    fd: RawFd
}

impl Vsock {
    pub fn new() -> Self {
        let socket_fd = socket(AddressFamily::Vsock, SockType::Stream,
                               SockFlag::empty(), None).unwrap();
        Vsock { fd: socket_fd, }
    }

    pub fn raw_fd(&self) -> RawFd {
        self.fd
    }

    pub fn connect(&self, cid: u32, port: u32) -> Result<()> {

        let res = unsafe {
            let (addr, len) = sockaddr_vm(cid, port);
            libc::connect(self.fd, mem::transmute(&addr), len)
        };

        return Errno::result(res).map(drop);
    }

    pub fn accept(&self) -> Result<Vsock> {
        let client_fd = accept(self.fd)?;

        Ok(Vsock {fd: client_fd})
    }

    pub fn bind(&self, cid: u32, port: u32) -> Result<()> {
        let res = unsafe {
            let (addr, len) = sockaddr_vm(cid, port);
            libc::bind(self.fd, mem::transmute(&addr), len)
        };

        return Errno::result(res).map(drop);
    }

    pub fn getsockname(&self) -> Result<(u32, u32)> {
        let addr: libc::sockaddr_vm;

        let res = unsafe {
            addr =  mem::zeroed();
            let mut addrlen: libc::socklen_t = mem::size_of::<libc::sockaddr_vm>()
                                               as libc::socklen_t;
            libc::getsockname(self.fd, mem::transmute(&addr), &mut addrlen)
        };

        Errno::result(res)?;

        return Ok((addr.svm_cid, addr.svm_port));
    }

    pub fn getpeername(&self) -> Result<(u32, u32)> {
        let addr: libc::sockaddr_vm;

        let res = unsafe {
            addr =  mem::zeroed();
            let mut addrlen: libc::socklen_t = mem::size_of::<libc::sockaddr_vm>()
                                               as libc::socklen_t;
            libc::getpeername(self.fd, mem::transmute(&addr), &mut addrlen)
        };

        Errno::result(res)?;

        return Ok((addr.svm_cid, addr.svm_port));
    }

    pub fn listen(&self, backlog: usize) -> Result<()> {
        listen(self.fd, backlog)
    }

    pub fn recv(&self, buf: &mut [u8], flags: MsgFlags) -> Result<usize> {
        recv(self.fd, buf, flags)
    }

    pub fn send(&self, buf: &[u8], flags: MsgFlags) -> Result<usize> {
        send(self.fd, buf, flags)
    }
}

impl Drop for Vsock {

    fn drop(&mut self) {
        let _ = nix::unistd::close(self.fd);
    }

}
